use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::path::{Path, PathBuf};

use anyhow::Result;
use async_stream::try_stream;
use chrono::{DateTime, Local, Utc};
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
use tokio_stream::{Stream, StreamExt};

use crate::account::AccLabel;
use crate::blobs;
use crate::client::Client;
use crate::registry::{
    Registry, WithAccountAtom, WithBlobsAtom, WithDb, WithDeviceAtom, WithInTxn, WithTimelineAtom,
};
use crate::timeline::card::{CardFile, CardView, ContentView};

pub trait ExportCtx<C: Clone>: WithDb + WithInTxn<C> + WithDeviceAtom + WithBlobsAtom<C> {}
impl<T, C: Clone> ExportCtx<C> for T where
    T: WithDb + WithInTxn<C> + WithDeviceAtom + WithBlobsAtom<C>
{
}

#[derive(Clone)]
pub struct ExportAtom {}

impl ExportAtom {
    pub fn new() -> Self {
        Self {}
    }

    /// Export single card
    pub async fn export_card<C: Client>(
        &self,
        ctx: &impl ExportCtx<C>,
        card_id: &str,
    ) -> Result<ExportedCard> {
        let (acc, card) = ctx.in_txn(|tx_ctx| {
            let acc = tx_ctx.account().require_account(tx_ctx)?;
            let card = tx_ctx.timeline().get_card(tx_ctx, card_id)?;
            Ok((acc, card))
        })?;

        // A mapping from label ID to label
        let acc_labels: HashMap<_, _> = acc.labels.iter().map(|l| (&l.id, l)).collect();
        let exported = Self::build_exported_card(ctx, card, &acc_labels).await?;
        Ok(exported)
    }

    async fn build_exported_card<C: Client>(
        ctx: &impl ExportCtx<C>,
        card: CardView,
        acc_labels: &HashMap<&String, &AccLabel>,
    ) -> Result<ExportedCard> {
        let exporter = CardExporter::new(card, acc_labels);
        let (card_export, attachments) = exporter.prepare(ctx).await?;
        MarkdownWriter::serialize_card(card_export, attachments)
    }

    /// Return an iterator over exported cards
    pub fn cards<'a, C: Client + 'a>(
        &self,
        registry: Registry<C>,
    ) -> Result<impl Stream<Item = Result<ExportedCard>> + 'a> {
        let acc = registry
            .db_ctx()
            .in_txn(|tx_ctx| tx_ctx.account().require_account(tx_ctx))?;

        Ok(try_stream! {
            let ctx = registry.db_ctx();
            // A mapping from label ID to label
            let acc_labels: HashMap<_, _> = acc.labels.iter().map(|l| (&l.id, l)).collect();
            let mut offset = 0;

            loop {
                let card = match ctx.in_txn(|tx_ctx| tx_ctx.timeline().find_first(tx_ctx, offset)) {
                    Ok(c) => c,
                    Err(err) => {
                        tracing::warn!("Failed to query card: {}", err);
                        break;
                    }
                };
                match card {
                    Some(card) => {
                        tracing::info!(card.id, offset, "Exporting");
                        let exported = Self::build_exported_card(&ctx, card, &acc_labels).await?;
                        yield exported;
                    },
                    None => {
                        break
                    },
                }
                offset += 1;
            }
        })
    }

    pub async fn cards_to_dir<C: Client>(
        &self,
        registry: Registry<C>,
        out_dir: PathBuf,
    ) -> Result<()> {
        let cards = self.cards(registry)?;
        tokio::pin!(cards);

        // Create export directories
        let export_dir = out_dir.join(format!(
            "Bolik Timeline export {}",
            Local::now().format("%Y-%m-%d")
        ));
        tracing::info!("Starting export to {}", export_dir.display());
        let files_dir = export_dir.join("Files");
        std::fs::create_dir_all(&files_dir)?;

        while let Some(exported) = cards.next().await {
            let exported = match exported {
                Ok(e) => e,
                Err(err) => {
                    tracing::warn!("Failed to prepare card for export: {}", err);
                    continue;
                }
            };

            if let Err(err) = Self::copy_exported(exported, &export_dir) {
                tracing::warn!("Failed to copy exported card: {}", err);
            }
        }
        Ok(())
    }

    fn copy_exported(exported: ExportedCard, export_dir: &Path) -> Result<()> {
        // Write markdown file
        let file_path = export_dir.join(&exported.file_name);
        let mut file = std::fs::File::create(&file_path)?;
        std::io::Write::write_all(&mut file, exported.content.as_bytes())?;

        // Copy all attachments to export dir
        for attachment in exported.files {
            let attachment_target = export_dir.join(&attachment.content_path);
            std::fs::copy(&attachment.path, &attachment_target)?;
        }

        Ok(())
    }
}

/// A single exported card
pub struct ExportedCard {
    pub id: String,
    pub created_at: DateTime<Utc>,
    /// A suggested file name
    pub file_name: String,
    /// Card serialized to markdown
    pub content: String,
    /// List of file paths
    pub files: Vec<ExportedAttachment>,
}

pub struct ExportedAttachment {
    /// Location on disk
    pub path: String,
    /// Relative path to the link in content.
    /// For `[hello.txt](./Files/hello 1bdc5.txt)` content_path will be `./Files/hello 1bdc5.txt`
    pub content_path: String,
}

struct CardExport {
    metadata: MetadataSection,
    content: Vec<ContentSection>,
}

struct MetadataSection {
    id: String,
    created_at: DateTime<Utc>,
    labels: Vec<String>,
}

#[derive(Debug)]
enum ContentSection {
    /// Lines of styles text
    Paragraph(Vec<Vec<TextSpan>>),
    List(ListSection),
    Files(FilesSection),
    Heading(String, u8),
}

#[derive(Debug)]
struct TextSpan {
    text: String,
    styles: HashSet<TextStyle>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum TextStyle {
    Bold,
    Italic,
    Strikethrough,
    Underline,
}

#[derive(Debug)]
struct ListSection {
    r#type: ListType,
    items: Vec<ListItem>,
}

#[derive(Debug, PartialEq)]
enum ListType {
    Ordered,
    Unordered,
    Checklist,
}

#[derive(Debug)]
struct ListItem {
    checked: bool,
    text: Vec<TextSpan>,
}

#[derive(Default, Debug)]
struct FilesSection {
    files: Vec<FileItem>,
}

#[derive(Debug)]
struct FileItem {
    name: String,
    path: String,
}

struct CardExporter<'a> {
    card: CardView,
    acc_labels: &'a HashMap<&'a String, &'a AccLabel>,

    sections: Vec<ContentSection>,
    /// Lines of current paragraph.
    paragraph: Vec<Vec<TextSpan>>,
}

impl<'a> CardExporter<'a> {
    pub fn new(card: CardView, acc_labels: &'a HashMap<&'a String, &'a AccLabel>) -> Self {
        Self {
            card,
            acc_labels,
            sections: vec![],
            paragraph: vec![],
        }
    }

    pub async fn prepare<C: Client>(
        mut self,
        ctx: &impl ExportCtx<C>,
    ) -> Result<(CardExport, Vec<ExportedAttachment>)> {
        let mut attachments = vec![];

        // Build a list of label names
        let mut labels = vec![];
        for card_label in &self.card.labels {
            if let Some(l) = self.acc_labels.get(&card_label.id) {
                labels.push(l.name.clone());
            }
        }
        labels.sort();

        fn last_line(lines: &mut Vec<String>) -> String {
            lines.pop().unwrap_or_default()
        }

        let blocks = std::mem::take(&mut self.card.blocks);
        for block in blocks {
            match block.view {
                ContentView::Text(mut t) => {
                    tracing::trace!("Processing text block {}", t.value);
                    let mut styles = HashSet::new();
                    if let Some(mut attrs) = t.attrs.take() {
                        match attrs.block {
                            // Checklist
                            Some(b) if b == "cl" => {
                                let checked = attrs.checked == Some(true);
                                self.push_list_item(ListType::Checklist, checked);
                                continue;
                            }
                            // Unordered list
                            Some(b) if b == "ul" => {
                                self.push_list_item(ListType::Unordered, false);
                                continue;
                            }
                            // Ordered list
                            Some(b) if b == "ol" => {
                                self.push_list_item(ListType::Ordered, false);
                                continue;
                            }
                            // TODO: code
                            // TODO: quote
                            _ => {}
                        }

                        // Heading
                        if let Some(level) = attrs.heading {
                            self.push_heading(level);
                            continue;
                        }

                        // Link
                        if let Some(link) = attrs.link.take() {
                            self.push_text(format!("<{link}>"), styles);
                            continue;
                        }

                        // Text styles
                        if let Some(true) = attrs.bold {
                            styles.insert(TextStyle::Bold);
                        }
                        if let Some(true) = attrs.italic {
                            styles.insert(TextStyle::Italic);
                        }
                        if let Some(true) = attrs.strikethrough {
                            styles.insert(TextStyle::Strikethrough);
                        }
                        if let Some(true) = attrs.underline {
                            styles.insert(TextStyle::Underline);
                        }
                    }

                    self.push_text(t.value, styles);
                }
                ContentView::File(f) => {
                    // Use local path or download the blob from remote
                    let blob_path = self.download_blob(ctx, &f).await?;

                    let disk_name = if let Some(name) = Path::new(&blob_path).file_name() {
                        name.to_string_lossy().to_string()
                    } else {
                        f.blob_id.clone()
                    };
                    let encoded_name =
                        percent_encode(disk_name.as_bytes(), NON_ALPHANUMERIC).to_string();
                    let human_name = f.name.as_ref().unwrap_or(&f.blob_id);

                    let encoded_href = Path::new(".").join("Files").join(&encoded_name);
                    let href = Path::new(".").join("Files").join(&disk_name);

                    let file_item = FileItem {
                        name: human_name.clone(),
                        path: encoded_href.to_string_lossy().to_string(),
                    };

                    self.push_file(file_item);
                    attachments.push(ExportedAttachment {
                        path: blob_path,
                        content_path: href.to_string_lossy().to_string(),
                    });
                }
            }
        }

        // Push last paragraph
        self.wrap_paragraph();

        Ok((
            CardExport {
                metadata: MetadataSection {
                    id: self.card.id,
                    created_at: self.card.created_at,
                    labels,
                },
                content: self.sections,
            },
            attachments,
        ))
    }

    fn wrap_paragraph(&mut self) {
        let mut p = std::mem::take(&mut self.paragraph);
        if let Some(last) = p.pop() {
            if last.is_empty() || (last.len() == 1 && last[0].text.is_empty()) {
                // Last line is empty
            } else {
                // Last line is not empty --> add back
                p.push(last);
            }
        }

        if !p.is_empty() {
            self.sections.push(ContentSection::Paragraph(p));
        }
    }

    fn push_text(&mut self, text: String, styles: HashSet<TextStyle>) {
        // Insert multiple lines
        let segment_lines = text.split('\n');
        for (i, line) in segment_lines.enumerate() {
            let span = TextSpan {
                text: line.into(),
                styles: styles.clone(),
            };
            match self.paragraph.last_mut() {
                Some(last) if i == 0 => {
                    // Try to append first segment line to the last paragraph line.
                    last.push(span);
                    continue;
                }
                _ => {}
            }

            self.paragraph.push(vec![span]);
        }
    }

    fn push_heading(&mut self, level: u8) {
        let last_line = self.paragraph.pop();
        self.wrap_paragraph();

        let Some(last_line) = last_line else {
            // Nothing to push
            return;
        };

        // Ignore styles for headings
        let text = last_line
            .into_iter()
            .map(|s| s.text)
            .collect::<Vec<_>>()
            .join("");
        self.sections.push(ContentSection::Heading(text, level));
    }

    fn push_list_item(&mut self, r#type: ListType, checked: bool) {
        let last_line = self.paragraph.pop();
        self.wrap_paragraph();

        let Some(last_line) = last_line else {
            // Nothing to push
            return;
        };

        let item = ListItem {
            checked,
            text: last_line,
        };

        match self.sections.last_mut() {
            Some(ContentSection::List(current)) if current.r#type == r#type => {
                // Merge lists of the same type
                current.items.push(item);
                return;
            }
            _ => {}
        }

        self.sections.push(ContentSection::List(ListSection {
            r#type,
            items: vec![item],
        }));
    }

    fn push_file(&mut self, item: FileItem) {
        self.wrap_paragraph();

        match self.sections.last_mut() {
            Some(ContentSection::Files(current)) => {
                // Merge
                current.files.push(item);
                return;
            }
            _ => {}
        }

        self.sections
            .push(ContentSection::Files(FilesSection { files: vec![item] }));
    }

    async fn download_blob<C: Client>(
        &self,
        ctx: &impl ExportCtx<C>,
        f: &CardFile,
    ) -> Result<String> {
        let local_blob = {
            let conn = ctx.db().conn.lock().unwrap();
            blobs::find_by_id(&conn, &f.blob_id, &f.device_id)?
        };
        let local_path = if let Some(b) = local_blob {
            let p = PathBuf::from(&b.path);
            if p.exists() {
                Some(b.path)
            } else {
                None
            }
        } else {
            None
        };

        match local_path {
            Some(p) => {
                tracing::debug!(f.blob_id, f.name, "File found locally");
                Ok(p)
            }
            None => {
                tracing::debug!(f.blob_id, f.name, "Downloading file from remote");
                let path = ctx.blobs().download(ctx, &self.card, &f).await?;
                Ok(path)
            }
        }
    }
}

struct MarkdownWriter {}

impl MarkdownWriter {
    fn serialize_card(
        card: CardExport,
        attachments: Vec<ExportedAttachment>,
    ) -> Result<ExportedCard> {
        let mut output = String::new();

        // Write metadata
        let MetadataSection {
            id,
            created_at,
            labels,
        } = &card.metadata;

        output.push_str("# Bolik card v2\n\n");
        output.write_fmt(format_args!("* ID: {}\n", id))?;
        output.write_fmt(format_args!("* Created at: {}\n", created_at.to_rfc3339()))?;
        output.write_fmt(format_args!("* Labels: {}\n", labels.join(", ")))?;

        output.push_str("\n-------------------------------\n\n");

        // Write content sections
        for section in card.content {
            tracing::trace!("Writing section: {:?}", section);
            match section {
                ContentSection::Paragraph(lines) => {
                    for spans in lines {
                        write_spans(&mut output, spans);
                        output.push_str("\n");
                    }
                }
                ContentSection::List(list) => {
                    for item in list.items {
                        let prefix = if item.checked {
                            "* [x]"
                        } else {
                            match list.r#type {
                                ListType::Ordered => "1.",
                                ListType::Unordered => "*",
                                ListType::Checklist => "* [ ]",
                            }
                        };
                        output.push_str(prefix);
                        output.push_str(" ");
                        write_spans(&mut output, item.text);
                        output.push_str("\n");
                    }
                }
                ContentSection::Files(files) => {
                    for file in files.files {
                        output
                            .write_fmt(format_args!("* [File:{}]({})\n", file.name, file.path))?;
                    }
                }
                ContentSection::Heading(text, level) => {
                    let prefix = "#".repeat(level as usize);
                    output.write_fmt(format_args!("{prefix} {text}\n"))?;
                }
            }

            output.push_str("\n");
        }

        let file_name = format!(
            "{} ({}).md",
            card.metadata.created_at.format("%Y-%m-%dT%H:%M:%S"),
            card.metadata.id.chars().take(6).collect::<String>()
        );
        Ok(ExportedCard {
            id: card.metadata.id,
            created_at: card.metadata.created_at,
            file_name,
            content: output,
            files: attachments,
        })
    }
}

fn write_spans(output: &mut String, spans: Vec<TextSpan>) {
    for span in spans {
        if span.styles.contains(&TextStyle::Underline) {
            output.push_str("<ins>");
        }
        if span.styles.contains(&TextStyle::Bold) {
            output.push_str("**");
        }
        if span.styles.contains(&TextStyle::Italic) {
            output.push_str("_");
        }
        if span.styles.contains(&TextStyle::Strikethrough) {
            output.push_str("~~");
        }

        output.push_str(&span.text);

        if span.styles.contains(&TextStyle::Strikethrough) {
            output.push_str("~~");
        }
        if span.styles.contains(&TextStyle::Italic) {
            output.push_str("_");
        }
        if span.styles.contains(&TextStyle::Bold) {
            output.push_str("**");
        }
        if span.styles.contains(&TextStyle::Underline) {
            output.push_str("</ins>");
        }
    }
}

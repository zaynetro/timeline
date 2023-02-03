use std::{cmp::min, collections::HashMap, path::Path};

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use percent_encoding::percent_decode_str;
use pulldown_cmark::{Event, Parser, Tag};

use crate::{
    account::{AccLabel, AccView},
    blobs::{self, SaveFileParams},
    timeline::{
        card::{CardChange, CardLabelsChange, CardText, CardTextAttrs, CardView, ContentView},
        EditCardOpts,
    },
};

use super::ImportCtx;

pub fn import_card<'a>(
    ctx: &impl ImportCtx<'a>,
    in_dir: &Path,
    acc: &mut AccView,
    content: &str,
) -> Result<ImportCardResult> {
    let mut parser = {
        let mut opts = pulldown_cmark::Options::empty();
        opts.insert(pulldown_cmark::Options::ENABLE_TASKLISTS);
        opts.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);
        Parser::new_ext(content, opts)
    };

    let init = InitReader::default();
    let meta = init.read(&mut parser)?;
    let content = meta.read(&mut parser)?;

    if ctx.docs().find(ctx, &content.meta.id)?.is_some() {
        // This card exists already
        return Ok(ImportCardResult::Duplicate);
    }

    let card = content.read(&mut parser)?;
    let card_id = &card.meta.id;
    let mut changes = vec![];
    let mut label_changes = vec![];

    for label in &card.meta.labels {
        // Find existing acc label from the account.
        let acc_labels = get_acc_labels(acc);
        let label_id = match acc_labels.get(label) {
            Some(acc_label) => acc_label.id.clone(),
            None => {
                // If not found create a new label.
                let acc_label = AccLabel::new(label.to_string());
                let label_id = acc_label.id.clone();
                // Update account immeditately, so that when we index the card
                // we include newest label from the account.
                *acc = ctx.account().edit_account(ctx, |yrs_doc| {
                    tracing::debug!("Adding {} label to account", acc_label.name);
                    AccView::create_label(yrs_doc, acc_label);
                    Ok(())
                })?;

                label_id
            }
        };

        label_changes.push(CardLabelsChange::AddLabel { label_id });
    }

    for content in card.content.into_iter() {
        tracing::trace!("Read content: {:?}", content);
        match content {
            ReadContent::Paragraph(spans) => {
                for span in spans {
                    changes.push(CardChange::append(ContentView::Text(span)));
                }
                changes.push(CardChange::append_text("\n"));
            }
            ReadContent::Heading(text, level) => {
                changes.extend(CardChange::append_text_heading(text, level));
            }
            ReadContent::List(tasks) => {
                for task in tasks.items {
                    for span in task.spans {
                        changes.push(CardChange::append(ContentView::Text(span)));
                    }
                    changes.push(CardChange::append(ContentView::Text(CardText {
                        value: "\n".into(),
                        attrs: Some(CardTextAttrs {
                            block: Some(tasks.r#type.block_name().into()),
                            checked: if task.checked { Some(true) } else { None },
                            ..Default::default()
                        }),
                    })));
                }
                changes.push(CardChange::append_text("\n"));
            }
            ReadContent::Files(files) => {
                for ReadFile { name, path } in files {
                    let full_path = in_dir.join(&path);
                    if !full_path.exists() {
                        return Err(anyhow!("File is missing: {}", full_path.display()));
                    }

                    let card_file = blobs::save_file(
                        ctx.txn(),
                        SaveFileParams {
                            blob_dir: &ctx.device().blobs_dir,
                            path: &full_path,
                            original_file_name: Some(name),
                            device_id: ctx.device().id.clone(),
                            card_id: &card_id,
                        },
                    )?;
                    changes.push(CardChange::append(ContentView::File(card_file)));
                }

                changes.push(CardChange::append_text("\n"));
            }
        }
    }

    // Save card in documents table
    let card_doc = ctx
        .timeline()
        .edit_card_opts(
            ctx,
            EditCardOpts {
                id: card_id,
                changes,
                acl_changes: vec![],
                created_at: Some(card.meta.created_at),
                skip_counter: false,
            },
        )?
        .0;

    if !label_changes.is_empty() {
        ctx.timeline()
            .edit_card_labels(ctx, card_id, label_changes)?;
    }

    ctx.timeline().generate_thumbnail(ctx, &card_doc)?;
    Ok(ImportCardResult::Imported(card_doc))
}

#[derive(Default)]
struct CardBuilder {
    id: Option<String>,
    created_at: Option<DateTime<Utc>>,
    labels: Vec<String>,
}

#[derive(Default)]
struct InitReader {}

impl InitReader {
    fn read<'a>(self, events: &mut impl Iterator<Item = Event<'a>>) -> Result<MetaReader> {
        let mut in_heading = false;
        for event in events {
            match event {
                Event::Start(Tag::Heading(_, _, _)) => {
                    in_heading = true;
                }
                Event::End(Tag::Heading(_, _, _)) => {
                    in_heading = false;
                }
                Event::Text(t) if t.starts_with("Bolik card") && in_heading => {
                    return Ok(MetaReader::default());
                }
                _ => {}
            }
        }

        Err(anyhow!("Expected a line: '# Bolik card'"))
    }
}

#[derive(Default)]
struct MetaReader {
    card: CardBuilder,
}

impl MetaReader {
    fn read<'a>(
        mut self,
        events: &mut impl Iterator<Item = Event<'a>>,
    ) -> Result<CardContentReader> {
        for event in events {
            match event {
                Event::Text(t) => {
                    if let Some(id) = t.strip_prefix("ID: ") {
                        self.card.id = Some(id.to_string());
                    } else if let Some(created_at_str) = t.strip_prefix("Created at: ") {
                        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                            .context("Parse created at")?;
                        self.card.created_at = Some(created_at.into());
                    } else if let Some(labels_str) = t.strip_prefix("Labels: ") {
                        self.card.labels = labels_str
                            .split(", ")
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                }
                Event::Rule => match (self.card.id, self.card.created_at) {
                    (Some(id), Some(created_at)) => {
                        return Ok(CardContentReader::new(CardMeta {
                            id,
                            created_at,
                            labels: self.card.labels,
                        }));
                    }
                    (None, _) => return Err(anyhow!("Expected a line: '* ID:'")),
                    (_, None) => return Err(anyhow!("Expected a line: '* Created at:'")),
                },
                _ => {}
            }
        }

        Err(anyhow!("Expected a rule: '------'"))
    }
}

struct CardMeta {
    id: String,
    created_at: DateTime<Utc>,
    labels: Vec<String>,
}

struct ReadCard {
    meta: CardMeta,
    content: Vec<ReadContent>,
}

struct CardContentReader {
    meta: CardMeta,
    content: Vec<ReadContent>,
}

impl CardContentReader {
    fn new(meta: CardMeta) -> Self {
        Self {
            meta,
            content: vec![],
        }
    }

    fn read<'a>(mut self, events: &mut impl Iterator<Item = Event<'a>>) -> Result<ReadCard> {
        while let Some(event) = events.next() {
            match event {
                Event::Start(Tag::Paragraph) => {
                    self.read_paragraph(events);
                }
                Event::Start(Tag::List(num)) => {
                    self.read_list(events, num);
                }
                Event::End(Tag::List(_)) => {}
                Event::Start(Tag::Heading(_, _, _)) => {
                    self.read_heading(events);
                }
                _ => {}
            }
        }

        Ok(ReadCard {
            meta: self.meta,
            content: self.content,
        })
    }

    fn read_paragraph<'a>(&mut self, events: &mut impl Iterator<Item = Event<'a>>) {
        let mut span_reader = SpanReader::default();
        for event in events {
            match event {
                // Finish
                Event::End(Tag::Paragraph) => {
                    if !span_reader.spans.is_empty() {
                        self.content.push(ReadContent::Paragraph(span_reader.spans));
                    }
                    return;
                }
                _ => {
                    span_reader.read_single(event);
                }
            }
        }
    }

    fn read_list<'a>(&mut self, events: &mut impl Iterator<Item = Event<'a>>, num: Option<u64>) {
        let mut r#type = if num.is_some() {
            ListType::Ordered
        } else {
            ListType::Unordered
        };
        let mut items = vec![];
        let mut checked = false;
        let mut span_reader = SpanReader::default();

        let mut files = vec![];

        for event in events {
            match event {
                Event::Start(Tag::Item) => {
                    checked = false;
                }
                Event::End(Tag::Item) => {
                    if let Some(file) = span_reader.file {
                        files.push(file);
                    } else {
                        items.push(ListItem {
                            checked,
                            spans: span_reader.spans,
                        });
                    }
                    span_reader = SpanReader::default();
                }
                Event::TaskListMarker(check_marker) => {
                    // Override list type
                    r#type = ListType::Checklist;
                    checked = check_marker;
                }
                Event::End(Tag::List(_)) => {
                    if files.is_empty() {
                        self.content
                            .push(ReadContent::List(ReadList { r#type, items }));
                    } else {
                        self.content.push(ReadContent::Files(files));
                    }
                    return;
                }
                _ => {
                    span_reader.read_single(event);
                }
            }
        }
    }

    fn read_heading<'a>(&mut self, events: &mut impl Iterator<Item = Event<'a>>) {
        let mut text = String::new();
        for event in events {
            match event {
                Event::Text(t) => {
                    text.push_str(&t);
                }
                Event::End(Tag::Heading(level, _, _)) => {
                    let l = min(level as usize, 3);
                    self.content.push(ReadContent::Heading(text, l as u8));
                    return;
                }
                _ => {}
            }
        }
    }
}

#[derive(Default)]
struct SpanReader {
    spans: Vec<CardText>,
    attrs: CardTextAttrs,
    link: Option<String>,
    file: Option<ReadFile>,
}

impl SpanReader {
    fn read_single<'a>(&mut self, event: Event<'a>) {
        match event {
            // Text
            Event::Text(t) => {
                if let Some(href) = &self.link {
                    // Check if we are reading a file
                    if let Some(name) = t.strip_prefix("File:") {
                        // Save file
                        match percent_decode_str(href).decode_utf8() {
                            Ok(path) => {
                                self.file = Some(ReadFile {
                                    name: name.into(),
                                    path: path.into(),
                                });
                            }
                            Err(err) => {
                                tracing::warn!("Failed to decode file path={}: {}", href, err);
                            }
                        }

                        self.link = None;
                    }
                    return;
                }

                self.spans
                    .push(CardText::new(&*t, Some(self.attrs.clone())));
            }
            Event::SoftBreak => {
                self.attrs = CardTextAttrs::default();
                self.spans.push(CardText::new("\n", None));
            }

            // Styles
            Event::Start(Tag::Strong) => {
                self.attrs.bold = Some(true);
            }
            Event::End(Tag::Strong) => {
                self.attrs.bold = None;
            }
            Event::Start(Tag::Emphasis) => {
                self.attrs.italic = Some(true);
            }
            Event::End(Tag::Emphasis) => {
                self.attrs.italic = None;
            }
            Event::Start(Tag::Strikethrough) => {
                self.attrs.strikethrough = Some(true);
            }
            Event::End(Tag::Strikethrough) => {
                self.attrs.strikethrough = None;
            }
            Event::Html(h) if &*h == "<ins>" => {
                self.attrs.underline = Some(true);
            }
            Event::Html(h) if &*h == "</ins>" => {
                self.attrs.underline = None;
            }
            Event::Start(Tag::Link(_, href, _)) => {
                self.link = Some(href.to_string());
            }
            Event::End(Tag::Link(_, _, _)) => {
                if let Some(href) = self.link.take() {
                    self.attrs.link = Some(href.clone());
                    self.spans
                        .push(CardText::new(href, Some(self.attrs.clone())));
                    self.attrs.link = None;
                    self.link = None;
                }
            }
            _ => {}
        }
    }
}

#[derive(Debug)]
enum ReadContent {
    Paragraph(Vec<CardText>),
    Heading(String, u8),
    List(ReadList),
    Files(Vec<ReadFile>),
}

#[derive(Debug)]
struct ReadList {
    r#type: ListType,
    items: Vec<ListItem>,
}

#[derive(Debug, PartialEq)]
enum ListType {
    Ordered,
    Unordered,
    Checklist,
}

impl ListType {
    fn block_name(&self) -> &str {
        match self {
            Self::Ordered => "ol",
            Self::Unordered => "ul",
            Self::Checklist => "cl",
        }
    }
}

#[derive(Debug)]
struct ListItem {
    checked: bool,
    spans: Vec<CardText>,
}

#[derive(Debug)]
struct ReadFile {
    name: String,
    path: String,
}

/// Get a mapping from label name to label
fn get_acc_labels(acc: &AccView) -> HashMap<&String, &AccLabel> {
    acc.labels.iter().map(|l| (&l.name, l)).collect()
}

pub enum ImportCardResult {
    Imported(CardView),
    Duplicate,
}

use std::{collections::HashMap, fmt::Write, path::Path};

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use percent_encoding::percent_decode_str;

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
    let mut lines = content.lines();
    let init = InitReader::default();
    let meta = init.read(&mut lines)?;
    let content = meta.read(&mut lines)?;
    let card = content.read(&mut lines)?;
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

    let mut was_file = false;
    for content in card.content.into_iter() {
        match content {
            ReadContent::Text(t) => {
                if was_file {
                    changes.push(CardChange::append_text("\n"));
                }
                changes.push(CardChange::append_text(t));
                changes.push(CardChange::append_text("\n"));
                was_file = false;
            }
            ReadContent::Tasks(tasks) => {
                changes.push(CardChange::append_text("\n"));
                for task in tasks {
                    changes.push(CardChange::append_text(task.text));
                    changes.push(CardChange::append(ContentView::Text(CardText {
                        value: "\n".into(),
                        attrs: Some(CardTextAttrs {
                            block: Some("cl".into()),
                            checked: if task.completed { Some(true) } else { None },
                            ..Default::default()
                        }),
                    })));
                }
                changes.push(CardChange::append_text("\n"));
                was_file = false;
            }
            ReadContent::File { path, name, .. } => {
                let full_path = in_dir.join(&path);
                if !full_path.exists() {
                    return Err(anyhow!("File is missing: {}", full_path.display()));
                }

                let card_file = blobs::save_file(
                    ctx.txn(),
                    SaveFileParams {
                        blob_dir: &ctx.device().blobs_dir,
                        path: &full_path,
                        original_file_name: name,
                        device_id: ctx.device().id.clone(),
                        card_id: &card_id,
                    },
                )?;
                changes.push(CardChange::append(ContentView::File(card_file)));
                was_file = true;
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
    Ok(ImportCardResult { card: card_doc })
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
    fn read<'a>(self, lines: impl Iterator<Item = &'a str>) -> Result<MetaReader> {
        for line in lines {
            if line.starts_with("# Bolik card") {
                return Ok(MetaReader::default());
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
    fn read<'a>(mut self, lines: impl Iterator<Item = &'a str>) -> Result<CardContentReader> {
        for line in lines {
            if let Some(id) = line.strip_prefix("* ID: ") {
                self.card.id = Some(id.to_string());
            } else if let Some(created_at_str) = line.strip_prefix("* Created at: ") {
                let created_at =
                    DateTime::parse_from_rfc3339(&created_at_str).context("Parse created at")?;
                self.card.created_at = Some(created_at.into());
            } else if let Some(labels_str) = line.strip_prefix("* Labels: ") {
                self.card.labels = labels_str
                    .split(", ")
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            } else if line.starts_with("## Content") {
                match (self.card.id, self.card.created_at) {
                    (Some(id), Some(created_at)) => {
                        return Ok(CardContentReader::new(CardMeta {
                            id,
                            created_at,
                            labels: self.card.labels,
                        }));
                    }
                    (None, _) => return Err(anyhow!("Expected a line: '* ID:'")),
                    (_, None) => return Err(anyhow!("Expected a line: '* Created at:'")),
                }
            }
        }

        Err(anyhow!("Expected a line: '## Content'"))
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
    current: ContentReaderType,
}

impl CardContentReader {
    fn new(meta: CardMeta) -> Self {
        Self {
            meta,
            content: vec![],
            current: ContentReaderType::None,
        }
    }

    fn read<'a>(mut self, lines: impl Iterator<Item = &'a str>) -> Result<ReadCard> {
        for line in lines {
            if line.starts_with("### Text") {
                self.end_current_with(ContentReaderType::Text(TextContentReader::default()))?;
            } else if line.starts_with("### Tasks") {
                self.end_current_with(ContentReaderType::Tasks(TasksContentReader::default()))?;
            } else if line.starts_with("### File") {
                self.end_current_with(ContentReaderType::File(FileContentReader::default()))?;
            } else {
                match &mut self.current {
                    ContentReaderType::None => {}
                    ContentReaderType::Text(t) => t.read_single(&line)?,
                    ContentReaderType::Tasks(t) => t.read_single(&line)?,
                    ContentReaderType::File(f) => f.read_single(&line)?,
                }
            }
        }

        self.end_current_with(ContentReaderType::None)?;
        Ok(ReadCard {
            meta: self.meta,
            content: self.content,
        })
    }

    fn end_current_with(&mut self, next: ContentReaderType) -> Result<()> {
        match std::mem::replace(&mut self.current, next) {
            ContentReaderType::None => {}
            ContentReaderType::Text(t) => {
                self.content
                    .push(ReadContent::Text(t.text.trim().to_string()));
            }
            ContentReaderType::Tasks(t) => {
                self.content.push(ReadContent::Tasks(t.tasks));
            }
            ContentReaderType::File(f) => {
                let path = f.path.ok_or(anyhow!("Expected file to have a path"))?;
                let blob_id = f.blob_id.ok_or(anyhow!("Expected file to have an ID"))?;

                self.content.push(ReadContent::File {
                    blob_id,
                    path,
                    name: f.name,
                });
            }
        }
        Ok(())
    }
}

enum ContentReaderType {
    None,
    Text(TextContentReader),
    Tasks(TasksContentReader),
    File(FileContentReader),
}

enum ReadContent {
    Text(String),
    File {
        #[allow(unused)]
        blob_id: String,
        path: String,
        name: Option<String>,
    },
    Tasks(Vec<CardTask>),
}

#[derive(Default)]
struct TextContentReader {
    text: String,
}

impl TextContentReader {
    fn read_single(&mut self, line: &str) -> Result<()> {
        writeln!(&mut self.text, "{}", line)?;
        Ok(())
    }
}

#[derive(Default)]
struct TasksContentReader {
    tasks: Vec<CardTask>,
}

impl TasksContentReader {
    fn read_single(&mut self, line: &str) -> Result<()> {
        if let Some(task) = line.strip_prefix("* [x] ") {
            self.tasks.push(CardTask {
                text: task.to_string(),
                completed: true,
            });
        } else if let Some(task) = line.strip_prefix("* [ ] ") {
            self.tasks.push(CardTask {
                text: task.to_string(),
                completed: false,
            });
        }

        Ok(())
    }
}

#[derive(Default)]
struct FileContentReader {
    blob_id: Option<String>,
    name: Option<String>,
    path: Option<String>,
}

impl FileContentReader {
    fn read_single(&mut self, line: &str) -> Result<()> {
        if let Some(name) = line.strip_prefix("* Name: ") {
            self.name = Some(name.to_string());
        } else if let Some(blob_id) = line.strip_prefix("* ID: ") {
            self.blob_id = Some(blob_id.to_string());
        } else if line.starts_with("[Link to ") {
            let href = match &self.name {
                Some(name) => line.strip_prefix(&format!("[Link to {}](", name)),
                None => line.strip_prefix("[Link to file]("),
            }
            .ok_or(anyhow!("Cannot find file path in {}", line))?;

            let href = href.strip_suffix(")").unwrap_or(href);
            let path = percent_decode_str(href).decode_utf8()?;
            self.path = Some(path.to_string());
        }

        Ok(())
    }
}

/// Get a mapping from label name to label
fn get_acc_labels(acc: &AccView) -> HashMap<&String, &AccLabel> {
    acc.labels.iter().map(|l| (&l.name, l)).collect()
}

struct CardTask {
    completed: bool,
    text: String,
}

pub struct ImportCardResult {
    pub card: CardView,
}

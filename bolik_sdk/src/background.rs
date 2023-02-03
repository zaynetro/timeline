use std::fmt::Display;

use anyhow::{Context, Result};
use tracing::instrument;

use crate::{
    client::Client,
    output::OutputEvent,
    registry::{Registry, WithInTxn, WithTimelineAtom},
    timeline::card::{CardFile, CardView},
};

pub enum BackgroundInput {
    Sync,
    EmptyBin,
    ProcessFiles(CardView, tokio::sync::oneshot::Sender<()>),
    DownloadFile {
        card_id: String,
        card_file: CardFile,
    },
}

impl Display for BackgroundInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sync => f.write_str("Sync"),
            Self::EmptyBin => f.write_str("EmptyBin"),
            Self::ProcessFiles(card, _) => {
                f.write_fmt(format_args!("ProcessFiles(card_id={})", card.id))
            }
            Self::DownloadFile { card_id, card_file } => f.write_fmt(format_args!(
                "DownloadFile(blob_id={} device_id={} doc_id={})",
                card_file.blob_id, card_file.device_id, card_id
            )),
        }
    }
}

pub struct BackgroundTask<C: Clone> {
    debug_name: String,
    registry: Registry<C>,
}

#[instrument(name = "bg", skip_all, fields(d = task.debug_name))]
pub async fn run<C: Client>(
    mut task: BackgroundTask<C>,
    mut rx: tokio::sync::mpsc::Receiver<BackgroundInput>,
) {
    while let Some(input) = rx.recv().await {
        let input_str = format!("{}", input);

        if let Err(err) = task.process(input).await {
            tracing::warn!("Failed to process input ({}): {:?}", input_str, err);
        }
    }
}

impl<C> BackgroundTask<C>
where
    C: Client,
{
    pub fn new(registry: Registry<C>, debug_name: String) -> Self {
        Self {
            registry,
            debug_name,
        }
    }

    async fn process(&mut self, input: BackgroundInput) -> Result<()> {
        match input {
            BackgroundInput::Sync => match self.sync().await {
                Ok(_) => {
                    self.broadcast(OutputEvent::Synced)?;
                }
                Err(err) => {
                    self.broadcast(OutputEvent::SyncFailed)?;
                    return Err(err);
                }
            },
            BackgroundInput::EmptyBin => self.empty_bin()?,
            BackgroundInput::ProcessFiles(card, sender) => {
                self.process_files(card).await?;
                let _ = sender.send(());
            }
            BackgroundInput::DownloadFile { card_id, card_file } => {
                let ctx = self.registry.db_ctx();
                let card = ctx.in_txn(|ctx_tx| ctx_tx.timeline().get_card(ctx_tx, &card_id))?;
                match self.registry.blobs.download(&ctx, &card, &card_file).await {
                    Ok(path) => {
                        self.broadcast(OutputEvent::DownloadCompleted {
                            blob_id: card_file.blob_id,
                            device_id: card_file.device_id,
                            path,
                        })?;
                    }
                    Err(err) => {
                        self.broadcast(OutputEvent::DownloadFailed {
                            blob_id: card_file.blob_id,
                        })?;
                        return Err(err);
                    }
                }
            }
        };
        Ok(())
    }

    fn broadcast(&self, event: OutputEvent) -> Result<()> {
        self.registry.broadcast.send(event)?;
        Ok(())
    }

    async fn sync(&self) -> Result<()> {
        let ctx = self.registry.db_ctx();

        // Mailbox
        self.registry.mailbox.sync(&ctx).await?;

        // Docs
        self.registry
            .sync_docs
            .sync(&ctx)
            .await
            .context("Sync docs")?;

        Ok(())
    }

    fn empty_bin(&self) -> Result<()> {
        self.registry.in_txn(|ctx, r| {
            if r.account.get_account_id(ctx).is_some() {
                r.timeline.empty_bin(ctx, None)?;
            }
            Ok(())
        })?;
        Ok(())
    }

    async fn process_files(&self, card: CardView) -> Result<()> {
        let ctx = self.registry.db_ctx();
        let res = ctx.in_txn(|ctx_tx| self.registry.timeline.generate_thumbnail(ctx_tx, &card))?;
        if res.card_changes > 0 {
            let _ = self.broadcast(OutputEvent::DocUpdated { doc_id: card.id });
        }
        Ok(())
    }
}

use std::io::Cursor;

use anyhow::{anyhow, bail, Result};
use bolik_migrations::rusqlite::{params, OptionalExtension, Row};
use bolik_proto::sync::doc_payload::DocSchema;
use chrono::{DateTime, Utc};
use image::{
    imageops::FilterType, io::Reader as ImageReader, DynamicImage, GenericImage, GenericImageView,
    ImageError, ImageOutputFormat, Rgba,
};
use uuid::Uuid;

use crate::{
    blobs,
    documents::{self, DbDocRow, DbDocRowMeta},
    registry::{WithAccountAtom, WithBackend, WithDeviceAtom, WithDocsAtom, WithTxn},
    BIN_LABEL_ID,
};

use super::{
    acl_doc::{AclChange, AclDoc},
    card::{
        CardChange, CardFile, CardLabels, CardLabelsChange, CardView, ContentView, FileThumbnail,
    },
};

pub trait TimelineCtx<'a>: WithTxn<'a> + WithAccountAtom + WithDocsAtom + WithDeviceAtom {}
impl<'a, T> TimelineCtx<'a> for T where
    T: WithTxn<'a> + WithAccountAtom + WithDocsAtom + WithDeviceAtom
{
}

#[derive(Clone)]
pub struct TimelineAtom {}

impl TimelineAtom {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get_card<'a>(&self, ctx: &impl TimelineCtx<'a>, id: &str) -> Result<CardView> {
        self.find_card(ctx, id)?.ok_or(anyhow!("Card not found"))
    }

    fn find_card<'a>(&self, ctx: &impl TimelineCtx<'a>, id: &str) -> Result<Option<CardView>> {
        // Read doc and doc labels
        let row = ctx.docs().find(ctx, id)?;
        if let Some(row) = row {
            let labels_row = ctx.docs().find(ctx, &format!("{}/labels", id))?;
            Ok(Some(CardView::from_db(row, labels_row).0))
        } else {
            Ok(None)
        }
    }

    pub fn edit_card<'a>(
        &self,
        ctx: &impl TimelineCtx<'a>,
        id: &str,
        changes: Vec<CardChange>,
    ) -> Result<CardView> {
        let card = self
            .edit_card_opts(
                ctx,
                EditCardOpts {
                    id,
                    changes,
                    acl_changes: vec![],
                    created_at: None,
                    skip_counter: false,
                },
            )?
            .0;
        Ok(card)
    }

    pub fn edit_card_acl<'a>(
        &self,
        ctx: &impl TimelineCtx<'a>,
        id: &str,
        acl_changes: Vec<AclChange>,
    ) -> Result<CardView> {
        let card = self
            .edit_card_opts(
                ctx,
                EditCardOpts {
                    id,
                    changes: vec![],
                    acl_changes,
                    created_at: None,
                    skip_counter: false,
                },
            )?
            .0;
        Ok(card)
    }

    pub fn edit_card_opts<'a>(
        &self,
        ctx: &impl TimelineCtx<'a>,
        opts: EditCardOpts<'_>,
    ) -> Result<(CardView, yrs::Doc)> {
        let id = opts.id;
        let acc_id = ctx.account().require_account_id(ctx)?;

        // Read from the database
        let mut doc_row = match ctx.docs().find(ctx, id)? {
            Some(row) => row,
            None => {
                // If not found create a new one
                let timeline_doc = CardView::init(ctx.device().yrs_client_id);
                let created_at = opts.created_at.unwrap_or(Utc::now());
                let row = DbDocRow {
                    meta: DbDocRowMeta {
                        id: id.to_string(),
                        created_at: created_at.clone(),
                        edited_at: created_at,
                        schema: DocSchema::CardV1 as i32,
                        author_device_id: "".into(),
                        counter: 0,
                    },
                    yrs: timeline_doc,
                    acl: AclDoc::init(ctx.device().yrs_client_id, &acc_id),
                };
                row
            }
        };

        // Apply changes to it
        let yrs_doc = &doc_row.yrs;
        let acl_doc = &doc_row.acl;
        let acl_view = AclDoc::from_doc(acl_doc);

        if !acl_view.allowed_to_edit(&acc_id) {
            bail!("This account is not allowed to edit");
        }

        CardView::edit(yrs_doc, opts.changes);

        if acl_view.allowed_to_admin(&acc_id) {
            for change in opts.acl_changes.into_iter() {
                tracing::debug!(?change);
                match change {
                    AclChange::Add { account_id, rights } => {
                        AclDoc::add(acl_doc, account_id, rights)
                    }
                    AclChange::Remove { account_id } => AclDoc::remove(acl_doc, &account_id),
                    AclChange::MoveToBin => AclDoc::move_to_bin(acl_doc),
                }
            }
        } else if !opts.acl_changes.is_empty() {
            bail!("This account not allowed to edit ACL");
        }

        // Save doc
        doc_row.meta.author_device_id = ctx.device().id.clone();
        if !opts.skip_counter {
            doc_row.meta.counter = ctx.device().increment_clock(ctx)?;
        }
        doc_row.meta.edited_at = Utc::now();
        ctx.docs().save(ctx, &doc_row)?;

        let labels_row = ctx.docs().find(ctx, &format!("{}/labels", id))?;

        // Index the card
        let (view, doc) = CardView::from_db(doc_row, labels_row);
        super::index_card(ctx.txn(), &view)?;

        // TODO: find if any collaborators were removed. If they were then send queue a special doc message to them.
        //       (basically, just push current doc version)

        Ok((view, doc))
    }

    pub fn edit_card_labels<'a>(
        &self,
        ctx: &impl TimelineCtx<'a>,
        card_id: &str,
        changes: Vec<CardLabelsChange>,
    ) -> Result<CardView> {
        let acc_id = ctx.account().require_account_id(ctx)?;
        let card_row = ctx
            .docs()
            .find(ctx, card_id)?
            .ok_or(anyhow!("Can't edit labels for missing card"))?;

        // Read from the database
        let id = format!("{}/labels", card_id);
        let mut doc_row = match ctx.docs().find(ctx, &id)? {
            Some(row) => row,
            None => {
                // If not found create a new one
                let labels_doc = CardLabels::init(ctx.device().yrs_client_id);
                let now = Utc::now();
                let row = DbDocRow {
                    meta: DbDocRowMeta {
                        id: id.to_string(),
                        created_at: now,
                        edited_at: now,
                        schema: DocSchema::CardLabelsV1 as i32,
                        author_device_id: "".into(),
                        counter: 0,
                    },
                    yrs: labels_doc,
                    acl: AclDoc::init(ctx.device().yrs_client_id, &acc_id),
                };
                row
            }
        };

        // Apply changes to it
        let yrs_doc = &doc_row.yrs;
        let acl_doc = &doc_row.acl;
        let acl_view = AclDoc::from_doc(acl_doc);

        if !acl_view.allowed_to_edit(&acc_id) {
            bail!("This account is not allowed to edit");
        }

        for change in changes.into_iter() {
            tracing::debug!(%change);
            match change {
                CardLabelsChange::AddLabel { label_id } => {
                    CardLabels::add_label(&yrs_doc, label_id)
                }
                CardLabelsChange::RemoveLabel { label_id } => {
                    CardLabels::remove_label(&yrs_doc, &label_id)
                }
            }
        }

        // Save doc
        doc_row.meta.author_device_id = ctx.device().id.clone();
        doc_row.meta.counter = ctx.device().increment_clock(ctx)?;
        doc_row.meta.edited_at = Utc::now();
        ctx.docs().save(ctx, &doc_row)?;

        // Index the card
        let view = CardView::from_db(card_row, Some(doc_row)).0;
        super::index_card(ctx.txn(), &view)?;

        Ok(view)
    }

    /// Permanently delete all cards that were moved to been earlier than `till`.
    pub fn empty_bin<'a>(
        &self,
        ctx: &(impl TimelineCtx<'a> + WithBackend),
        till: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let acc_id = ctx.account().require_account_id(ctx)?;
        let till = till.unwrap_or_else(|| Utc::now() - chrono::Duration::days(30));

        // Go through all cards in the bin
        let mut stmt = ctx.txn().prepare(
            r#"
    SELECT d.id, d.acl_data, d.created_at, d2.id, d2.data, d2.created_at
      FROM documents d
      JOIN card_index i ON d.id = i.id
      LEFT JOIN documents d2 ON d.id || '/labels' = d2.id
     WHERE d.schema = ? AND i.label_ids MATCH ?
     ORDER BY d.created_at DESC"#,
        )?;
        let yrs_client_id = 1; // Doesn't matter in this case

        let delete_single = |row: &Row| -> Result<()> {
            // Find when card was added to the bin
            let card_id: String = row.get(0)?;
            let acl_bytes: Vec<u8> = row.get(1)?;
            let acl = documents::build_yrs_doc(yrs_client_id, &acl_bytes)?;
            let created_at: DateTime<Utc> = row.get(2)?;

            let labels_id: Option<String> = row.get(3)?;
            let labels_bytes: Option<Vec<u8>> = row.get(4)?;
            let labels_created_at: Option<DateTime<Utc>> = row.get(5)?;

            // Check ACL doc
            let added_to_bin_at = if let Some(added_at) = AclDoc::in_bin_since(&acl) {
                Some(added_at)
            } else if let Some(bytes) = labels_bytes {
                // Check labels doc
                let labels = documents::build_yrs_doc(yrs_client_id, &bytes)?;
                CardLabels::in_bin_since(&labels)
            } else {
                None
            };
            tracing::trace!(
                card_id,
                in_bin_since = ?added_to_bin_at,
                till = ?till,
                "Checking card for deletion (has_labels={})",
                labels_id.is_some()
            );

            match added_to_bin_at {
                Some(added_at) if added_at < till => {
                    tracing::info!(
                        card_id,
                        "Permanently deleting card (has_labels={})",
                        labels_id.is_some()
                    );

                    let deleted_at = Utc::now();
                    self.permanently_delete(ctx, &card_id, PermanentDeleteOpts::default())?;

                    // Add entries in deleted docs
                    ctx.docs()
                        .add_to_deleted_queue(ctx, &acc_id, &card_id, created_at, deleted_at)?;
                    if let Some(id) = labels_id {
                        ctx.docs().add_to_deleted_queue(
                            ctx,
                            &acc_id,
                            &id,
                            labels_created_at.unwrap_or(deleted_at),
                            deleted_at,
                        )?;
                    }
                }
                _ => {}
            }
            Ok(())
        };

        let mut rows = stmt.query(params![
            DocSchema::CardV1 as i32,
            format!(r#""{}""#, BIN_LABEL_ID)
        ])?;
        while let Some(row) = rows.next()? {
            if let Err(err) = delete_single(row) {
                tracing::warn!("Failed to permanently delete card: {}", err);
            }
        }

        Ok(())
    }

    pub fn permanently_delete<'a>(
        &self,
        ctx: &impl TimelineCtx<'a>,
        card_id: &str,
        opts: PermanentDeleteOpts,
    ) -> Result<()> {
        // Load card
        let Some(card) = self.find_card(ctx, card_id)? else {
            return Ok(());
        };

        if !opts.keep_blobs {
            for block in &card.blocks {
                if let ContentView::File(file) = &block.view {
                    // Remove file from disk
                    let local_path = blobs::get_file_path(ctx.txn(), &file.blob_id)?;
                    if let Some(path) = local_path {
                        if let Err(err) = std::fs::remove_file(&path) {
                            tracing::warn!("Failed to remove a file: {}", err);
                        }
                    }

                    // Clean blobs table
                    blobs::rm_row(ctx.txn(), &file.blob_id)?;
                }
            }
        }

        // Remove labels doc
        documents::delete_row(ctx.txn(), &format!("{}/labels", card_id))?;
        // Remove card doc
        documents::delete_row(ctx.txn(), card_id)?;

        Ok(())
    }

    /// Find first card after offset
    pub fn find_first<'a>(
        &self,
        ctx: &(impl WithTxn<'a> + WithDocsAtom),
        offset: u64,
    ) -> Result<Option<CardView>> {
        let row = ctx.docs().find_first(ctx, DocSchema::CardV1, offset)?;
        if let Some(doc) = row {
            let labels_row = ctx.docs().find(ctx, &format!("{}/labels", doc.meta.id))?;
            Ok(Some(CardView::from_db(doc, labels_row).0))
        } else {
            Ok(None)
        }
    }

    /// Restore card from bin.
    /// Create a copy of the card with new ACL.
    pub fn restore_from_bin<'a>(
        &self,
        ctx: &(impl TimelineCtx<'a> + WithBackend),
        card_id: &str,
    ) -> Result<CardView> {
        let acc_id = ctx.account().require_account_id(ctx)?;
        let mut row = ctx
            .docs()
            .find(ctx, card_id)?
            .ok_or(anyhow!("Card not found"))?;
        let mut labels_row = ctx.docs().find(ctx, &format!("{}/labels", card_id))?;
        let created_at = row.meta.created_at;

        // Generate new ID and reset ACL
        row.meta.id = Uuid::new_v4().to_string();
        row.acl = AclDoc::init(ctx.device().yrs_client_id, &acc_id);
        row.meta.author_device_id = ctx.device().id.clone();
        row.meta.counter = ctx.device().increment_clock(ctx)?;

        // Set new labels id, reset ACL and remove bin label
        if let Some(labels) = &mut labels_row {
            labels.meta.id = format!("{}/labels", row.meta.id);
            labels.acl = AclDoc::init(ctx.device().yrs_client_id, &acc_id);
            labels.meta.author_device_id = ctx.device().id.clone();
            labels.meta.counter = ctx.device().increment_clock(ctx)?;
            CardLabels::remove_label(&labels.yrs, BIN_LABEL_ID);
            ctx.docs().save(ctx, labels)?;
        }

        ctx.docs().save(ctx, &row)?;
        let view = CardView::from_db(row, labels_row).0;
        super::index_card(ctx.txn(), &view)?;

        // We don't want to show old card in the bin
        if let Err(err) = self
            .permanently_delete(ctx, card_id, PermanentDeleteOpts { keep_blobs: true })
            .and_then(|_| {
                ctx.docs()
                    .add_to_deleted_queue(ctx, &acc_id, card_id, created_at, Utc::now())
            })
        {
            tracing::warn!("Cannot permanently delete after restoring a copy: {}", err);
        }

        Ok(view)
    }

    pub fn generate_thumbnail<'a>(
        &self,
        ctx: &impl TimelineCtx<'a>,
        card: &CardView,
    ) -> Result<GenThumbResult> {
        let mut changes = vec![];

        let mut has_files = false;
        let had_thumbnail = card.thumbnail.is_some();
        let mut has_thumbnail = false;

        for block in &card.blocks {
            if let ContentView::File(f) = &block.view {
                has_files = true;

                match &card.thumbnail {
                    Some(e) if e.from_checksum == f.checksum => {
                        // Image hasn't changed (keep existing thumbnail)
                        has_thumbnail = true;
                        break;
                    }
                    _ => {}
                }

                let blob = match blobs::find_by_id(ctx.txn(), &f.blob_id, &f.device_id)? {
                    Some(b) => b,
                    None => {
                        continue;
                    }
                };

                match create_thumbnail(&blob.path, &f) {
                    Ok(ThumbnailResult::Created(thumb)) => {
                        has_thumbnail = true;
                        changes.push(CardChange::SetThumbnail(Some(thumb)));
                        break;
                    }
                    Ok(ThumbnailResult::Skipped) => {}
                    Err(err) => {
                        tracing::warn!("Cannot generate thumbnail: {:?}", err);
                    }
                }
            }
        }

        // Reset thumbnail when all images were removed
        if has_files && had_thumbnail && !has_thumbnail {
            changes.push(CardChange::SetThumbnail(None));
        }

        let changes_len = changes.len();
        if !changes.is_empty() {
            self.edit_card(ctx, &card.id, changes)?;
        }

        Ok(GenThumbResult {
            card_changes: changes_len,
        })
    }

    /// Return if provided card has been indexed.
    pub fn is_indexed<'a>(&self, ctx: &impl WithTxn<'a>, card_id: &str) -> Result<bool> {
        let found = ctx
            .txn()
            .query_row(
                "SELECT 1 FROM card_index WHERE id = ?",
                [&card_id],
                |_row| Ok(()),
            )
            .optional()?;
        Ok(found.is_some())
    }

    /// Index card that haven't been indexed yet.
    pub fn index_missing<'a>(&self, ctx: &impl TimelineCtx<'a>, card_id: &str) -> Result<bool> {
        if !self.is_indexed(ctx, card_id)? {
            if let Some(card) = self.find_card(ctx, card_id)? {
                tracing::info!(card_id, "Indexing missing card");
                super::index_card(ctx.txn(), &card)?;
                return Ok(true);
            }
        }
        Ok(false)
    }
}

pub struct EditCardOpts<'a> {
    pub id: &'a str,
    pub changes: Vec<CardChange>,
    pub acl_changes: Vec<AclChange>,
    /// Override created_at when creating a new card
    pub created_at: Option<DateTime<Utc>>,
    /// Skip incrementing local counter
    pub skip_counter: bool,
}

fn create_thumbnail(file_path: &str, card_file: &CardFile) -> Result<ThumbnailResult> {
    // Read the file
    let reader = ImageReader::open(file_path)?;
    let img = match reader.decode() {
        Ok(img) => img,
        Err(ImageError::Unsupported(_)) => {
            return Ok(ThumbnailResult::Skipped);
        }
        Err(err) => {
            return Err(anyhow!(err));
        }
    };

    let max_width = 400;
    let max_height = 400;
    if img.width() < max_width && img.height() < max_height {
        // Image is too small for thumbnails
        tracing::debug!("Not creating a thumbnail: Image is too small");
        return Ok(ThumbnailResult::Skipped);
    }

    // Generate a thumbnail
    tracing::debug!(
        blob_id = card_file.blob_id,
        "Generating thumbnail from {}x{}",
        img.width(),
        img.height()
    );
    let mut bytes = vec![];
    let thumb = img.resize_to_fill(max_width, max_height, FilterType::Triangle);
    let mut thumb = match thumb_rotate(file_path, &thumb) {
        Ok(Some(t)) => t,
        Ok(None) => thumb,
        Err(err) => {
            tracing::warn!("Failed to rotate thumbnail: {}", err);
            thumb
        }
    };

    // Use white color instead of transparent background
    for x in 0..thumb.width() {
        for y in 0..thumb.height() {
            let pixel = thumb.get_pixel(x, y);
            if pixel.0[3] == 0 {
                // This pixel is fully transparent. Set to white color.
                thumb.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
    }

    thumb.write_to(&mut Cursor::new(&mut bytes), ImageOutputFormat::Jpeg(75))?;
    let mime_type = "image/jpeg";

    Ok(ThumbnailResult::Created(FileThumbnail {
        mime_type: mime_type.to_string(),
        width: thumb.width(),
        height: thumb.height(),
        data: bytes,
        from_checksum: card_file.checksum.clone(),
    }))
}

/// Rotate thumbnail based on the orientation tag present in EXIF data.
fn thumb_rotate(file_path: &str, thumb: &DynamicImage) -> Result<Option<DynamicImage>> {
    let file = std::fs::File::open(file_path)?;
    let mut bufreader = std::io::BufReader::new(&file);
    let exifreader = exif::Reader::new();
    let exif_info = exifreader.read_from_container(&mut bufreader)?;

    // Refs:
    // - https://pub.dev/documentation/image/latest/image/bakeOrientation.html
    // - https://magnushoff.com/articles/jpeg-orientation/
    let thumb = match exif_info
        .get_field(exif::Tag::Orientation, exif::In::PRIMARY)
        .and_then(|f| f.value.get_uint(0))
    {
        // Do nothing
        Some(1) => {
            return Ok(None);
        }
        // Flip horizontally
        Some(2) => thumb.fliph(),
        // Rotate 180°
        Some(3) => thumb.rotate180(),
        // Flip vertically
        Some(4) => thumb.flipv(),
        // Rotate 90° and flip horizontally
        Some(5) => thumb.rotate90().fliph(),
        // Rotate 90°
        Some(6) => thumb.rotate90(),
        // Flip horizontally and Rotate 90°
        Some(7) => thumb.fliph().rotate90(),
        // Rotate 270°
        Some(8) => thumb.rotate270(),
        v @ _ => {
            bail!("Unknown image orientation={:?}", v);
        }
    };
    Ok(Some(thumb))
}

enum ThumbnailResult {
    /// File is not an image or image is too small
    Skipped,
    /// Create new thumbnail
    Created(FileThumbnail),
}

pub struct GenThumbResult {
    pub card_changes: usize,
}

#[derive(Default)]
pub struct PermanentDeleteOpts {
    /// We want to keep local blobs when copying a card and deleting an old version.
    pub keep_blobs: bool,
}

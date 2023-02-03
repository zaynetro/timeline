use std::{io::Read, path::Path};

use anyhow::Result;
use bolik_migrations::rusqlite::{params, Connection, OptionalExtension, Params};
use bytes::BytesMut;
use multihash::{Blake3_256, Hasher};
use tokio_util::codec::Decoder;
use uuid::Uuid;

use crate::{secrets, timeline::card::CardFile};

pub struct BlobRef {
    pub id: String,
    pub device_id: String,
    pub checksum: String,
    pub path: String,
    pub synced: bool,
}

mod blobs_atom;
pub use blobs_atom::{BlobsAtom, BlobsCtx};

fn query_row<P>(conn: &Connection, query: &str, params: P) -> Result<Option<BlobRef>>
where
    P: Params,
{
    let row = conn
        .query_row(query, params, |row| {
            Ok(BlobRef {
                id: row.get(0)?,
                device_id: row.get(1)?,
                checksum: row.get(2)?,
                path: row.get(3)?,
                synced: row.get(4)?,
            })
        })
        .optional()?;
    Ok(row)
}

pub fn find_by_id(conn: &Connection, blob_id: &str, device_id: &str) -> Result<Option<BlobRef>> {
    query_row(
        conn,
        r#"
SELECT
  id,
  device_id,
  checksum,
  path,
  synced
 FROM blobs
WHERE id = ? AND device_id = ?"#,
        params![blob_id, device_id],
    )
}

pub struct SaveFileParams<'a> {
    pub blob_dir: &'a Path,
    pub card_id: &'a str,
    pub path: &'a Path,
    pub original_file_name: Option<String>,
    pub device_id: String,
}

/// Save selected blob (when user picks a file)
pub fn save_file(conn: &Connection, params: SaveFileParams<'_>) -> Result<CardFile> {
    // Calculate checksum
    let path = params.path;
    let (checksum, file_size_bytes) = hash_file(path)?;
    tracing::debug!(?checksum, "Saving file {}", path.display());

    let original_file_name = params.original_file_name.or_else(|| {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_string())
    });

    // Copy to app support dir and insert ref to db (blobs table)
    let blob_id = Uuid::new_v4().to_string();
    let blob_file_name = build_blob_file_name(&original_file_name, &blob_id, params.card_id);
    let blob_path = params.blob_dir.join(&blob_file_name);

    tracing::debug!(?blob_id, "Copying to={}", blob_path.display());
    std::fs::copy(path, &blob_path)?;

    let blob_path_str = format!("{}", blob_path.display());
    let blob_ref = BlobRef {
        id: blob_id,
        device_id: params.device_id,
        checksum,
        path: blob_path_str,
        synced: false,
    };
    save(conn, &blob_ref)?;

    Ok(CardFile {
        blob_id: blob_ref.id,
        device_id: blob_ref.device_id,
        checksum: blob_ref.checksum,
        size_bytes: file_size_bytes as u32,
        name: original_file_name,
        // TODO:
        dimensions: None,
    })
}

pub fn mark_synced(conn: &Connection, blob_id: &str) -> Result<()> {
    conn.execute("UPDATE blobs SET synced = 1 WHERE id = ?", params![blob_id])?;
    Ok(())
}

/// Remove row from blobs table
pub fn rm_row(conn: &Connection, blob_id: &str) -> Result<()> {
    conn.execute("DELETE FROM blobs WHERE id = ?", [blob_id])?;
    Ok(())
}

/// Get local file by id and return path
pub fn get_file_path(conn: &Connection, blob_id: &str) -> Result<Option<String>> {
    let path = conn
        .query_row(
            "SELECT path FROM blobs WHERE id = ?",
            params![blob_id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(path)
}

/// Insert a new row
pub fn save(conn: &Connection, blob: &BlobRef) -> Result<()> {
    conn.execute(
        r#"
INSERT INTO blobs (id, device_id, checksum, path, synced) VALUES (?1, ?2, ?3, ?4, ?5)
  ON CONFLICT (id, device_id) DO UPDATE
    SET checksum = excluded.checksum,
        path = excluded.path,
        synced = excluded.synced"#,
        params![
            blob.id,
            blob.device_id,
            blob.checksum,
            blob.path,
            blob.synced,
        ],
    )?;
    Ok(())
}

/// Calculate file checksum
pub fn hash_file(path: impl AsRef<Path>) -> Result<(String, u32)> {
    let mut hasher = Blake3_256::default();
    let mut file = std::fs::File::open(path.as_ref())?;
    let mut buffer = [0u8; 16384];
    let mut file_size_bytes = 0;

    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }

        hasher.update(&buffer[..count]);
        file_size_bytes += count;
    }
    let checksum = secrets::id_from_key(hasher.finalize());
    Ok((checksum, file_size_bytes as u32))
}

/// Read frames in a chunks of expected size. Last chunk could be less than expected.
struct FixedBytesCodec {
    expected: usize,
    processed: usize,
    total: usize,
}

impl FixedBytesCodec {
    fn new(expected: usize, total: usize) -> Self {
        Self {
            expected,
            processed: 0,
            total,
        }
    }
}

impl Decoder for FixedBytesCodec {
    type Item = BytesMut;
    type Error = std::io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<BytesMut>, std::io::Error> {
        if buf.len() >= self.expected {
            // We have enough bytes
            self.processed += self.expected;
            Ok(Some(buf.split_to(self.expected)))
        } else if (buf.len() + self.processed) == self.total {
            // We have reached the end
            if buf.len() == 0 {
                Ok(None)
            } else {
                self.processed += buf.len();
                Ok(Some(buf.split_to(buf.len())))
            }
        } else {
            Ok(None)
        }
    }
}

fn build_blob_file_name(original_name: &Option<String>, blob_id: &str, card_id: &str) -> String {
    let short_card_id: String = card_id.chars().take(6).collect();

    let pair = original_name.as_ref().map(|name| {
        let path = Path::new(name);
        let stem = path.file_stem().and_then(|s| s.to_str());
        let ext = path.extension().and_then(|ext| ext.to_str());
        (stem, ext)
    });
    match pair {
        Some((Some(stem), Some(ext))) => {
            format!("{} (version {}).{}", stem, short_card_id, ext)
        }
        Some((Some(stem), None)) => {
            format!("{} (version {})", stem, short_card_id)
        }
        Some((None, Some(ext))) => {
            format!("{} (version {}).{}", blob_id, short_card_id, ext)
        }
        _ => format!("{} (version {})", blob_id, short_card_id),
    }
}

/// Delete all blobs stored locally.
pub fn dangerous_delete_all(blob_dir: &Path) -> Result<()> {
    std::fs::remove_dir_all(blob_dir)?;
    Ok(())
}

use anyhow::{anyhow, bail, Result};
use bolik_proto::sync::request;
use chacha20poly1305::{
    aead::{
        generic_array::{typenum::Unsigned, GenericArray},
        stream::{DecryptorBE32, EncryptorBE32},
    },
    consts::U7,
    ChaCha20Poly1305,
};
use multihash::{Blake3_256, Hasher};
use std::path::Path;
use tokio::{fs::File, io::AsyncWriteExt};
use tokio_stream::StreamExt;
use tokio_util::{
    codec::FramedRead,
    io::{ReaderStream, StreamReader},
};
use tracing::instrument;

use crate::{
    blobs::FixedBytesCodec,
    client::Client,
    documents::DocSecret,
    registry::{WithDeviceAtom, WithInTxn, WithTxn},
    secrets,
    timeline::card::{CardFile, CardView},
};

use super::BlobRef;

/// Buffer size to use when encrypting blobs.
const STREAM_BUF_SIZE: usize = 16368;
/// Authentication tag size of a single encrypted chunk.
const AUTH_TAG_SIZE: usize = 16;

pub trait BlobsCtx<C: Clone>: WithInTxn<C> + WithDeviceAtom {}
impl<T, C: Clone> BlobsCtx<C> for T where T: WithInTxn<C> + WithDeviceAtom {}

#[derive(Clone)]
pub struct BlobsAtom<C: Clone> {
    client: C,
}

impl<C: Client> BlobsAtom<C> {
    pub fn new(client: C) -> Self {
        Self { client }
    }

    /// Download a blob from remote
    #[instrument(skip_all, fields(blob_id = file.blob_id))]
    pub async fn download(
        &self,
        ctx: &impl BlobsCtx<C>,
        card: &CardView,
        file: &CardFile,
    ) -> Result<String> {
        let secret = if let Some(s) = card.secrets.get(&file.blob_id) {
            DocSecret::new(&card.id, &s.secret)
        } else {
            bail!("File secret not found blob_id={}", file.blob_id);
        };

        // Download the file
        let path = self
            .download_blob(&ctx.device().blobs_dir, &card.id, file, secret)
            .await?;

        // Mark blob as synced
        let blob_ref = BlobRef {
            id: file.blob_id.clone(),
            device_id: file.device_id.clone(),
            checksum: file.checksum.clone(),
            path,
            synced: true,
        };
        ctx.in_txn(|tx_ctx| super::save(tx_ctx.txn(), &blob_ref))?;
        Ok(blob_ref.path)
    }

    /// Download a blob by checksum and return path
    async fn download_blob(
        &self,
        blob_dir: &Path,
        card_id: &str,
        file: &CardFile,
        secret: DocSecret,
    ) -> Result<String> {
        tracing::debug!(%file.checksum, %secret.id, "Downloading blob...");
        // Fetch file from remote using blob id
        let blob_id = &file.blob_id;
        let (stream_size, stream) = self
            .client
            .download_blob(&request::PresignDownload {
                blob_id: blob_id.clone(),
                device_id: file.device_id.clone(),
                doc_id: card_id.to_string(),
            })
            .await?;
        tokio::pin!(stream);

        let tmp_file_path = blob_dir.join(format!(".tmp-{}-{}", blob_id, file.device_id));
        let mut tmp_file = tokio::fs::File::create(&tmp_file_path).await?;
        let mut decryptor = stream_decryptor(secret.cipher, &file.checksum);
        let mut hasher = Blake3_256::default();

        // We need to read the blob in the same sized chunks as it was used to be uploaded.
        // Except for the addition of authentication tag overhead.
        const BUFFER_LEN: usize = STREAM_BUF_SIZE + AUTH_TAG_SIZE;

        let stream_reader = StreamReader::new(
            stream.map(|c| c.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))),
        );
        let mut framed = FramedRead::new(
            stream_reader,
            FixedBytesCodec::new(BUFFER_LEN, stream_size as usize),
        );
        let mut read = 0;

        while let Some(frame) = framed.next().await {
            let encrypted_bytes = frame?;
            read += encrypted_bytes.len();
            tracing::trace!(
                "Download progress {}/{} (chunk={})",
                read,
                stream_size,
                encrypted_bytes.len()
            );

            // Decrypt
            let bytes = decryptor
                .decrypt_next(encrypted_bytes.as_ref())
                .map_err(|err| anyhow!("DecryptStream: {}", err))?;

            // Write to a file
            tmp_file.write(&bytes).await?;
            // Calculate checksum
            hasher.update(&bytes);
        }

        tmp_file.sync_all().await?;
        let checksum = secrets::id_from_key(hasher.finalize());
        tracing::debug!(?checksum, "Downloaded remote blob.");

        if file.checksum != checksum {
            tokio::fs::remove_file(&tmp_file_path).await?;
            return Err(anyhow!("Downloaded remote has mismatching checksum"));
        }

        let blob_file_name = super::build_blob_file_name(&file.name, &file.blob_id, card_id);
        let blob_path = blob_dir.join(&blob_file_name);
        tokio::fs::rename(&tmp_file_path, &blob_path).await?;

        let path = format!("{}", blob_path.display());
        Ok(path)
    }

    /// Upload a blob
    pub async fn upload_blob(
        &self,
        _ctx: &impl BlobsCtx<C>,
        blob: &BlobRef,
        secret: &DocSecret,
    ) -> Result<()> {
        let mut stream_cipher = stream_encryptor(secret.cipher.clone(), &blob.checksum);
        let file = File::open(&blob.path).await?;
        let file_size = file.metadata().await?.len();
        tracing::debug!(%secret.id, %blob.checksum, %file_size);
        let mut written = 0;

        let file_stream =
            ReaderStream::with_capacity(file, STREAM_BUF_SIZE).map(move |chunk| match chunk {
                Ok(bytes) => {
                    written += bytes.len();
                    tracing::trace!(
                        "Upload progress {}/{} (chunk={})",
                        written,
                        file_size,
                        bytes.len()
                    );
                    stream_cipher
                        .encrypt_next(bytes.as_ref())
                        .map_err(|err| anyhow!("{:?}", err))
                }
                Err(err) => Err(anyhow!("{:?}", err)),
            });
        // Amount of chunks fully filled
        let full_chunks = file_size / STREAM_BUF_SIZE as u64;
        let last_chunk_len = file_size % STREAM_BUF_SIZE as u64;
        // Each chunk is extended by authentication tag.
        let content_len = full_chunks * (STREAM_BUF_SIZE + AUTH_TAG_SIZE) as u64
            + last_chunk_len
            + AUTH_TAG_SIZE as u64;

        self.client
            .upload_blob(&blob.id, content_len, file_stream)
            .await?;
        Ok(())
    }
}

fn stream_encryptor(cipher: ChaCha20Poly1305, checksum: &str) -> EncryptorBE32<ChaCha20Poly1305> {
    let nonce = GenericArray::from_iter(checksum.bytes().take(U7::USIZE));
    EncryptorBE32::from_aead(cipher, &nonce)
}

fn stream_decryptor(cipher: ChaCha20Poly1305, checksum: &str) -> DecryptorBE32<ChaCha20Poly1305> {
    let nonce = GenericArray::from_iter(checksum.bytes().take(U7::USIZE));
    DecryptorBE32::from_aead(cipher, &nonce)
}

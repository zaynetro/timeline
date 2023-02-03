use std::{future::Future, time::Duration};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bolik_proto::{
    prost::Message,
    sync::{request, response, DeviceVectorClock, KeyPackageMessage},
};
use bytes::Bytes;
use chrono::Utc;
use openmls::{ciphersuite::signature::SignaturePrivateKey, prelude::TlsSerializeTrait};
use openmls_rust_crypto::OpenMlsRustCrypto;
use reqwest::{header::HeaderMap, Body, RequestBuilder, Response};
use tokio_stream::Stream;

use crate::secrets;

#[derive(Clone, Default)]
pub struct ClientConfig {
    pub host: String,
    #[cfg(test)]
    pub mock_server: mock::MockServerArc,
}

impl ClientConfig {
    pub fn with_host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }
}

#[derive(Clone)]
pub struct HttpClient {
    signature_key: SignaturePrivateKey,
    client: reqwest::Client,
    conf: ClientConfig,
}

impl HttpClient {
    async fn post_bytes(&self, method: &str, bytes: Vec<u8>) -> Result<()> {
        let res = self
            .send_signed(
                self.client
                    .post(format!("{}/{}", self.conf.host, method))
                    .body(bytes),
            )?
            .await?;
        Self::expect_success(method, res).await?;
        Ok(())
    }

    fn send_signed(
        &self,
        builder: RequestBuilder,
    ) -> Result<impl Future<Output = Result<Response, reqwest::Error>>> {
        let mut req = builder.build()?;
        let timestamp = format!("{}", Utc::now().timestamp());

        let mut payload = vec![];
        payload.extend(timestamp.as_bytes());
        payload.extend(req.method().as_str().as_bytes());
        payload.extend(req.url().path().as_bytes());
        if let Some(query) = req.url().query() {
            payload.extend(query.as_bytes());
        }

        let backend = &OpenMlsRustCrypto::default();
        let signature = self.signature_key.sign(backend, payload.as_ref())?;
        let encoded = signature.tls_serialize_detached()?;
        let signature_str = secrets::id_from_key(encoded.as_ref());

        let headers = req.headers_mut();
        headers.insert("signature", signature_str.parse()?);
        headers.insert("timestamp", timestamp.parse()?);

        Ok(self.client.execute(req))
    }

    async fn expect_success(method: &str, res: Response) -> Result<Response> {
        let status = res.status();
        if status.is_success() {
            Ok(res)
        } else {
            let body = res.bytes().await?;
            let text = String::from_utf8_lossy(&body);
            Err(anyhow!(
                "{} returned status={} body={}",
                method,
                status,
                text
            ))
        }
    }

    async fn download_s3(&self, url: &str) -> Result<Response> {
        let req = self
            .client
            .get(url)
            .timeout(Duration::from_secs(60 * 5))
            .build()?;
        let res = self.client.execute(req).await?;
        let res = Self::expect_success("Download blob from S3", res).await?;
        Ok(res)
    }
}

#[async_trait]
pub trait Client: Clone + Send + Sync {
    type BlobStream: Stream<Item = reqwest::Result<Bytes>> + Send + Sync;

    fn new(
        conf: ClientConfig,
        device_id: String,
        signature_key: SignaturePrivateKey,
    ) -> Result<Self>;

    async fn upload_key_package(&self, package: KeyPackageMessage) -> Result<()>;

    async fn push_mailbox(&self, message: request::PushMailbox) -> Result<()>;
    async fn fetch_mailbox(&self) -> Result<response::Mailbox>;
    async fn ack_mailbox_message(
        &self,
        message_id: &str,
        info: request::AckMailboxInfo,
    ) -> Result<()>;

    async fn upload_blob(
        &self,
        blob_id: &str,
        content_len: u64,
        encrypted_stream: impl Stream<Item = anyhow::Result<Vec<u8>>> + Send + Sync + 'static,
    ) -> Result<()>;
    async fn download_blob(
        &self,
        payload: &request::PresignDownload,
    ) -> Result<(u32, Self::BlobStream)>;

    async fn get_account_devices(&self, account_id: &str) -> Result<response::AccountDevices>;
    async fn get_device_packages(&self, device_id: &str) -> Result<response::DevicePackages>;

    async fn fetch_docs(&self, clock: &DeviceVectorClock) -> Result<response::AccountDocs>;
    async fn get_doc_version(
        &self,
        doc_id: &str,
        author_device_id: &str,
    ) -> Result<response::DocVersion>;
    async fn push_doc(&self, doc: request::DocMessage) -> Result<()>;
}

#[async_trait]
impl Client for HttpClient {
    type BlobStream = impl Stream<Item = reqwest::Result<Bytes>> + Send + Sync;

    fn new(
        conf: ClientConfig,
        device_id: String,
        signature_key: SignaturePrivateKey,
    ) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert("device-id", device_id.parse()?);

        let sdk_version = std::env!("CARGO_PKG_VERSION");
        let user_agent = format!("bolik-rust-sdk ({})", sdk_version);
        headers.insert("user-agent", user_agent.parse()?);

        let client = reqwest::ClientBuilder::new()
            .user_agent("bolik-timeline")
            .timeout(Duration::from_secs(30))
            .default_headers(headers)
            .build()?;

        Ok(Self {
            conf,
            signature_key,
            client,
        })
    }

    async fn upload_key_package(&self, package: KeyPackageMessage) -> Result<()> {
        let res = self
            .post_bytes("key-package", package.encode_to_vec())
            .await?;
        Ok(res)
    }

    async fn push_mailbox(&self, message: request::PushMailbox) -> Result<()> {
        let res = self.post_bytes("mailbox", message.encode_to_vec()).await?;
        Ok(res)
    }

    async fn fetch_mailbox(&self) -> Result<response::Mailbox> {
        let res = self
            .send_signed(self.client.get(format!("{}/mailbox", self.conf.host)))?
            .await?;

        let res = Self::expect_success("fetch_mailbox", res).await?;
        let mut body = res.bytes().await?;
        let mailbox = response::Mailbox::decode(&mut body)?;
        Ok(mailbox)
    }

    async fn ack_mailbox_message(
        &self,
        message_id: &str,
        info: request::AckMailboxInfo,
    ) -> Result<()> {
        let res = self
            .send_signed(
                self.client
                    .delete(format!("{}/mailbox/ack/{}", self.conf.host, message_id))
                    .body(info.encode_to_vec()),
            )?
            .await?;
        Self::expect_success("ack_mailbox", res).await?;
        Ok(())
    }

    async fn upload_blob(
        &self,
        blob_id: &str,
        content_len: u64,
        encrypted_stream: impl Stream<Item = anyhow::Result<Vec<u8>>> + Send + Sync + 'static,
    ) -> Result<()> {
        // First get upload URL
        let res = self
            .send_signed(
                self.client
                    .put(format!("{}/blobs/upload", self.conf.host))
                    .body(
                        request::PresignUpload {
                            blob_id: blob_id.into(),
                            size_bytes: content_len,
                        }
                        .encode_to_vec(),
                    ),
            )?
            .await?;
        let res = Self::expect_success("Presign upload", res).await?;
        let mut body = res.bytes().await?;
        let url = response::PresignedUrl::decode(&mut body)?;
        tracing::debug!(
            url = url.url,
            content_len = content_len,
            "Uploading blob to S3"
        );

        // Then upload file directly to returned URL
        let req = self
            .client
            .put(url.url)
            .header("content-type", "application/octet-stream")
            .header("content-length", content_len.to_string())
            .body(Body::wrap_stream(encrypted_stream))
            .timeout(Duration::from_secs(60 * 5))
            .build()?;

        let upload_res = self.client.execute(req).await?;
        Self::expect_success("Upload blob to S3", upload_res).await?;
        Ok(())
    }

    async fn download_blob(
        &self,
        payload: &request::PresignDownload,
    ) -> Result<(u32, Self::BlobStream)> {
        // First get download URL
        let res = self
            .send_signed(
                self.client
                    .put(format!("{}/blobs/download", self.conf.host))
                    .body(payload.encode_to_vec()),
            )?
            .await?;
        let res = Self::expect_success("Presign download", res).await?;
        let mut body = res.bytes().await?;
        let url = response::PresignedUrl::decode(&mut body)?;
        tracing::debug!(url = url.url, "Downloading blob from S3");

        // Then download directly from returned URL (Try several times)
        let download_res = match self.download_s3(&url.url).await {
            Ok(res) => res,
            Err(err) => {
                tracing::warn!("{:?}, retrying...", err);
                tokio::time::sleep(Duration::from_secs(2)).await;
                self.download_s3(&url.url).await?
            }
        };

        let content_len = download_res
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or_default();
        return Ok((content_len, download_res.bytes_stream()));
    }

    async fn get_account_devices(&self, account_id: &str) -> Result<response::AccountDevices> {
        let res = self
            .send_signed(
                self.client
                    .get(format!("{}/account/{}/devices", self.conf.host, account_id)),
            )?
            .await?;
        let res = Self::expect_success("get_account_devices", res).await?;
        let mut body = res.bytes().await?;
        let devices = response::AccountDevices::decode(&mut body)?;
        Ok(devices)
    }

    async fn get_device_packages(&self, device_id: &str) -> Result<response::DevicePackages> {
        let res = self
            .send_signed(
                self.client
                    .get(format!("{}/device/{}/packages", self.conf.host, device_id)),
            )?
            .await?;
        let res = Self::expect_success("get_device_packages", res).await?;
        let mut body = res.bytes().await?;
        let devices = response::DevicePackages::decode(&mut body)?;
        Ok(devices)
    }

    async fn fetch_docs(&self, clock: &DeviceVectorClock) -> Result<response::AccountDocs> {
        let data = clock.encode_to_vec();
        let res = self
            .send_signed(
                self.client
                    .post(format!("{}/docs/list", self.conf.host))
                    .body(data),
            )?
            .await?;
        let res = Self::expect_success("fetch_docs", res).await?;
        let mut body = res.bytes().await?;
        let docs = response::AccountDocs::decode(&mut body)?;
        Ok(docs)
    }

    async fn get_doc_version(
        &self,
        doc_id: &str,
        author_device_id: &str,
    ) -> Result<response::DocVersion> {
        let res = self
            .send_signed(self.client.get(format!(
                "{}/docs/version/{}/{}",
                self.conf.host, doc_id, author_device_id
            )))?
            .await?;
        let res = Self::expect_success("get_doc_version", res).await?;
        let mut body = res.bytes().await?;
        let doc = response::DocVersion::decode(&mut body)?;
        Ok(doc)
    }

    async fn push_doc(&self, message: request::DocMessage) -> Result<()> {
        let data = message.encode_to_vec();
        let data_len = data.len();
        let res = self
            .send_signed(
                self.client
                    .post(format!("{}/docs", self.conf.host))
                    .body(data),
            )?
            .await?;

        tracing::trace!(
            doc_id = message.id,
            "Uploading doc message_size={}KB",
            data_len / 1000
        );
        Self::expect_success("push_doc", res).await?;
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod mock {
    use std::{
        cmp::max,
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    use anyhow::{bail, Context};
    use bolik_chain::SignatureChain;
    use bolik_proto::sync::KeyPackageMessage;
    use openmls::prelude::{
        hash_ref::HashReference, KeyPackage, MlsMessageIn, TlsDeserializeTrait,
    };
    use openmls_rust_crypto::RustCrypto;
    use tokio_stream::StreamExt;

    use crate::device::get_device_id;

    use super::*;

    #[derive(Default)]
    pub struct MockClientData {
        // Called methods
        key_packages: Vec<KeyPackageMessage>,
        pushed_mailbox: Vec<request::PushMailbox>,
        uploaded_docs: Vec<request::DocMessage>,
        downloaded_blobs: Vec<String>,
    }

    struct ServerGroup {
        chain: SignatureChain,
        is_account: bool,
    }

    pub type MockServerArc = Arc<Mutex<MockServer>>;

    #[derive(Default)]
    pub struct MockServer {
        /// Mapping per key ref
        key_packages: HashMap<HashReference, KeyPackage>,
        /// Mailbox per device id
        mailboxes: HashMap<String, Vec<MailboxEntry>>,
        /// Known groups per group id
        groups: HashMap<String, ServerGroup>,
        /// Documents per per account id
        documents: HashMap<String, Vec<response::DocVersion>>,
        /// Blobs per blob id
        blobs: HashMap<String, Vec<u8>>,
    }

    struct MailboxEntry {
        from_device_id: String,
        entry: response::mailbox::Entry,
    }

    impl MockServer {
        fn upload_key_package(&mut self, message: KeyPackageMessage) -> Result<()> {
            let package = KeyPackage::tls_deserialize(&mut message.data.as_slice())?;
            let key_ref = package.hash_ref(&RustCrypto::default())?;
            self.key_packages.insert(key_ref, package);
            Ok(())
        }

        fn push_mailbox(
            &mut self,
            from_device_id: &str,
            message: request::PushMailbox,
        ) -> Result<()> {
            match message.value {
                Some(request::push_mailbox::Value::Account(a)) => {
                    // Save account signature chain
                    let chain_msg = a.chain.ok_or(anyhow!("SignatureChain is missing"))?;
                    let chain = SignatureChain::decode(chain_msg.clone())?;
                    let group_id = chain.root().to_string();
                    self.groups.insert(
                        group_id,
                        ServerGroup {
                            chain,
                            is_account: true,
                        },
                    );
                }
                Some(request::push_mailbox::Value::Commit(c)) => {
                    // Update signature chain
                    let chain_msg = c
                        .chain
                        .clone()
                        .ok_or(anyhow!("SignatureChain is missing"))?;
                    let chain = SignatureChain::decode(chain_msg.clone())?;
                    let group_id = chain.root().to_string();

                    let is_account = self
                        .groups
                        .get(&group_id)
                        .map(|g| g.is_account)
                        .unwrap_or(false);
                    self.groups.insert(
                        group_id,
                        ServerGroup {
                            chain: chain.clone(),
                            is_account,
                        },
                    );

                    // Forward message to members
                    let mls_message = MlsMessageIn::tls_deserialize(&mut c.mls.as_slice())?;
                    let epoch = mls_message.epoch().as_u64();
                    let members = chain.members_at(epoch, &RustCrypto::default())?;
                    for device_id in members.device_ids() {
                        if device_id != from_device_id {
                            let mailbox = self
                                .mailboxes
                                .entry(device_id.to_string())
                                .or_insert(vec![]);
                            mailbox.push(MailboxEntry {
                                from_device_id: from_device_id.to_string(),
                                entry: response::mailbox::Entry {
                                    id: message.id.clone(),
                                    value: Some(response::mailbox::entry::Value::Message(
                                        response::SecretGroupMessage {
                                            mls: c.mls.clone(),
                                            chain: Some(chain_msg.clone()),
                                            // We need to specify the hash of a chain before the commit was applied.
                                            // This field is a pointer to the old version of the group
                                            // (the version that should be fetch to apply the commit message to).
                                            chain_hash: chain
                                                .hash_at(epoch)
                                                .ok_or(anyhow!(
                                                    "SignatureChain is missing block at epoch={}",
                                                    epoch
                                                ))?
                                                .to_string(),
                                        },
                                    )),
                                },
                            });
                        }
                    }

                    // Forward welcome to invitees
                    if let Some(welcome) = c.welcome {
                        for key_package in &chain.last().body.ops.add {
                            let device_id = get_device_id(key_package.credential())?;
                            let mailbox = self.mailboxes.entry(device_id).or_insert(vec![]);
                            mailbox.push(MailboxEntry {
                                from_device_id: from_device_id.to_string(),
                                entry: response::mailbox::Entry {
                                    id: message.id.clone(),
                                    value: Some(response::mailbox::entry::Value::Welcome(
                                        response::SecretGroupWelcome {
                                            welcome: welcome.clone(),
                                            chain: c.chain.clone(),
                                        },
                                    )),
                                },
                            });

                            let key_ref = key_package.hash_ref(&RustCrypto::default())?;
                            self.key_packages.remove(&key_ref);
                        }
                    }
                }
                Some(request::push_mailbox::Value::Message(m)) => {
                    // Forward message to recipients
                    for member in m.to_device_ids {
                        if &member != from_device_id {
                            let mailbox = self.mailboxes.entry(member).or_insert(vec![]);
                            mailbox.push(MailboxEntry {
                                from_device_id: from_device_id.to_string(),
                                entry: response::mailbox::Entry {
                                    id: message.id.clone(),
                                    value: Some(response::mailbox::entry::Value::Message(
                                        response::SecretGroupMessage {
                                            mls: m.mls.clone(),
                                            chain_hash: m.chain_hash.clone(),
                                            chain: None,
                                        },
                                    )),
                                },
                            });
                        }
                    }
                }
                None => {}
            }

            Ok(())
        }

        fn get_mailbox(&self, device_id: &str) -> response::Mailbox {
            if let Some(mailbox) = self.mailboxes.get(device_id) {
                let skip = 0 as usize;
                let entries = mailbox
                    .iter()
                    .skip(skip)
                    .filter(|e| e.from_device_id != device_id)
                    .map(|e| e.entry.clone())
                    .collect();
                response::Mailbox { entries }
            } else {
                response::Mailbox { entries: vec![] }
            }
        }

        fn ack_mailbox_message(&mut self, device_id: &str, message_id: &str) {
            if let Some(mailbox) = self.mailboxes.get_mut(device_id) {
                mailbox.retain(|e| e.entry.id != message_id);
            }
        }

        fn push_doc(&mut self, device_id: &str, message: request::DocMessage) -> Result<()> {
            let acc_id = self
                .find_account_id(device_id)
                .ok_or(anyhow!("Device not connected to account"))?;

            for to_account in &message.to_account_ids {
                let docs = self.documents.entry(to_account.clone()).or_insert(vec![]);

                // Remove docs that this message replaces
                docs.retain(|doc| {
                    // Delete docs only from this account
                    if &acc_id != to_account {
                        return true;
                    }

                    // Keep other docs
                    if doc.doc_id != message.id {
                        return true;
                    }

                    let client_counter = message
                        .current_clock
                        .as_ref()
                        .unwrap()
                        .vector
                        .get(&doc.author_device_id)
                        .unwrap_or(&u64::MAX);
                    // If the doc is missing from the clock or server's counter is bigger then keep
                    doc.counter > *client_counter
                });

                match &message.body {
                    Some(request::doc_message::Body::Encrypted(body)) => {
                        docs.push(response::DocVersion {
                            doc_id: message.id.clone(),
                            counter: message.counter,
                            payload_signature: message.payload_signature.clone(),
                            author_device_id: device_id.to_string(),
                            created_at_sec: message.created_at_sec,
                            body: Some(response::doc_version::Body::Encrypted(
                                response::doc_version::EncryptedBody {
                                    secret_id: body.secret_id.clone(),
                                    payload: body.payload.clone(),
                                },
                            )),
                        });
                    }
                    Some(request::doc_message::Body::Deleted(body)) => {
                        docs.push(response::DocVersion {
                            doc_id: message.id.clone(),
                            counter: message.counter,
                            payload_signature: message.payload_signature.clone(),
                            author_device_id: device_id.to_string(),
                            created_at_sec: message.created_at_sec,
                            body: Some(response::doc_version::Body::Deleted(
                                response::doc_version::DeletionBody {
                                    deleted_at_sec: body.deleted_at_sec,
                                },
                            )),
                        });
                    }
                    None => {
                        bail!("Empty doc.body");
                    }
                }
            }
            Ok(())
        }

        fn fetch_docs(
            &self,
            device_id: &str,
            clock: &DeviceVectorClock,
        ) -> Result<response::AccountDocs> {
            let acc_id = self
                .find_account_id(device_id)
                .ok_or(anyhow!("Device not connected to account"))?;

            let mut res = response::AccountDocs::default();
            let Some(docs) = self.documents.get(&acc_id) else {
                return Ok(res);
            };

            for doc in docs {
                if device_id == &doc.author_device_id {
                    res.last_seen_counter = max(res.last_seen_counter, doc.counter);
                }

                match clock.vector.get(&doc.author_device_id) {
                    Some(counter) if doc.counter > *counter => {
                        // We have a newer doc version
                        res.docs.push(doc.clone());
                    }
                    Some(_) => {}
                    None => {
                        // Device doesn't know about docs from this device yet
                        res.docs.push(doc.clone());
                    }
                }
            }
            Ok(res)
        }

        fn get_account_devices(&self, account_id: &str) -> Result<response::AccountDevices> {
            // Find signature chain
            let group = self
                .groups
                .get(account_id)
                .ok_or(anyhow!("Account not found"))?;
            let members = group.chain.members(&RustCrypto::default())?;
            let device_ids = members.device_ids();

            // Find key packages for each device
            let mut package_messages = vec![];

            for package in self.key_packages.values() {
                let device_id = get_device_id(package.credential())?;
                if device_ids.contains(&(device_id.as_ref())) {
                    package_messages.push(KeyPackageMessage {
                        data: package.tls_serialize_detached()?,
                    });
                }
            }

            Ok(response::AccountDevices {
                chain: Some(group.chain.encode()?),
                key_packages: package_messages,
            })
        }

        fn get_device_packages(&self, device_id: &str) -> Result<response::DevicePackages> {
            let mut package_messages = vec![];

            for package in self.key_packages.values() {
                let id = get_device_id(package.credential())?;
                if device_id == id {
                    package_messages.push(KeyPackageMessage {
                        data: package.tls_serialize_detached()?,
                    });
                }
            }

            Ok(response::DevicePackages {
                key_packages: package_messages,
            })
        }

        fn find_account_id(&self, device_id: &str) -> Option<String> {
            let crypto = &RustCrypto::default();
            for group in self.groups.values() {
                if !group.is_account {
                    continue;
                }

                let members = group.chain.members(crypto).ok()?;
                if members.find_by_id(device_id).is_some() {
                    // Found group
                    return Some(group.chain.root().to_string());
                }
            }

            None
        }
    }

    #[derive(Clone, Default)]
    pub struct MockClient {
        pub conf: ClientConfig,
        device_id: String,
        pub data: Arc<Mutex<MockClientData>>,
    }

    impl MockClient {
        pub fn pushed_to_mailbox(&self) -> Vec<request::PushMailbox> {
            self.data.lock().unwrap().pushed_mailbox.clone()
        }

        pub fn uploaded_key_packages(&self) -> Vec<KeyPackageMessage> {
            self.data.lock().unwrap().key_packages.clone()
        }

        pub fn uploaded_docs(&self) -> Vec<request::DocMessage> {
            self.data.lock().unwrap().uploaded_docs.clone()
        }

        pub fn uploaded_blobs(&self) -> Vec<(String, Vec<u8>)> {
            let server = self.conf.mock_server.lock().unwrap();
            server.blobs.clone().into_iter().collect()
        }

        pub fn downloaded_blobs(&self) -> Vec<String> {
            self.data.lock().unwrap().downloaded_blobs.clone()
        }

        pub fn mock_blob_download(&self, blob_id: &str, bytes: Vec<u8>) {
            let mut server = self.conf.mock_server.lock().unwrap();
            server.blobs.insert(blob_id.to_string(), bytes);
        }

        pub fn clear(&self) {
            let mut data = self.data.lock().unwrap();
            *data = MockClientData::default();
        }
    }

    #[async_trait]
    impl Client for MockClient {
        type BlobStream = impl Stream<Item = reqwest::Result<Bytes>>;

        fn new(
            conf: ClientConfig,
            device_id: String,
            _signature_key: SignaturePrivateKey,
        ) -> Result<Self> {
            Ok(Self {
                conf,
                device_id,
                data: Arc::default(),
            })
        }

        async fn upload_key_package(&self, package: KeyPackageMessage) -> Result<()> {
            self.data.lock().unwrap().key_packages.push(package.clone());
            self.conf
                .mock_server
                .lock()
                .unwrap()
                .upload_key_package(package)
                .context("MockClient")?;
            Ok(())
        }

        async fn push_mailbox(&self, message: request::PushMailbox) -> Result<()> {
            self.data
                .lock()
                .unwrap()
                .pushed_mailbox
                .push(message.clone());
            self.conf
                .mock_server
                .lock()
                .unwrap()
                .push_mailbox(&self.device_id, message)
                .context("MockClient")?;
            Ok(())
        }

        async fn fetch_mailbox(&self) -> Result<response::Mailbox> {
            let mailbox = self
                .conf
                .mock_server
                .lock()
                .unwrap()
                .get_mailbox(&self.device_id);
            Ok(mailbox)
        }

        async fn ack_mailbox_message(
            &self,
            message_id: &str,
            _info: request::AckMailboxInfo,
        ) -> Result<()> {
            self.conf
                .mock_server
                .lock()
                .unwrap()
                .ack_mailbox_message(&self.device_id, message_id);
            Ok(())
        }

        async fn upload_blob(
            &self,
            blob_id: &str,
            content_len: u64,
            encrypted_blob: impl Stream<Item = Result<Vec<u8>>> + Send + Sync + 'static,
        ) -> Result<()> {
            let mut buf: Vec<u8> = vec![];
            tokio::pin!(encrypted_blob);
            while let Some(chunk) = encrypted_blob.next().await {
                let bytes = chunk?;
                buf.extend(&bytes);
            }
            if buf.len() as u64 != content_len {
                bail!("Provided content-len={} != buf={}", content_len, buf.len());
            }

            self.conf
                .mock_server
                .lock()
                .unwrap()
                .blobs
                .insert(blob_id.to_string(), buf);
            Ok(())
        }

        async fn download_blob(
            &self,
            payload: &request::PresignDownload,
        ) -> Result<(u32, Self::BlobStream)> {
            self.data
                .lock()
                .unwrap()
                .downloaded_blobs
                .push(payload.blob_id.to_string());
            let server = self.conf.mock_server.lock().unwrap();
            let value = server.blobs.get(&payload.blob_id).cloned();
            match value {
                Some(data) => {
                    let len = data.len() as u32;
                    let item: reqwest::Result<_> = Ok(Bytes::from(data));
                    let stream = tokio_stream::once(item);
                    Ok((len, stream))
                }
                None => Err(anyhow!("Blob not found")),
            }
        }

        async fn get_account_devices(&self, account_id: &str) -> Result<response::AccountDevices> {
            self.conf
                .mock_server
                .lock()
                .unwrap()
                .get_account_devices(account_id)
        }

        async fn get_device_packages(&self, device_id: &str) -> Result<response::DevicePackages> {
            self.conf
                .mock_server
                .lock()
                .unwrap()
                .get_device_packages(device_id)
        }

        async fn fetch_docs(&self, clock: &DeviceVectorClock) -> Result<response::AccountDocs> {
            let res = self
                .conf
                .mock_server
                .lock()
                .unwrap()
                .fetch_docs(&self.device_id, clock)?;
            Ok(res)
        }

        async fn get_doc_version(
            &self,
            _doc_id: &str,
            _author_device_id: &str,
        ) -> Result<response::DocVersion> {
            todo!("get_doc_version: Not implemented")
        }

        async fn push_doc(&self, message: request::DocMessage) -> Result<()> {
            self.data
                .lock()
                .unwrap()
                .uploaded_docs
                .push(message.clone());
            self.conf
                .mock_server
                .lock()
                .unwrap()
                .push_doc(&self.device_id, message)?;
            Ok(())
        }
    }
}

use axum::response::{IntoResponse, Response};
use bolik_chain::ChainError;
use bolik_migrations::{rusqlite, MigrationError};
use hyper::StatusCode;

/// Errors that happen during server setup.
#[derive(thiserror::Error, Debug)]
pub enum SetupError {
    #[error("Missing required environment variable: {0}")]
    MissingEnvVar(String),
    #[error("Invalid environment variable value: {0}")]
    InvalidEnvVar(String),
    #[error("S3 credentials: {0}")]
    S3Creds(#[from] s3::creds::error::CredentialsError),
    #[error("S3: {0}")]
    S3(#[from] s3::error::S3Error),
    #[error("DB migration: {0}")]
    Migration(#[from] MigrationError),
    #[error("Db: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("Start server: {0}")]
    StartServer(#[from] hyper::Error),
}

impl SetupError {
    pub fn missing_env_var(var: impl Into<String>) -> Self {
        Self::MissingEnvVar(var.into())
    }

    pub fn invalid_env_var(var: impl Into<String>) -> Self {
        Self::InvalidEnvVar(var.into())
    }
}

/// Authentication errors. Errors to which server responds with 401 or 403.
#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Missing device-id header")]
    MissingDeviceIdHeader,
    #[error("Missing timestamp header")]
    MissingTimestampHeader,
    #[error("Missing signature header")]
    MissingSignatureHeader,
    #[error("Bad signature")]
    BadSignature,
    #[error("Unknown device")]
    UnknownDevice,
}

/// User errors (aka client errors). Errors to which server responds with 4xx.
#[derive(thiserror::Error, Debug)]
pub enum UserError {
    #[error("Protobuf encode: {0}")]
    ProtoEncode(#[from] bolik_proto::prost::EncodeError),
    #[error("KeyPackageDecode: {0}")]
    KeyPackageDecode(tls_codec::Error),
    #[error("KeyPackageEncode: {0}")]
    KeyPackageEncode(tls_codec::Error),
    #[error("KeyPackage hash_ref: {0}")]
    KeyPackageHash(openmls::error::LibraryError),
    #[error("MlsMessageInDecode: {0}")]
    MlsMessageInDecode(tls_codec::Error),
    #[error("WelcomeDecode: {0}")]
    WelcomeDecode(tls_codec::Error),
    #[error("Credential id of KeyPackage and this device do not match")]
    KeyPackageCredMismatch,
    #[error("Device is not connected to any account")]
    NoAccount,
    #[error("Blob ID is in invalid format")]
    InvalidBlobId,
    #[error("Blob is too big")]
    BlobTooBig,
    #[error("Blob is not uploaded blob_id={blob_id} device_id={device_id}")]
    MissingBlob { blob_id: String, device_id: String },
    #[error("{0} not found")]
    NotFound(String),
    #[error("Invalid created_at_sec (out-of-range): {0}s {1}ns")]
    InvalidCreatedAt(i64, u32),
    #[error("Invalid deleted_at_sec (out-of-range): {0}s {1}ns")]
    InvalidDeletedAt(i64, u32),
    #[error("SignatureChain must have only one block")]
    WrongSignatureChainLen,
    #[error("SignatureChain not verified: {0}")]
    SignatureChainVerify(ChainError),
    #[error("Encode SignatureChain: {0}")]
    SignatureChainDecode(ChainError),
    #[error("Missing {field} field")]
    MissingField { field: String },
    #[error("Expected a handshake MLS message")]
    ExpectedHandshakeMsg,
    #[error("Expected a non-handshake MLS message")]
    ExpectedNonHandshakeMsg,
    #[error("Mls group ID doesn't match SignatureChain")]
    GroupChainMismatch,
    #[error("SignatureChain is missing block at epoch={0}")]
    SignatureChainMissingEpoch(u64),
    #[error("Decode base58: {0}")]
    Base58Decode(bs58::decode::Error),
}

/// Database errors.
#[derive(thiserror::Error, Debug)]
#[error("Db: '{query}': {source}")]
pub struct DbError {
    query: String,
    source: rusqlite::Error,
    // Future: once this lands https://github.com/rust-lang/rust/issues/99301
    // backtrace: Backtrace,
}

impl DbError {
    pub fn new(query: impl Into<String>, source: rusqlite::Error) -> Self {
        Self {
            query: query.into(),
            source,
            // backtrace: Backtrace::capture(),
        }
    }

    pub fn txn(source: rusqlite::Error) -> Self {
        Self::new("Start transaction", source)
    }

    pub fn commit(source: rusqlite::Error) -> Self {
        Self::new("Transaction commit", source)
    }
}

/// Server errors (aka internal server errors). Errors to which server responds with 5xx.
#[derive(thiserror::Error, Debug)]
pub enum ServerError {
    #[error("Malformed Credential")]
    MalformedCredential(String),
    #[error("SignatureChain: {0}")]
    SignatureChain(ChainError),
    #[error("S3: {0}")]
    S3Error(#[from] s3::error::S3Error),
    #[error("Protobuf decode: {0}")]
    ProtoDecode(#[from] bolik_proto::prost::DecodeError),
}

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    User(#[from] UserError),
    #[error(transparent)]
    Server(#[from] ServerError),
    #[error(transparent)]
    Db(#[from] DbError),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!("Error {}", self);

        let (status, error_message) = match self {
            AppError::Auth(e) => (StatusCode::UNAUTHORIZED, format!("{}", e)),
            AppError::User(ref e @ UserError::NotFound(_)) => {
                (StatusCode::NOT_FOUND, format!("{}", e))
            }
            AppError::User(e) => (StatusCode::BAD_REQUEST, format!("{}", e)),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, format!("")),
        };

        let body = error_message;
        (status, body).into_response()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum JobError {
    #[error(transparent)]
    Db(#[from] DbError),
}

/// Helper trait to convert errors into DbError with user-defined context.
/// Inspired by anyhow::Context.
pub trait DbContext<T> {
    /// Wrap the error value with additional context.
    fn db_context<C>(self, context: C) -> Result<T, DbError>
    where
        C: std::fmt::Display + Send + Sync + 'static;

    fn db_txn(self) -> Result<T, DbError>
    where
        Self: Sized,
    {
        self.db_context("txn")
    }

    fn db_commit(self) -> Result<T, DbError>
    where
        Self: Sized,
    {
        self.db_context("commit")
    }
}

impl<T> DbContext<T> for Result<T, rusqlite::Error> {
    fn db_context<C>(self, context: C) -> Result<T, DbError>
    where
        C: std::fmt::Display,
    {
        self.map_err(|err| DbError::new(format!("{}", context), err))
    }
}

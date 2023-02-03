use std::time::Duration;

use axum::{
    error_handling::HandleErrorLayer,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    BoxError, Router,
};
use bolik_migrations::rusqlite::{params, OptionalExtension};
use chrono::Utc;
use hyper::{Body, Request, StatusCode};
use openmls::prelude::{Credential, TlsDeserializeTrait};
use openmls_rust_crypto::OpenMlsRustCrypto;
use tower::{timeout::TimeoutLayer, ServiceBuilder};
use tower_http::{classify::ServerErrorsFailureClass, trace::TraceLayer};
use tracing::{Level, Span};

use crate::{
    account, blobs, device, docs,
    error::{AppError, AuthError, DbError, ServerError},
    mailbox,
    mls::read_signature,
    state::AppState,
};

pub fn router(state: AppState) -> Router {
    let state2 = state.clone();
    let api = Router::new()
        .route("/key-package", post(device::save_key_package))
        .route("/mailbox", post(mailbox::push).get(mailbox::fetch))
        .route("/mailbox/ack/:message_id", delete(mailbox::ack_message))
        .route("/docs", post(docs::save))
        .route("/docs/list", post(docs::list))
        .route("/docs/version/:id/:device_id", get(docs::get_version))
        .route("/blobs/upload", put(blobs::presign_upload))
        .route("/blobs/download", put(blobs::presign_download))
        .route("/account/:id/devices", get(account::list_devices))
        .route("/device/:id/packages", get(device::list_packages))
        .route_layer(middleware::from_fn(move |req, next| {
            auth(req, next, state2.clone())
        }));

    let app = Router::new()
        .nest("/api", api)
        .route("/", get(server_status))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|error: BoxError| async move {
                    if error.is::<tower::timeout::error::Elapsed>() {
                        Ok(StatusCode::REQUEST_TIMEOUT)
                    } else {
                        tracing::error!(%error, "failed");
                        Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Unhandled internal error: {}", error),
                        ))
                    }
                }))
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(|request: &Request<Body>| {
                            let device_id = request
                                .headers()
                                .get("device-id")
                                .and_then(|v| v.to_str().ok())
                                .map(|s| s.to_string())
                                .unwrap_or_default();
                            tracing::span!(
                                Level::INFO,
                                "request",
                                method = %request.method(),
                                uri = %request.uri(),
                                %device_id,
                            )
                        })
                        .on_request(())
                        .on_response(|response: &Response<_>, latency: Duration, _span: &Span| {
                            if response.status().is_success()
                                || response.status().is_redirection()
                                || response.status() == StatusCode::NOT_FOUND
                            {
                                tracing::trace!(
                                    status = %response.status(),
                                    latency = format_args!("{} ms", latency.as_millis()),
                                    "finished",
                                );
                            } else {
                                tracing::warn!(
                                    status = %response.status(),
                                    latency = format_args!("{} ms", latency.as_millis()),
                                    "failed",
                                );
                            };
                        })
                        .on_body_chunk(())
                        .on_eos(())
                        .on_failure(
                            |error: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                                tracing::error!(?error, "Server error");
                            },
                        ),
                )
                .layer(TimeoutLayer::new(Duration::from_secs(60))),
        );
    app
}

#[derive(Debug, Clone)]
pub struct CurrentDevice {
    pub device_id: String,
}

async fn auth<B>(
    mut req: Request<B>,
    next: Next<B>,
    app: AppState,
) -> Result<Response, StatusCode> {
    // Verify signature
    match verify_signature(&app, &req) {
        Ok(device_id) => {
            req.extensions_mut().insert(CurrentDevice { device_id });
            Ok(next.run(req).await)
        }
        Err(err) => {
            tracing::debug!("Cannot authenticate: {:?}", err);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

fn verify_signature<B>(app: &AppState, req: &Request<B>) -> Result<String, AppError> {
    let headers = req.headers();
    let device_id = req
        .headers()
        .get("device-id")
        .and_then(|header| header.to_str().ok())
        .map(|id| id.to_string())
        .ok_or(AuthError::MissingDeviceIdHeader)?;

    let timestamp = headers
        .get("timestamp")
        .and_then(|header| header.to_str().ok())
        .ok_or(AuthError::MissingTimestampHeader)?;
    let signature = headers
        .get("signature")
        .and_then(|header| header.to_str().ok())
        .and_then(|id| read_signature(id).ok())
        .ok_or(AuthError::MissingSignatureHeader)?;

    // TODO: verify timestamp is not old (parse timestamp as unix seconds)

    let req_path = format!("/api{}", req.uri().path());
    let credential_data: Option<Vec<u8>> = {
        let conn = app.conn.lock().unwrap();
        conn.query_row(
            "SELECT data FROM credentials WHERE device_id = ?",
            params![device_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| DbError::new("Find credential", err))?
    };

    match credential_data {
        Some(data) => {
            let credential = Credential::tls_deserialize(&mut data.as_slice())
                .map_err(|err| ServerError::MalformedCredential(format!("{}", err)))?;
            let mut payload = vec![];
            payload.extend(timestamp.as_bytes());
            payload.extend(req.method().as_str().as_bytes());
            payload.extend(req_path.as_bytes());
            if let Some(query) = req.uri().query() {
                payload.extend(query.as_bytes());
            }

            let backend = &OpenMlsRustCrypto::default();
            credential
                .verify(backend, payload.as_slice(), &signature)
                .map_err(|_| AuthError::BadSignature)?;
        }
        None if req_path == "/api/key-package" => {
            // We can skip this for the first request to upload a key package
        }
        None => {
            return Err(AuthError::UnknownDevice.into());
        }
    }

    Ok(device_id)
}

#[axum::debug_handler]
async fn server_status() -> impl IntoResponse {
    let now = Utc::now();
    let region = match std::env::var("FLY_REGION") {
        Ok(r) => r,
        Err(_) => "local".into(),
    };
    format!(
        "Bolik API is running in region={} with time={}",
        region, now
    )
}

use std::time::Duration;

use error::SetupError;
pub use state::AppConfig;
use state::AppState;
use tokio::signal;

use crate::{router::router, state::build_app_state};

mod account;
mod blobs;
mod device;
mod docs;
pub mod error;
mod mailbox;
mod migration;
mod mls;
pub mod router;
pub mod state;

pub use mls::get_device_id;

pub async fn run() -> Result<(), SetupError> {
    let conf = AppConfig::from_env()?;
    let addr = conf.addr;
    let state = build_app_state(conf).await?;
    let app = router(state.clone());

    tokio::spawn(async move {
        batch_jobs_task(state).await;
    });

    tracing::info!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn batch_jobs_task(state: AppState) {
    tokio::time::sleep(Duration::from_secs(30)).await;
    loop {
        tracing::info!("Running batch jobs");

        if let Err(err) = state.mark_unused_blobs() {
            tracing::warn!("Cannot mark unused blobs: {}", err);
        }

        match state.cleanup_blobs(None).await {
            Ok(info) => {
                tracing::info!("Cleanup info: {:?}", info);
            }
            Err(err) => {
                tracing::warn!("Cannot cleanup blobs: {}", err);
            }
        }

        tokio::time::sleep(Duration::from_secs(15 * 60)).await;
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Signal received, starting graceful shutdown");
}

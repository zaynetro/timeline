use std::env;

use bolik_server::run;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    if env::var_os("RUST_LOG").is_none() {
        // Set `RUST_LOG=backend=debug` to see debug logs,
        // this only shows access logs.
        env::set_var("RUST_LOG", "info,bolik_server=debug,tower_http=info");
    }
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    if let Err(err) = run().await {
        tracing::error!("{:?}", err);
        std::process::exit(1);
    }
}

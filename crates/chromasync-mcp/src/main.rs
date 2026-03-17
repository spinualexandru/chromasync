use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::{EnvFilter, fmt};

use chromasync_mcp::ChromasyncServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting chromasync-mcp server");

    let service = ChromasyncServer::new().serve(stdio()).await?;

    service.waiting().await?;

    Ok(())
}

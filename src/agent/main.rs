use mimalloc::MiMalloc;
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use tracing_subscriber::EnvFilter;

mod types;
mod agents;
mod config;
mod controller;
mod cache;
mod utils;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::new("info")).init();
    tracing::info!("WEB3-DESTROYER INITIALIZING...");
    let ctrl = controller::Controller::new().await?;
    ctrl.run().await?;
    Ok(())
}  
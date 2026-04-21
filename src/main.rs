// UPGRADE: Force the entire binary to use mimalloc for a 15% speed boost
use mimalloc::MiMalloc;
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

// UPGRADE: Initialize the elite tracing engine
use tracing_subscriber::EnvFilter;

mod types;
mod agents;
mod config;
mod controller;
mod cache;
mod utils;

#[tokio::main]
async fn main() {
    // Initialize tracing (replaces println!)
    // You can control log level by setting RUST_LOG=debug in terminal
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("web3_destroyer=info".parse().unwrap()))
        .init();

    tracing::info!("🛑 WEB3-DESTROYER INITIALIZING...");
    tracing::info!("⚙️  Mimalloc active. Tracing online. Booting low-level execution engine...");

    let ctrl = match controller::Controller::new().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "FATAL: Failed to initialize Controller");
            std::process::exit(1);
        }
    };

    if let Err(e) = ctrl.run().await {
        tracing::error!(error = %e, "FATAL: Controller crashed");
    }
}

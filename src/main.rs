use mimalloc::MiMalloc;
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use tracing_subscriber::EnvFilter;
use std::env;

mod types;
mod agents;
mod config;
mod controller;
mod utils;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // 1. Setup logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("info,web3_destroyer=debug"))
        .init();

    // 2. Load environment variables
    // For Base Live Hunt, set this to: https://base.org
    let rpc_url = env::var("RPC_URL")
        .unwrap_or_else(|_| "http://localhost:8545".to_string());
    
    // If you have a WebSocket URL (for real-time hunting)
    let ws_url = env::var("WS_URL").ok();

    tracing::info!("🌪️ GHOST HUNTER INITIALIZING...");
    tracing::info!("📡 Connection: {}", rpc_url);

    let ctrl = controller::Controller::new(&rpc_url).await?;

    if let Some(ws) = ws_url {
        tracing::info!("🎧 Mode: WebSocket Live Stream (Base/Bera)");
        // ctrl.run_websocket(&ws).await?; // You can implement this next
    } else {
        tracing::info!("🎯 Mode: Target Hunt (targets.json)");
        // This runs the logic we just fixed (Proxy -> Source -> Scan)
        if let Err(e) = ctrl.run_live("targets.json").await {
            tracing::error!("❌ Hunt stopped: {:?}", e);
        }
    }

    Ok(())
}


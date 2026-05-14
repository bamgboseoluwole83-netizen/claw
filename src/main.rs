use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber;

mod agents;
mod report;

#[derive(Parser)]
#[command(name = "web3-destroyer")]
enum Args {
    /// Run single scan on a target contract
    Scan {
        /// Target contract address
        target: String,

        /// Directory containing target Solidity source code
        #[arg(long)]
        source_dir: Option<PathBuf>,

        /// Path to Foray source code (main.py directory)
        #[arg(long)]
        foray: Option<PathBuf>,

        /// Oracle/proxy contract address (override auto-discovery)
        #[arg(long)]
        proxy: Option<String>,

        /// Block number to pin fork state for PoC reproduction
        #[arg(long)]
        block_number: Option<u64>,

        /// Scan depth: quick, standard (default), or deep
        #[arg(long, default_value = "standard")]
        mode: String,
    },
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        )
        .init();

    let args = Args::parse();
    match args {
        Args::Scan {
            target,
            source_dir,
            foray,
            proxy,
            block_number,
            mode,
        } => {
            run_scan_mode(&target, source_dir, foray, proxy, block_number, &mode).await?;
        }
    }

    Ok(())
}

async fn run_scan_mode(
    target: &str,
    source_dir: Option<PathBuf>,
    foray_path: Option<PathBuf>,
    proxy_override: Option<String>,
    block_number: Option<u64>,
    mode: &str,
) -> eyre::Result<()> {
    let proxy_str = proxy_override.as_deref();
    let rpc_url =
        std::env::var("DRPC_URL").unwrap_or_else(|_| "http://127.0.0.1:8545".to_string());

    let target_address: alloy_primitives::Address = target
        .parse()
        .map_err(|e| eyre::eyre!("Invalid target address '{}': {}", target, e))?;

    let proxy_address: Option<alloy_primitives::Address> = proxy_str
        .and_then(|s| s.parse::<alloy_primitives::Address>().ok());

    let scan_mode: agents::ScanMode = mode
        .parse()
        .map_err(|e| eyre::eyre!("Invalid --mode: {}", e))?;

    tracing::info!(" Scan mode: {}", scan_mode);

    let mut controller = agents::controller::Controller::new(rpc_url, scan_mode);
    if let Some(dir) = source_dir {
        controller = controller.with_source_dir(dir);
    }
    if let Some(fp) = foray_path {
        controller = controller.with_foray_path(fp);
    }
    if let Some(pa) = proxy_address {
        controller = controller.with_proxy(pa);
    }
    if let Some(bn) = block_number {
        controller = controller.with_block_number(bn);
    }
    controller.add_target(target_address);
    let _summary = controller.run_pipeline().await;

    Ok(())
}

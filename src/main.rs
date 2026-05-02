use tracing_subscriber;

mod agents;
mod controller;
mod types;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let rpc_url = std::env::var("DRPC_URL")
        .unwrap_or_else(|_| "https://lb.drpc.live/ethereum/your_default_key".to_string());
    let etherscan_key = std::env::var("ETHERSCAN_KEY").ok();

    let ctrl = controller::Controller::new(&rpc_url, etherscan_key).await?;
    ctrl.run_live("targets.json").await?;

    Ok(())
}

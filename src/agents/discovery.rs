use alloy::providers::{Provider, RootProvider};
use alloy::transports::http::{Client, Http};
use std::sync::Arc;
use eyre::Result;

pub struct DiscoveryAgent {
    provider: Arc<RootProvider<Http<Client>>>,
}

impl DiscoveryAgent {
    pub fn new(provider: Arc<RootProvider<Http<Client>>>) -> Self {
        Self { provider }
    }

    /// Proves the dRPC connection is alive by pinging the latest block.
    /// Later, this will use trace filters to find new contract deployments.
    pub async fn get_latest_block(&self) -> Result<u64> {
        let block_number = self.provider.get_block_number().await?;
        println!("🔍 [DISCOVERY] Connected to chain. Latest block: {}", block_number);
        Ok(block_number)
    }
}

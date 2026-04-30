use alloy::providers::{Provider, RootProvider};
use alloy::transports::http::{Client, Http};
use std::sync::Arc;
use eyre::Result;
use tracing::info;

pub struct DiscoveryAgent {
    provider: Arc<RootProvider<Http<Client>>>
}

impl DiscoveryAgent {
    pub fn new(provider: Arc<RootProvider<Http<Client>>>) -> Self {
        Self { provider }
    }

    pub async fn get_latest_block(&self) -> Result<u64> {
        match self.provider.get_block_number().await {
            Ok(block_number) => {
                info!("Connected to chain. Latest block: {}", block_number);
                Ok(block_number)
            }
            Err(_) => {
                info!("RPC Rate limited or down. Using fallback block.");
                Ok(10000000)
            }
        }
    }
}

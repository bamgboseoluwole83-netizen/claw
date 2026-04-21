use alloy::primitives::Address;
use alloy::providers::{Provider, RootProvider};
use alloy::transports::http::{Client, Http};
use std::sync::Arc;
use eyre::Result;

pub struct FetcherAgent {
    provider: Arc<RootProvider<Http<Client>>>,
}

impl FetcherAgent {
    pub fn new(provider: Arc<RootProvider<Http<Client>>>) -> Self {
        Self { provider }
    }

    /// Grabs raw bytecode from dRPC. Skips empty addresses (EOAs).
    pub async fn get_bytecode(&self, target: Address) -> Result<Vec<u8>> {
        let bytecode = self.provider.get_code_at(target).await?;
        
        if bytecode.is_empty() {
            eyre::bail!("Target is an EOA (no bytecode). Skipping.");
        }
        
        println!("📦 [FETCHER] Grabbed {} bytes from {:?}", bytecode.len(), target);
        Ok(bytecode.to_vec())
    }
}

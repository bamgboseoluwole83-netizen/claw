use alloy::providers::{Provider, RootProvider};
use alloy::transports::http::Http;
use reqwest::Client;
use alloy_primitives::{Address, U256};
use eyre::Result;
use std::sync::Arc;

pub type HttpProvider = RootProvider<Http<Client>>;

pub struct Fetcher {
    pub provider: Arc<HttpProvider>,
}

impl Fetcher {
    pub fn new(provider: Arc<HttpProvider>) -> Self {
        Self { provider }
    }

    pub async fn get_code(&self, address: Address) -> Result<Vec<u8>> {
        let code = self.provider.get_code_at(address).await?;
        Ok(code.to_vec())
    }

    pub async fn get_storage_at(&self, address: Address, slot: U256) -> Result<U256> {
        let value = self.provider.get_storage_at(address, slot).await?;
        Ok(value)
    }
}

use alloy::providers::{Provider, RootProvider};
use alloy::transports::http::{Client, Http};
use alloy::rpc::types::EIP1186ProofResponse;
use alloy_primitives::{Address, U256};
use eyre::Result;
use std::sync::Arc;

pub type HttpProvider = RootProvider<Http<Client>>;

pub struct Fetcher {
    provider: Arc<HttpProvider>,
}

impl Fetcher {
    pub fn new(provider: Arc<HttpProvider>) -> Self {
        Self { provider }
    }

    /// Fetch a lightweight state witness (balance + chosen slots).
    pub async fn fetch_state_witness(
        &self,
        target: Address,
        slots: Vec<U256>,
    ) -> Result<EIP1186ProofResponse> {
        let proof = self.provider.get_proof(target, slots).await?;
        Ok(proof)
    }
}
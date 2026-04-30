use alloy::providers::{Provider, RootProvider};
use alloy::transports::http::{Client, Http};
use alloy_primitives::{Address, U256, B256};
use alloy::rpc::types::EIP1186AccountProofResponse;
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

    /// Returns the **raw implementation bytecode** at a given address
    pub async fn get_code_raw(&self, target: Address) -> Result<Vec<u8>> {
        let code = self.provider.get_code_at(target).await?;
        Ok(code.to_vec())
    }

    /// Returns the **real implementation bytecode**, resolving proxies automatically.
    pub async fn get_code(&self, target: Address) -> Result<Vec<u8>> {
        let impl_address = self.resolve_proxy_advanced(target).await?;
        self.get_code_raw(impl_address).await
    }

    /// Resolves proxies, returning the implementation address.
    /// Falls back to the original address if no proxy is detected.
    pub async fn resolve_proxy_advanced(&self, target: Address) -> Result<Address> {
        let addr_str = format!("{:?}", target);
        if let Some(impl_addr) = crate::agents::proxy_resolver::resolve_proxy_heimdall(&addr_str) {
            return Ok(impl_addr);
        }
        Ok(target) // not a proxy
    }

    /// Fetch storage proofs for specific slots.
    pub async fn get_storage_proof(
        &self,
        target: Address,
        slots: Vec<U256>,
    ) -> Result<EIP1186AccountProofResponse> {
        let b256_slots: Vec<B256> = slots
            .into_iter()
            .map(|s| B256::from_slice(&s.to_be_bytes::<32>()))
            .collect();
        let proof = self.provider.get_proof(target, b256_slots).await?;
        Ok(proof)
    }

    /// Read a single storage slot from the chain.
    pub async fn get_storage_at(&self, target: Address, slot: U256) -> Result<U256> {
        let value = self.provider.get_storage_at(target, slot).await?;
        Ok(value)
    }
}

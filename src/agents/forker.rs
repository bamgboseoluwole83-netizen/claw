use alloy::providers::Provider;
use revm::db::{CacheDB, EmptyDB};
use revm::primitives::{AccountInfo, Bytecode, U256 as RU256};
use alloy_primitives::Address;
use eyre::Result;
use std::sync::Arc;
use crate::agents::fetcher::HttpProvider;

pub struct ForkerAgent {
    provider: Arc<HttpProvider>,
}

impl ForkerAgent {
    pub fn new(provider: Arc<HttpProvider>) -> Self {
        Self { provider }
    }

    pub async fn build_cross_contract_db(&self, target: Address, _dependencies: Vec<Address>) -> Result<CacheDB<EmptyDB>> {
        tracing::info!(target: "forker", "⚡ BUILDING CROSS-CONTRACT SANDBOX...");
        let mut db = CacheDB::new(EmptyDB::new());

        let mut real_bytecode = None;
        for _ in 0..5 {
            match self.provider.get_code_at(target).await {
                Ok(code) => { real_bytecode = Some(code); break; }
                Err(e) => {
                    tracing::warn!(target: "forker", "RPC hiccup for {:?}: {}. Retrying...", target, e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
        let real_bytecode = real_bytecode.ok_or_else(|| eyre::eyre!("Failed to fetch bytecode after 5 tries"))?;
        if !real_bytecode.is_empty() {
            db.insert_account_info(target, AccountInfo {
                balance: RU256::from(10_000_000_000_000_000_000u128),
                nonce: 1,
                code: Some(Bytecode::new_raw(real_bytecode.into())),
                code_hash: Default::default(),
            });
        }
        tracing::info!(target: "forker", "✅ Sandbox ready.");
        Ok(db)
    }
}

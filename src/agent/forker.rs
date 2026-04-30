use alloy::providers::Provider;
use revm::db::{CacheDB, EmptyDB};
use revm::primitives::{AccountInfo, Bytecode, U256 as RU256};
use alloy_primitives::Address;
use eyre::Result;
use std::sync::Arc;
use crate::controller::HttpProvider;

pub struct ForkerAgent {
    provider: Arc<HttpProvider>,
}

impl ForkerAgent {
    pub fn new(provider: Arc<HttpProvider>) -> Self {
        Self { provider }
    }

    pub async fn build_cross_contract_db(&self, target: Address, dependencies: Vec<Address>) -> Result<CacheDB<EmptyDB>> {
        tracing::info!(target: "forker", "⚡ BUILDING CROSS-CONTRACT SANDBOX...");
        let mut db = CacheDB::new(EmptyDB::new());

        tracing::info!(target: "forker", "Loading target + {} dependencies...", dependencies.len());

        self.inject_real_logic(&mut db, target, RU256::from(10_000_000_000_000_000_000u128)).await?;
        
        for dep in &dependencies {
            self.inject_real_logic(&mut db, *dep, RU256::from(1_000_000_000_000_000_000_000_000u128)).await?;
        }

        tracing::info!(target: "forker", "✅ Sandbox ready.");
        Ok(db)
    }

    async fn inject_real_logic(&self, db: &mut CacheDB<EmptyDB>, addr: Address, fake_balance: RU256) -> Result<()> {
        // RESILIENCE: Retry loop for flaky RPCs
        let mut real_bytecode = None;
        for _ in 0..3 {
            match self.provider.get_code_at(addr).await {
                Ok(code) => {
                    real_bytecode = Some(code);
                    break;
                }
                Err(e) => {
                    tracing::warn!(target: "forker", "RPC hiccup for {:?}: {}. Retrying...", addr, e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }

        let real_bytecode = real_bytecode.ok_or_else(|| eyre::eyre!("Failed to fetch bytecode from RPC after 3 tries for {:?}", addr))?;
        
        if real_bytecode.is_empty() {
            tracing::warn!(target: "forker", "Skipping {:?}: No code (EOA)", addr);
            return Ok(());
        }

        db.insert_account_info(addr, AccountInfo {
            balance: fake_balance, 
            nonce: 1,
            code: Some(Bytecode::new_raw(real_bytecode.into())),
            code_hash: Default::default(),
        });

        let fake_caller: Address = Address::from([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xdE, 0xaD]);
        
        let mut hasher = blake3::Hasher::new();
        hasher.update(&fake_caller.0[..]); 
        hasher.update(&[0u8; 32]);         
        let balance_slot_hash = hasher.finalize();
        
        let balance_slot_u256 = RU256::from_be_bytes(*balance_slot_hash.as_bytes());
        db.insert_account_storage(addr, balance_slot_u256, RU256::from(1_000_000_000_000_000_000_000_000u128))?;

        Ok(())
    }
}
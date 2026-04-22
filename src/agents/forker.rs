use alloy_primitives::{Address, B256, U256};
use alloy::providers::{Provider, RootProvider};
use alloy::transports::http::{Client, Http};
use revm::db::{CacheDB, EmptyDB};
use revm::primitives::{AccountInfo, Bytecode};
use std::sync::Arc;
use crate::agents::cross_ghost_reentrancy::ChaosDatabase;
use eyre::Result;
use tracing::info;

pub struct ForkerAgent {
    provider: Arc<RootProvider<Http<Client>>>,
}

impl ForkerAgent {
    pub fn new(provider: Arc<RootProvider<Http<Client>>>) -> Self {
        Self { provider }
    }

    pub async fn fork_and_wrap_chaos(
        &self,
        target_address: Address,
        caller_address: Address, // NEW: We need to know who is calling
        oracle_slot: U256,
        chaotic_price: U256,
    ) -> Result<ChaosDatabase<CacheDB<EmptyDB>>> {
        info!(target: "forker", "🌐 Forking Mainnet state...");

        let bytecode = self.provider.get_code_at(target_address).await?;
        let mut db = CacheDB::new(EmptyDB::new());
        
        db.insert_account_info(target_address, AccountInfo {
            balance: U256::ZERO,
            nonce: 0,
            code: Some(Bytecode::new_raw(bytecode.into())),
            code_hash: B256::ZERO,
        });

        // NEW: Give our caller 1 ETH so the transaction doesn't fail instantly
        db.insert_account_info(caller_address, AccountInfo {
            balance: U256::from(10u128.pow(18)), // 1 ETH
            nonce: 1,
            code: None,
            code_hash: B256::ZERO,
        });

        info!(target: "forker", "✅ State downloaded & Caller funded.");

        Ok(ChaosDatabase { inner: db, oracle_slot, chaotic_price })
    }
}

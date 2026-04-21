use alloy::primitives::{Address, B256, U256};
use alloy::providers::{Provider, RootProvider};
use alloy::transports::http::{Client, Http};
use revm::db::{CacheDB, EmptyDB};
use revm::primitives::{AccountInfo, Bytecode};
use std::sync::Arc;
use crate::agents::cross_ghost_reentrancy::ChaosDatabase;
use eyre::Result;
use tracing::info;

/// Bridges the gap between Async Alloy and Sync REVM.
pub struct ForkerAgent {
    provider: Arc<RootProvider<Http<Client>>>,
}

impl ForkerAgent {
    pub fn new(provider: Arc<RootProvider<Http<Client>>>) -> Self {
        Self { provider }
    }

    /// Downloads a contract's state from Mainnet and packages it into our ChaosDatabase
    pub async fn fork_and_wrap_chaos(
        &self,
        target_address: Address,
        oracle_slot: U256,
        chaotic_price: U256,
    ) -> Result<ChaosDatabase<CacheDB<EmptyDB>>> {
        info!(target: "forker", "🌐 Forking Mainnet state for {:?}...", target_address);

        // 1. Fetch live data from dRPC via Async Alloy
        let bytecode = self.provider.get_code_at(target_address).await?;
        let balance = self.provider.get_balance(target_address).await?;
        
        // Note: In a full implementation, you'd also fetch specific storage slots here 
        // using eth_getStorageAt. For now, we prove the wrapper works with the base state.

        // 2. Build the Sync REVM Database
        let mut db = CacheDB::new(EmptyDB::new());
        
        let account_info = AccountInfo {
            balance,
            nonce: 0, // We don't need nonce for static analysis
            code: Some(Bytecode::new_raw(bytecode.into())),
            code_hash: B256::ZERO, // Auto-calculated by REVM if needed
        };

        // Insert the live state into REVM's memory
        db.insert_account_info(target_address, account_info);

        info!(target: "forker", "✅ State downloaded. Wrapping in ChaosDatabase...");

        // 3. Wrap it in our God-Tier interceptor
        let chaos_db = ChaosDatabase {
            inner: db,
            oracle_slot,
            chaotic_price,
        };

        Ok(chaos_db)
    }
}

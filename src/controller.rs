use crate::agents::{discovery::DiscoveryAgent, executor::ExecutorAgent};
use crate::agents::cross_ghost_reentrancy::{DivergenceEngine, ChaosDatabase};
use crate::cache::DestroyerCache;
use crate::config::load_config;
use crate::types::DestroyerConfig;
use alloy::primitives::{Address, U256};


use alloy::providers::{Provider, ProviderBuilder, RootProvider};
use alloy::transports::http::{Client, Http};
use revm::db::{CacheDB, EmptyDB};
use revm::primitives::{AccountInfo, Bytecode};
use eyre::Result;
use std::sync::Arc;

type HttpProvider = RootProvider<Http<Client>>;

pub struct Controller {
    config: DestroyerConfig,
    provider: Arc<HttpProvider>,
    cache: Arc<DestroyerCache>,
}

impl Controller {
    pub async fn new() -> Result<Self> {
        let config = load_config();
        let cache = Arc::new(DestroyerCache::new("destroyer.redb")?);
        let provider = Arc::new(ProviderBuilder::new().on_http(config.drpc_url.parse()?));
        Ok(Self { config, provider, cache })
    }

    pub async fn run(&self) -> Result<()> {
        tracing::info!(">> Controller online. Phase 7: Real-World Hunter active.");
        let discovery = DiscoveryAgent::new(self.provider.clone());
        let fake_caller: Address = "0x000000000000000000000000000000000000dEaD".parse()?;
        
        // TARGET: A known ERC4626 Vault (e.g., a real yield aggregator)
        // We will use a placeholder for now, we swap to a real one next step
        let target: Address = "0xBA12222222228d8Ba445958a75a0704d566BF2C8".parse()?; // Balancer Vault
        
        loop {
            tracing::info!("--- [REAL WORLD SCAN] ---");
            let _block = discovery.get_latest_block().await?;
            
            let bytecode = self.provider.get_code_at(target).await?.to_vec();
            if bytecode.is_empty() { continue; }
            
            // THE BLIND INTERROGATOR: Auto-generate calldata for standard functions
            // ERC4626 `deposit(address, uint256)` selector
            let deposit_selector: [u8; 4] = [0x6e, 0x55, 0xc4, 0x9d];
            let mut calldata = deposit_selector.to_vec();
            
            // Append fake arguments: asset address (32 bytes) + amount (32 bytes)
            calldata.extend_from_slice(&fake_caller.as_slice());
            calldata.extend_from_slice(&U256::from(10u128.pow(18)).to_be_bytes::<32>());
            
            let mut honest_db = CacheDB::new(EmptyDB::new());
            honest_db.insert_account_info(target, AccountInfo { balance: U256::ZERO, nonce: 1, code: Some(Bytecode::new_raw(bytecode.clone().into())), code_hash: Default::default() });
            honest_db.insert_account_info(fake_caller, AccountInfo { balance: U256::from(10u128.pow(19)), nonce: 1, code: None, code_hash: Default::default() });
            let honest_state = ExecutorAgent::execute(ChaosDatabase { inner: honest_db, flip_mask: U256::ZERO }, fake_caller, target, calldata.clone(), U256::ZERO);
            
            let vectors = vec![U256::from(0xFF), U256::from(0xFFFF), U256::from(0xFFFF_FFFF_u64)];

            for mask in vectors {
                let mut chaos_db = CacheDB::new(EmptyDB::new());
                chaos_db.insert_account_info(target, AccountInfo { balance: U256::ZERO, nonce: 1, code: Some(Bytecode::new_raw(bytecode.clone().into())), code_hash: Default::default() });
                chaos_db.insert_account_info(fake_caller, AccountInfo { balance: U256::from(10u128.pow(19)), nonce: 1, code: None, code_hash: Default::default() });
                
                let chaotic_state = ExecutorAgent::execute(ChaosDatabase { inner: chaos_db, flip_mask: mask }, fake_caller, target, calldata.clone(), U256::ZERO);
                
                let report = DivergenceEngine::diff_states(&honest_state, &chaotic_state);
                
                if !report.divergent_slots.is_empty() {
                    tracing::error!(target: "dss", mask = %mask, total_dust = %report.total_dust_wei, ">> REAL-WORLD PRECISION BUG DETECTED!");
                } else {
                    tracing::info!(target: "dss", mask = %mask, ">> Clean.");
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    }
}

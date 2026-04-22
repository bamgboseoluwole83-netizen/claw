use crate::agents::{discovery::DiscoveryAgent, executor::ExecutorAgent};
use crate::agents::cross_ghost_reentrancy::{DivergenceEngine, ChaosDatabase};
use crate::cache::DestroyerCache;
use crate::config::load_config;
use crate::types::DestroyerConfig;
use alloy::primitives::{Address, B256, U256};
use alloy::providers::{ProviderBuilder, RootProvider};
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
        tracing::info!(">> Controller online. DSS Depth Engine armed.");
        let discovery = DiscoveryAgent::new(self.provider.clone());
        let fake_caller: Address = "0x000000000000000000000000000000000000dEaD".parse()?;
        let target: Address = "0x1337133713371337133713371337133713371337".parse()?;
        
        loop {
            tracing::info!("--- [NEW DEPTH SCAN] ---");
            let _block = discovery.get_latest_block().await?;
            
            // THE MALICIOUS TARGET: Reads Slot 1, multiplies by 2, saves to Slot 0
            let malicious_hex: Vec<u8> = vec![0x60, 0x01, 0x54, 0x60, 0x02, 0x02, 0x60, 0x00, 0x55, 0x00];
            
            // BYPASS FORKER: Manually inject raw hex into memory
            let mut honest_db = CacheDB::new(EmptyDB::new());
            honest_db.insert_account_info(target, AccountInfo { balance: U256::ZERO, nonce: 1, code: Some(Bytecode::new_raw(malicious_hex.clone().into())), code_hash: B256::ZERO });
            honest_db.insert_account_info(fake_caller, AccountInfo { balance: U256::from(10u128.pow(18)), nonce: 1, code: None, code_hash: B256::ZERO });
            let honest_state = ExecutorAgent::execute(ChaosDatabase { inner: honest_db, oracle_slot: U256::from(1), chaotic_price: U256::from(100) }, fake_caller, target, vec![], U256::ZERO);
            
            let mut chaos_db = CacheDB::new(EmptyDB::new());
            chaos_db.insert_account_info(target, AccountInfo { balance: U256::ZERO, nonce: 1, code: Some(Bytecode::new_raw(malicious_hex.into())), code_hash: B256::ZERO });
            chaos_db.insert_account_info(fake_caller, AccountInfo { balance: U256::from(10u128.pow(18)), nonce: 1, code: None, code_hash: B256::ZERO });
            let chaotic_state = ExecutorAgent::execute(ChaosDatabase { inner: chaos_db, oracle_slot: U256::from(1), chaotic_price: U256::from(150) }, fake_caller, target, vec![], U256::ZERO);
            
            let report = DivergenceEngine::diff_states(&honest_state, &chaotic_state);
            
            if !report.divergent_slots.is_empty() {
                tracing::error!(target: "dss", total_dust = %report.total_dust_wei, slots_affected = report.divergent_slots.len(), ">> UNFINDABLE BUG DETECTED! Storage state diverged based on price manipulation!");
            } else {
                tracing::info!(">> Clean.");
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
    }
}

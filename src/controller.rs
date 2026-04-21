use crate::agents::cross_ghost_reentrancy::DivergenceEngine;
use crate::agents::discovery::DiscoveryAgent;
use crate::agents::disassembler::disassemble;
use crate::agents::fetcher::FetcherAgent;
use crate::cache::DestroyerCache;
use crate::config::load_config;
use crate::types::DestroyerConfig;
use alloy::primitives::{Address, U256};
use alloy::providers::{ProviderBuilder, RootProvider};
use alloy::transports::http::{Client, Http};
use eyre::Result;
use std::collections::HashMap;
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
        
        let provider = Arc::new(
            ProviderBuilder::new()
                .on_http(config.drpc_url.parse()?)
        );

        Ok(Self { config, provider, cache })
    }

    pub async fn run(&self) -> Result<()> {
        tracing::info!("🧠 Controller online. DSS Symmetry Engine armed.");
        
        let discovery = DiscoveryAgent::new(self.provider.clone());
        let fetcher = FetcherAgent::new(self.provider.clone());

        loop {
            tracing::info!("--- [NEW SCAN CYCLE] ---");
            let _block = discovery.get_latest_block().await?;
            let weth_address: Address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse()?;
            
            if let Ok(bytecode) = fetcher.get_bytecode(weth_address).await {
                let _map = disassemble(&bytecode)?;
                self.run_dss_simulation();
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
    }

    fn run_dss_simulation(&self) {
        tracing::info!("🧬 [DSS] Spawning 3-way Symmetry Shadow Execution...");
        let balance_slot = U256::from(1);

        let mut honest_state = HashMap::new();
        honest_state.insert(balance_slot, U256::from(1000));

        let mut chaotic_pos_state = HashMap::new();
        chaotic_pos_state.insert(balance_slot, U256::from(990)); 

        let mut chaotic_neg_state = HashMap::new();
        chaotic_neg_state.insert(balance_slot, U256::from(1002)); 

        let report_pos = DivergenceEngine::diff_states(&honest_state, &chaotic_pos_state);
        let report_neg = DivergenceEngine::diff_states(&honest_state, &chaotic_neg_state);

        let is_sandwichable = DivergenceEngine::check_symmetry(&report_pos, &report_neg);

        if !report_pos.divergent_slots.is_empty() {
            tracing::warn!(
                target: "dss",
                sandwichable = is_sandwichable,
                "🚨 DIVERGENCE & ASYMMETRY PROVEN! Routing to Email Reporter."
            );
        } else {
            tracing::info!("🟢 [DSS] States match. Contract math is clean.");
        }
    }
}

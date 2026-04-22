use crate::agents::{discovery::DiscoveryAgent, fetcher::FetcherAgent, forker::ForkerAgent, executor::ExecutorAgent};
use crate::cache::DestroyerCache;
use crate::config::load_config;
use crate::types::DestroyerConfig;
use alloy::primitives::{Address, U256};
use alloy::providers::{ProviderBuilder, RootProvider};
use alloy::transports::http::{Client, Http};
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
        tracing::info!("🧠 Controller online. CPU & Forker armed.");
        
        let discovery = DiscoveryAgent::new(self.provider.clone());
        let fetcher = FetcherAgent::new(self.provider.clone());
        let forker = ForkerAgent::new(self.provider.clone());

        loop {
            tracing::info!("--- [NEW SCAN CYCLE] ---");
            let _block = discovery.get_latest_block().await?;
            let weth: Address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse()?;
            let fake_caller: Address = "0x000000000000000000000000000000000000dEaD".parse()?;
            
            if let Ok(_bytecode) = fetcher.get_bytecode(weth).await {
                let calldata = vec![0xd0, 0xe3, 0x0d, 0xb0]; 
                let chaos_db = forker.fork_and_wrap_chaos(weth, fake_caller, U256::from(42), U256::from(999)).await.unwrap();
                
                let chaotic_state = ExecutorAgent::execute(chaos_db, fake_caller, weth, calldata, U256::from(10u128.pow(17)));
                
                tracing::warn!(target: "dss", "🧬 Extracted {} state changes to diff!", chaotic_state.len());
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
    }
}  
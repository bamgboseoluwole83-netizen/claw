use crate::agents::discovery::DiscoveryAgent;
use crate::agents::disassembler::disassemble;
use crate::agents::fetcher::FetcherAgent;
use crate::agents::forker::ForkerAgent;
use crate::cache::DestroyerCache;
use crate::config::load_config;
use crate::types::DestroyerConfig;
use alloy::primitives::{Address, U256};
use alloy::providers::{ProviderBuilder, RootProvider};
use alloy::transports::http::{Client, Http};
use eyre::Result;
use std::sync::Arc;
use revm::Database;

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
        tracing::info!("🧠 Controller online. Forker & DSS armed.");
        
        let discovery = DiscoveryAgent::new(self.provider.clone());
        let fetcher = FetcherAgent::new(self.provider.clone());
        let forker = ForkerAgent::new(self.provider.clone());

        loop {
            tracing::info!("--- [NEW SCAN CYCLE] ---");
            let _block = discovery.get_latest_block().await?;
            let weth_address: Address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse()?;
            
            if let Ok(bytecode) = fetcher.get_bytecode(weth_address).await {
                let _map = disassemble(&bytecode)?;
                self.test_live_chaos_fork(&forker, weth_address).await;
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
    }

    async fn test_live_chaos_fork(&self, forker: &ForkerAgent, target: Address) {
        let fake_slot = U256::from(42);
        let fake_price = U256::from(999999);

        let mut chaos_db = match forker.fork_and_wrap_chaos(target, fake_slot, fake_price).await {
            Ok(db) => db,
            Err(e) => {
                tracing::error!(error = %e, "Failed to fork");
                return;
            }
        };

        match chaos_db.storage(target, fake_slot) {
            Ok(value) => {
                if value == fake_price {
                    tracing::warn!(target: "dss", injected_value = %value, "🚨 LIVE FORK HIJACK SUCCESSFUL!");
                } else {
                    tracing::error!("Hijack failed.");
                }
            }
            Err(e) => tracing::error!("REVM DB error: {:?}", e),
        }
    }
} 
use alloy_primitives::Address;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct DestroyerConfig {
    pub drpc_url: String,
    pub flashbots_relay_url: String,
    pub private_key: String,
    pub balancer_vault: Address,
    pub real_targets: Vec<Address>
}

impl DestroyerConfig {
    pub fn load_from_env() -> Self {
        let pk = std::env::var("PRIVATE_KEY").unwrap_or_else(|_| "0x0000000000000000000000000000000000000000001".to_string());
        DestroyerConfig {
            drpc_url: std::env::var("DRPC_URL").unwrap_or_else(|_| "https://lb.drpc.live/ethereum/AgJp_vcoLE0_mJ2tqAApYT2H_YaUOJ4R8ZXQtiKh6MJI".to_string()),
            flashbots_relay_url: std::env::var("FLASHBOTS_URL").unwrap_or_else(|_| "https://relay.flashbots.net".to_string()),
            private_key: pk,
            balancer_vault: "0xBA12222222228d8Ba445958a75a0704d566BF2C8".parse().unwrap(),
            real_targets: vec![
                "0xC02aaA39b223FE8D0a0e5C4F27ead9083C756Cc2".parse().unwrap(), 
                "0x7d2b36b32a80e3b46ff8e1c7808005102dfc31d0".parse().unwrap(), 
            ],
        }
    }
} 
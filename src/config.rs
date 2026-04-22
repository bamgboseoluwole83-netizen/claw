use alloy_primitives::Address;
use crate::types::DestroyerConfig;

pub fn load_config() -> DestroyerConfig {
    DestroyerConfig {
        drpc_url: std::env::var("DRPC_URL").unwrap_or_else(|_| "https://lb.drpc.live/ethereum/AgJp_vcoLE0_mJ2tqAApYT2H_YaUOJ4R8ZXQtiKh6MJI".to_string()),
        flashbots_relay_url: std::env::var("FLASHBOTS_URL").unwrap_or_else(|_| "https://relay.flashbots.net".to_string()),
        private_key: std::env::var("PRIVATE_KEY").unwrap_or_else(|_| "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string()),
        balancer_vault: "0xBA12222222228d8Ba445958a75a0704d566BF2C8".parse::<Address>().unwrap(),
    }
}

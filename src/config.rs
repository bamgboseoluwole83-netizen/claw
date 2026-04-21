use alloy_primitives::Address;
use crate::types::DestroyerConfig;

/// Loads environment variables into our strict DestroyerConfig type.
/// We will use a .env file later, but for now we hardcode the structure.
pub fn load_config() -> DestroyerConfig {
    // In a real deploy, use `dotenvy` crate. For skeleton, we mock it.
    DestroyerConfig {
        drpc_url: std::env::var("DRPC_URL").unwrap_or_else(|_| "https://eth.drpc.org".to_string()),
        flashbots_relay_url: std::env::var("FLASHBOTS_URL").unwrap_or_else(|_| "https://relay.flashbots.net".to_string()),
        private_key: std::env::var("PRIVATE_KEY").unwrap_or_else(|_| "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string()), // Default Anvil key FOR TESTING ONLY
        balancer_vault: "0xBA12222222228d8Ba445958a75a0704d566BF2C8".parse::<Address>().unwrap(),
    }
}

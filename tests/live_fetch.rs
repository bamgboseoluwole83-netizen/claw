use alloy::providers::{Provider, ProviderBuilder};
use alloy_primitives::Address;

#[tokio::test]
async fn test_fetch_usdc_bytecode() {
    let rpc_url = std::env::var("RPC_URL")
        .unwrap_or_else(|_| "https://mainnet.base.org".to_string());
    let provider = ProviderBuilder::new().on_http(rpc_url.parse().unwrap());
    let usdc: Address = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".parse().unwrap();

    let code = provider.get_code_at(usdc).await.unwrap();
    assert!(!code.is_empty(), "USDC bytecode should be non‑empty");
    println!("✅ USDC bytecode fetched: {} bytes", code.len());
}

use std::process::Command;
use alloy_primitives::Address;
use tracing::info;

pub fn resolve_proxy_heimdall(address: &str) -> Option<Address> {
    let output = Command::new("heimdall")
        .args(["resolve", address, "--rpc-url", "https://mainnet.base.org"])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    info!("heimdall resolve stdout: {}", stdout);

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("0x") && trimmed.len() == 42 {
            if let Ok(addr) = trimmed.parse::<Address>() {
                // Fixed comparison: convert `address` to lowercase String
                if addr.to_string().to_lowercase() != address.to_lowercase() {
                    info!("🔮 Heimdall resolved proxy to {:?}", addr);
                    return Some(addr);
                }
            }
        }
    }
    None
}
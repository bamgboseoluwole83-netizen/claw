use std::process::Command;
use alloy_primitives::Address;
use tracing::info;

/// Uses the heimdall CLI to resolve any proxy, including Diamond and EIP‑1967.
/// Returns the implementation address or None.
pub fn resolve_proxy_heimdall(address: &str) -> Option<Address> {
    let output = Command::new("heimdall")
        .args(["resolve", address, "--rpc-url", "https://mainnet.base.org"])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Parse the first line that starts with "0x" and is 42 chars long
    for line in stdout.lines() {
        if let Some(addr_str) = line.trim().split_whitespace().last() {
            if addr_str.starts_with("0x") && addr_str.len() == 42 {
                if let Ok(addr) = addr_str.parse() {
                    info!("🔮 Heimdall resolved proxy to {}", addr_str);
                    return Some(addr);
                }
            }
        }
    }
    None
}

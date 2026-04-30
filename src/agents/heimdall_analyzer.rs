use std::process::Command;
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use tracing::{info, warn};
use serde_json::Value;
use alloy_primitives::Address;

/// Cached results to avoid calling heimdall multiple times for the same address.
static CACHE: Lazy<Mutex<HashMap<String, ContractAnalysis>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Information extracted from a contract by heimdall.
#[derive(Debug, Clone)]
pub struct ContractAnalysis {
    /// Storage slot → variable name (if available)
    pub storage_slots: HashMap<usize, String>,
    /// Addresses of contracts called via STATICCALL (potential price oracles)
    pub potential_oracles: Vec<Address>,
    /// Function selectors found in the contract
    pub function_selectors: Vec<String>,
    /// Raw decompiled pseudocode (for deeper analysis later)
    pub pseudocode: String,
}

/// Run heimdall on a given address and extract oracle-related data.
/// Returns None if heimdall is not installed or the analysis fails.
pub fn analyze(address: &str) -> Option<ContractAnalysis> {
    // Check cache first
    let mut cache = CACHE.lock().unwrap();
    if let Some(analysis) = cache.get(address) {
        return Some(analysis.clone());
    }

    // Call heimdall CLI (e.g., heimdall decompile <address>)
    let output = Command::new("heimdall")
        .args(["decompile", address])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        warn!("Heimdall decompile failed for {}: {}", address, stdout);
        return None;
    }

    // Parse the JSON output (heimdall v0.9+ outputs structured JSON)
    let json: Value = serde_json::from_str(&stdout).ok()?;
    let mut analysis = ContractAnalysis {
        storage_slots: HashMap::new(),
        potential_oracles: Vec::new(),
        function_selectors: Vec::new(),
        pseudocode: String::new(),
    };

    // Extract storage layout (if present)
    if let Some(storage) = json["storage_layout"].as_object() {
        for (slot_str, var_name) in storage {
            if let Ok(slot) = usize::from_str_radix(slot_str.trim_start_matches("0x"), 16) {
                analysis.storage_slots.insert(slot, var_name.as_str().unwrap_or("unknown").to_string());
            }
        }
    }

    // Extract external calls (STATICCALL targets)
    if let Some(calls) = json["static_calls"].as_array() {
        for call in calls {
            if let Some(addr_str) = call.as_str() {
                if let Ok(addr) = addr_str.parse::<Address>() {
                    analysis.potential_oracles.push(addr);
                }
            }
        }
    }

    // Extract function selectors
    if let Some(selectors) = json["selectors"].as_array() {
        for sel in selectors {
            if let Some(s) = sel.as_str() {
                analysis.function_selectors.push(s.to_string());
            }
        }
    }

    // Keep raw pseudocode for future use
    if let Some(pseudo) = json["pseudocode"].as_str() {
        analysis.pseudocode = pseudo.to_string();
    }

    info!("Heimdall analysis complete for {}: {} oracles, {} slots",
        address, analysis.potential_oracles.len(), analysis.storage_slots.len());

    cache.insert(address.to_string(), analysis.clone());
    Some(analysis)
}

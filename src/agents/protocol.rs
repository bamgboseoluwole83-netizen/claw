use std::collections::{HashMap, HashSet};

use alloy::primitives::Address;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProtocolType {
    Unknown,
    Lending,
    DexPool,
    DexRouter,
    Vault,
    TokenErc20,
    TokenErc721,
    TokenErc1155,
    Oracle,
    Proxy,
    Governance,
    MultiSig,
    Bridge,
    Staker,
    NftMarketplace,
}

impl ProtocolType {
    pub fn label(self) -> &'static str {
        match self {
            ProtocolType::Unknown => "Unknown",
            ProtocolType::Lending => "Lending Protocol",
            ProtocolType::DexPool => "DEX Pool",
            ProtocolType::DexRouter => "DEX Router",
            ProtocolType::Vault => "Vault",
            ProtocolType::TokenErc20 => "ERC20 Token",
            ProtocolType::TokenErc721 => "ERC721 Token",
            ProtocolType::TokenErc1155 => "ERC1155 Token",
            ProtocolType::Oracle => "Oracle",
            ProtocolType::Proxy => "Proxy",
            ProtocolType::Governance => "Governance",
            ProtocolType::MultiSig => "MultiSig",
            ProtocolType::Bridge => "Bridge",
            ProtocolType::Staker => "Staker",
            ProtocolType::NftMarketplace => "NFT Marketplace",
        }
    }

    /// Priority scoring for contests (lending/DEX/vault = higher value)
    pub fn contest_priority(self) -> u8 {
        match self {
            ProtocolType::Lending => 10,
            ProtocolType::DexPool | ProtocolType::DexRouter => 9,
            ProtocolType::Vault => 8,
            ProtocolType::Bridge => 7,
            ProtocolType::Oracle => 6,
            ProtocolType::Governance => 5,
            ProtocolType::Staker => 4,
            ProtocolType::TokenErc20 => 3,
            _ => 1,
        }
    }

    /// Suggested aggressiveness for economic strategies
    pub fn economic_strategies(self) -> &'static [&'static str] {
        match self {
            ProtocolType::Lending => &[
                "oracle_manipulation",
                "twap_oracle_manipulation",
                "flash_liquidity_drain",
                "cross_protocol_liquidation",
            ],
            ProtocolType::DexPool | ProtocolType::DexRouter => {
                &["mev_sandwich", "price_arbitrage", "multi_hop_arbitrage"]
            }
            ProtocolType::Vault => &["erc4626_inflation", "flash_liquidity_drain"],
            _ => &["oracle_manipulation", "price_arbitrage"],
        }
    }

    /// Risk indicators to enable based on protocol type
    pub fn enabled_risk_indicators(self) -> &'static [&'static str] {
        match self {
            ProtocolType::Lending => &[
                "DELEGATECALL",
                "SELFDESTRUCT",
                "extcodecopy",
                "dynamic targets",
            ],
            ProtocolType::Vault => &["SELFDESTRUCT", "writable slots", "dynamic targets"],
            ProtocolType::DexPool => &["external CALL", "Contract creation"],
            _ => &["DELEGATECALL", "SELFDESTRUCT"],
        }
    }
}

/// Classifies a contract's protocol type from its function selectors
pub struct ProtocolClassifier {
    /// Known protocol signatures: selector -> protocol type + name
    known_selectors: HashMap<[u8; 4], (&'static str, ProtocolType)>,
}

impl ProtocolClassifier {
    pub fn new() -> Self {
        let mut ks: HashMap<[u8; 4], (&'static str, ProtocolType)> = HashMap::new();

        // ── ERC20 ──
        ks.insert(
            [0xa9, 0x05, 0x9c, 0xbb],
            ("transfer", ProtocolType::TokenErc20),
        );
        ks.insert(
            [0x09, 0x5e, 0xa7, 0xb3],
            ("approve", ProtocolType::TokenErc20),
        );
        ks.insert(
            [0x70, 0xa0, 0x82, 0x31],
            ("balanceOf", ProtocolType::TokenErc20),
        );
        ks.insert(
            [0xdd, 0x62, 0xed, 0x3e],
            ("transferFrom", ProtocolType::TokenErc20),
        );
        ks.insert(
            [0x18, 0x16, 0x0d, 0xdd],
            ("totalSupply", ProtocolType::TokenErc20),
        );

        // ── ERC4626 Vault ──
        ks.insert([0x6e, 0x55, 0x3f, 0x65], ("deposit", ProtocolType::Vault));
        ks.insert([0xba, 0x08, 0x7d, 0x5b], ("mint", ProtocolType::Vault));
        ks.insert([0x7a, 0x25, 0x0d, 0x56], ("withdraw", ProtocolType::Vault));
        ks.insert([0xce, 0x96, 0xc7, 0x15], ("redeem", ProtocolType::Vault));
        ks.insert(
            [0x38, 0xd0, 0x52, 0x3c],
            ("totalAssets", ProtocolType::Vault),
        );
        ks.insert(
            [0x18, 0x16, 0x0d, 0xdd],
            ("totalSupply", ProtocolType::Vault),
        );
        ks.insert(
            [0x8e, 0xa0, 0x55, 0x26],
            ("convertToShares", ProtocolType::Vault),
        );

        // ── Uniswap V2 Pair ──
        ks.insert([0x02, 0x22, 0x9c, 0x0a], ("mint", ProtocolType::DexPool));
        ks.insert([0x02, 0x22, 0x72, 0x84], ("burn", ProtocolType::DexPool));
        ks.insert([0x02, 0x4e, 0xcf, 0xc8], ("swap", ProtocolType::DexPool));
        ks.insert(
            [0x09, 0x02, 0xf1, 0xac],
            ("getReserves", ProtocolType::DexPool),
        );
        ks.insert([0x0d, 0xfe, 0x16, 0x81], ("token0", ProtocolType::DexPool));
        ks.insert([0xd2, 0x12, 0xda, 0xdf], ("token1", ProtocolType::DexPool));

        // ── Uniswap V2 Router ──
        ks.insert(
            [0x38, 0xed, 0x17, 0x39],
            ("swapExactTokensForTokens", ProtocolType::DexRouter),
        );
        ks.insert(
            [0x88, 0x03, 0xdb, 0xee],
            ("swapExactETHForTokens", ProtocolType::DexRouter),
        );
        ks.insert(
            [0x79, 0x1a, 0xc9, 0x47],
            ("swapExactTokensForETH", ProtocolType::DexRouter),
        );
        ks.insert(
            [0x7f, 0xf3, 0x61, 0xab],
            ("addLiquidity", ProtocolType::DexRouter),
        );
        ks.insert(
            [0xe8, 0xe3, 0x37, 0x00],
            ("addLiquidityETH", ProtocolType::DexRouter),
        );

        // ── Lending (Aave/Compound) ──
        ks.insert([0xc5, 0xeb, 0xea, 0xec], ("borrow", ProtocolType::Lending));
        ks.insert([0xd0, 0xe3, 0x0d, 0xb0], ("deposit", ProtocolType::Lending));
        ks.insert(
            [0x85, 0x38, 0x28, 0xb6],
            ("withdrawAll", ProtocolType::Lending),
        );
        ks.insert(
            [0xec, 0x8e, 0x48, 0x60],
            ("liquidate", ProtocolType::Lending),
        );
        ks.insert([0x04, 0x1f, 0x79, 0x36], ("repay", ProtocolType::Lending));
        ks.insert(
            [0x3e, 0xc0, 0x6d, 0x96],
            ("setOracle", ProtocolType::Lending),
        );
        ks.insert(
            [0x9a, 0x99, 0xb4, 0xce],
            ("getAccountHealth", ProtocolType::Lending),
        );

        // ── Oracle ──
        ks.insert([0xfe, 0xaf, 0xfc, 0xef], ("getPrice", ProtocolType::Oracle));
        ks.insert([0xf2, 0x20, 0xdc, 0x04], ("setPrice", ProtocolType::Oracle));
        ks.insert([0x3a, 0x0a, 0x35, 0x36], ("consult", ProtocolType::Oracle));
        ks.insert(
            [0x50, 0xd2, 0x5b, 0x5c],
            ("latestAnswer", ProtocolType::Oracle),
        );
        ks.insert(
            [0x9e, 0x6b, 0x15, 0x4a],
            ("twapPrice", ProtocolType::Oracle),
        );

        // ── Governance ──
        ks.insert(
            [0x36, 0x62, 0x14, 0x6f],
            ("propose", ProtocolType::Governance),
        );
        ks.insert(
            [0x01, 0x3c, 0xf4, 0x4b],
            ("execute", ProtocolType::Governance),
        );
        ks.insert([0x76, 0x7a, 0xde, 0x1c], ("vote", ProtocolType::Governance));
        ks.insert(
            [0x5d, 0x84, 0x31, 0x7c],
            ("queue", ProtocolType::Governance),
        );

        // ── Bridge ──
        ks.insert([0x40, 0x0a, 0x07, 0xcf], ("deposit", ProtocolType::Bridge));
        ks.insert([0x5a, 0x3a, 0x9b, 0x88], ("withdraw", ProtocolType::Bridge));
        ks.insert([0xae, 0x66, 0x04, 0x91], ("relay", ProtocolType::Bridge));

        // ── MultiSig ──
        ks.insert(
            [0x17, 0x3b, 0x73, 0x5d],
            ("submitTransaction", ProtocolType::MultiSig),
        );
        ks.insert(
            [0xc0, 0x1a, 0x8c, 0x84],
            ("confirmTransaction", ProtocolType::MultiSig),
        );
        ks.insert(
            [0x20, 0xea, 0x8d, 0xb5],
            ("executeTransaction", ProtocolType::MultiSig),
        );

        // ── NFT Marketplace ──
        ks.insert(
            [0x93, 0x82, 0x6b, 0x7e],
            ("buyItem", ProtocolType::NftMarketplace),
        );
        ks.insert(
            [0x76, 0x2c, 0x30, 0xce],
            ("sellItem", ProtocolType::NftMarketplace),
        );
        ks.insert(
            [0xac, 0x38, 0x67, 0x9b],
            ("cancelOrder", ProtocolType::NftMarketplace),
        );

        Self {
            known_selectors: ks,
        }
    }

    /// Classify contract from its deployed bytecode
    pub fn classify_from_bytecode(&self, bytecode: &[u8]) -> ProtocolClassification {
        let selectors = self.extract_4byte_selectors(bytecode);
        self.classify_from_selectors(&selectors)
    }

    /// Classify from a pre-extracted list of 4-byte selectors
    pub fn classify_from_selectors(&self, selectors: &[[u8; 4]]) -> ProtocolClassification {
        if selectors.is_empty() {
            return ProtocolClassification {
                primary: ProtocolType::Unknown,
                secondary: None,
                confidence: 0.0,
                matched_selectors: vec![],
                all_selectors: vec![],
            };
        }

        // Count matches per protocol type
        let mut type_votes: HashMap<ProtocolType, Vec<&str>> = HashMap::new();

        for sel in selectors {
            if let Some((name, ptype)) = self.known_selectors.get(sel) {
                type_votes.entry(*ptype).or_default().push(name);
            }
        }

        // Find the most matched type
        let all_selectors: Vec<String> = selectors.iter().map(|s| hex::encode(s)).collect();
        let matched_selectors: Vec<(String, String, ProtocolType)> = selectors
            .iter()
            .filter_map(|sel| {
                self.known_selectors
                    .get(sel)
                    .map(|(name, ptype)| (hex::encode(sel), name.to_string(), *ptype))
            })
            .collect();

        let best = type_votes.iter().max_by_key(|(_, names)| names.len());

        match best {
            Some((ptype, names)) => {
                let ptype = *ptype;
                let confidence = names.len() as f64 / selectors.len().clamp(1, usize::MAX) as f64;
                let confidence = confidence.clamp(0.0, 1.0);

                // Find secondary type
                let mut remaining: Vec<(ProtocolType, Vec<&str>)> = type_votes
                    .clone()
                    .into_iter()
                    .filter(|(t, _)| *t != ptype)
                    .collect();
                remaining.sort_by_key(|(_, n)| n.len());
                let secondary = remaining.pop().map(|(t, _)| t);

                ProtocolClassification {
                    primary: ptype,
                    secondary,
                    confidence,
                    matched_selectors: matched_selectors,
                    all_selectors,
                }
            }
            None => ProtocolClassification {
                primary: ProtocolType::Unknown,
                secondary: None,
                confidence: 0.0,
                matched_selectors: matched_selectors,
                all_selectors,
            },
        }
    }

    /// Extract 4-byte function selectors from bytecode (PUSH4 + EQ pattern)
    fn extract_4byte_selectors(&self, bytecode: &[u8]) -> Vec<[u8; 4]> {
        let mut selectors = Vec::new();
        let mut seen = HashSet::new();
        let mut i = 0;

        while i + 5 < bytecode.len() {
            if bytecode[i] == 0x63 {
                let mut sel = [0u8; 4];
                sel.copy_from_slice(&bytecode[i + 1..i + 5]);
                let next = bytecode[i + 5];
                if (next == 0x14 || next == 0x15) && seen.insert(sel) {
                    selectors.push(sel);
                }
            }
            i += 1;
        }
        selectors
    }
}

#[derive(Debug, Clone)]
pub struct ProtocolClassification {
    pub primary: ProtocolType,
    pub secondary: Option<ProtocolType>,
    pub confidence: f64,
    pub matched_selectors: Vec<(String, String, ProtocolType)>,
    pub all_selectors: Vec<String>,
}

impl Default for ProtocolClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selector(hex_str: &str) -> [u8; 4] {
        let bytes = hex::decode(hex_str).unwrap();
        let mut sel = [0u8; 4];
        sel.copy_from_slice(&bytes);
        sel
    }

    #[test]
    fn test_classify_lending() {
        let classifier = ProtocolClassifier::new();
        let sels = vec![
            selector("c5ebeaec"), // borrow
            selector("d0e30db0"), // deposit
            selector("ec8e4860"), // liquidate
            selector("041f7936"), // repay
        ];
        let result = classifier.classify_from_selectors(&sels);
        assert_eq!(result.primary, ProtocolType::Lending);
        assert!(result.confidence > 0.0);
    }

    #[test]
    fn test_classify_dex_pool() {
        let classifier = ProtocolClassifier::new();
        let sels = vec![
            selector("02229c0a"), // mint
            selector("02227284"), // burn
            selector("024ecfc8"), // swap
            selector("0902f1ac"), // getReserves
        ];
        let result = classifier.classify_from_selectors(&sels);
        assert_eq!(result.primary, ProtocolType::DexPool);
    }

    #[test]
    fn test_classify_vault() {
        let classifier = ProtocolClassifier::new();
        let sels = vec![
            selector("6e553f65"), // deposit
            selector("ba087d5b"), // mint
            selector("7a250d56"), // withdraw
            selector("ce96c715"), // redeem
            selector("38d0523c"), // totalAssets
        ];
        let result = classifier.classify_from_selectors(&sels);
        assert_eq!(result.primary, ProtocolType::Vault);
    }

    #[test]
    fn test_classify_unknown() {
        let classifier = ProtocolClassifier::new();
        let sels = vec![selector("deadbeef"), selector("cafebabe")];
        let result = classifier.classify_from_selectors(&sels);
        assert_eq!(result.primary, ProtocolType::Unknown);
        assert_eq!(result.confidence, 0.0);
    }

    #[test]
    fn test_classifier_empty() {
        let classifier = ProtocolClassifier::new();
        let result = classifier.classify_from_selectors(&[]);
        assert_eq!(result.primary, ProtocolType::Unknown);
    }

    #[test]
    fn test_classify_erc20() {
        let classifier = ProtocolClassifier::new();
        let sels = vec![
            selector("a9059cbb"), // transfer
            selector("095ea7b3"), // approve
            selector("70a08231"), // balanceOf
            selector("18160ddd"), // totalSupply
        ];
        let result = classifier.classify_from_selectors(&sels);
        assert_eq!(result.primary, ProtocolType::TokenErc20);
    }

    #[test]
    fn test_classify_mixed_lending_with_secondary() {
        let classifier = ProtocolClassifier::new();
        let sels = vec![
            selector("c5ebeaec"), // borrow
            selector("d0e30db0"), // deposit
            selector("0902f1ac"), // getReserves (dex)
        ];
        let result = classifier.classify_from_selectors(&sels);
        // 2 lending, 1 dex -> should be lending
        assert_eq!(result.primary, ProtocolType::Lending);
        assert_eq!(result.secondary, Some(ProtocolType::DexPool));
    }

    #[test]
    fn test_protocol_type_label() {
        assert_eq!(ProtocolType::Lending.label(), "Lending Protocol");
        assert_eq!(ProtocolType::Unknown.label(), "Unknown");
        assert_eq!(ProtocolType::DexPool.label(), "DEX Pool");
    }

    #[test]
    fn test_contest_priority_lending() {
        assert_eq!(ProtocolType::Lending.contest_priority(), 10);
        assert_eq!(ProtocolType::Unknown.contest_priority(), 1);
    }

    #[test]
    fn test_economic_strategies_not_empty() {
        assert!(!ProtocolType::Lending.economic_strategies().is_empty());
        assert!(!ProtocolType::DexPool.economic_strategies().is_empty());
    }

    #[test]
    fn test_extract_4byte_selectors() {
        let classifier = ProtocolClassifier::new();
        // PUSH4 + EQ
        let bytecode = vec![
            0x63, 0xa9, 0x05, 0x9c, 0xbb, 0x14, // transfer + EQ
            0x63, 0x09, 0x5e, 0xa7, 0xb3, 0x14, // approve + EQ
        ];
        let sels = classifier.extract_4byte_selectors(&bytecode);
        assert_eq!(sels.len(), 2);
    }
}

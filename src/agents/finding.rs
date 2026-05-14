use alloy::primitives::Bytes;
use alloy::primitives::{Address, U256};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ToolKind {
    Slither,
    Conkas,
    Foray,
    Medusa,
    Halmos,
    Heimdall,
    Synthesizer,
    Wake,
    Mythril,
    Ityfuzz,
    Economic,
}

impl fmt::Display for ToolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolKind::Slither => write!(f, "Slither"),
            ToolKind::Conkas => write!(f, "Conkas"),
            ToolKind::Foray => write!(f, "Foray"),
            ToolKind::Medusa => write!(f, "Medusa"),
            ToolKind::Halmos => write!(f, "Halmos"),
            ToolKind::Heimdall => write!(f, "Heimdall"),
            ToolKind::Synthesizer => write!(f, "Synthesizer"),
            ToolKind::Wake => write!(f, "Wake"),
            ToolKind::Mythril => write!(f, "Mythril"),
            ToolKind::Ityfuzz => write!(f, "Ityfuzz"),
            ToolKind::Economic => write!(f, "Economic"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub tool: ToolKind,
    pub severity: f64,
    pub confidence: f64,
    pub description: String,
    pub target: Address,
    pub calldata: Option<Bytes>,
    pub evidence: Vec<String>,
}

impl Finding {
    pub fn score(&self) -> f64 {
        let base = self.severity * self.confidence;

        // Profit potential boost - findings with calldata can be exploited
        let mut profit_boost = 1.0;

        // Configurable boost values from env vars
        let calldata_boost: f64 = std::env::var("SCORE_CALLDATA_BOOST")
            .unwrap_or_else(|_| "1.30".to_string())
            .parse()
            .unwrap_or(1.30);

        let critical_boost: f64 = std::env::var("SCORE_CRITICAL_BOOST")
            .unwrap_or_else(|_| "1.25".to_string())
            .parse()
            .unwrap_or(1.25);

        let chain_boost: f64 = std::env::var("SCORE_CHAIN_BOOST")
            .unwrap_or_else(|_| "1.20".to_string())
            .parse()
            .unwrap_or(1.20);

        let penalty_factor: f64 = std::env::var("SCORE_LOW_SEVERITY_PENALTY")
            .unwrap_or_else(|_| "0.80".to_string())
            .parse()
            .unwrap_or(0.80);

        // 30% boost if finding has calldata (exploitable)
        if self.calldata.is_some() {
            profit_boost *= calldata_boost;
        }

        // 25% boost if it's a critical function that can extract funds
        let desc_lower = self.description.to_lowercase();
        let is_critical = desc_lower.contains("withdrawall")
            || desc_lower.contains("withdraw")
            || desc_lower.contains("transfer")
            || desc_lower.contains("setprice")
            || desc_lower.contains("mint")
            || desc_lower.contains("burn")
            || desc_lower.contains("setowner")
            || desc_lower.contains("execute");

        if is_critical && self.calldata.is_some() {
            profit_boost *= critical_boost;
        }

        // 20% boost for chain (multi-step exploits)
        if desc_lower.contains("chain") {
            profit_boost *= chain_boost;
        }

        // Penalty for purely informational findings without calldata
        if self.calldata.is_none() && self.severity < 4.0 {
            profit_boost *= penalty_factor;
        }

        base * profit_boost
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum VerifyStatus {
    Verified,
    Partial,
    Reverted,
}

impl fmt::Display for VerifyStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerifyStatus::Verified => write!(f, "✅ Verified"),
            VerifyStatus::Partial => write!(f, "⚠️ Partial"),
            VerifyStatus::Reverted => write!(f, "❌ Reverted"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VerifiedFinding {
    pub tool: ToolKind,
    pub description: String,
    pub target: Address,
    pub calldata: Bytes,
    pub profit_estimate: U256,
    pub severity: f64,
    pub score: f64,
    pub evidence: Vec<String>,
    pub status: VerifyStatus,
}

/// Deduplicate findings by grouping similar vulnerabilities
///
/// Findings are considered duplicates if they have:
/// 1. Same function selector (from calldata or description)
/// 2. Same vulnerability class (reentrancy, oracle, access control, etc.)
///
/// Returns deduplicated findings with merged evidence
pub fn deduplicate_findings(findings: &[Finding]) -> Vec<Finding> {
    use std::collections::HashMap;

    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    struct FindingKey {
        selector: Option<String>,
        vuln_class: String,
        target: Address,
    }

    let mut groups: HashMap<FindingKey, Vec<&Finding>> = HashMap::new();

    for finding in findings {
        let key = FindingKey {
            selector: extract_selector(finding),
            vuln_class: classify_vulnerability(&finding.description),
            target: finding.target,
        };
        groups.entry(key).or_default().push(finding);
    }

    let mut deduplicated = Vec::new();

    for (_key, group) in groups {
        if group.len() == 1 {
            deduplicated.push(group[0].clone());
            continue;
        }

        // Merge duplicates
        let mut merged = group[0].clone();
        merged.description = format!(
            "[{} tools] {}",
            group.len(),
            group
                .iter()
                .map(|f| f.description.split(':').next().unwrap_or(&f.description))
                .collect::<Vec<_>>()
                .join(", ")
        );

        // Merge evidence from all findings
        let mut all_evidence = Vec::new();
        for f in &group {
            all_evidence.extend(f.evidence.clone());
        }
        merged.evidence = all_evidence;

        // Take highest severity
        let max_severity = group.iter().map(|f| f.severity).fold(0.0, f64::max);
        merged.severity = max_severity;

        // Average confidence
        let avg_confidence: f64 =
            group.iter().map(|f| f.confidence).sum::<f64>() / group.len() as f64;
        merged.confidence = avg_confidence;

        // Keep calldata if any finding has it
        if merged.calldata.is_none() {
            for f in &group {
                if f.calldata.is_some() {
                    merged.calldata = f.calldata.clone();
                    break;
                }
            }
        }

        deduplicated.push(merged);
    }

    deduplicated
}

fn extract_selector(finding: &Finding) -> Option<String> {
    if let Some(ref cd) = finding.calldata {
        let len = cd.len();
        if len >= 4 {
            let bytes: &[u8] = cd;
            return Some(hex::encode(&bytes[..4]));
        }
    }
    None
}

fn classify_vulnerability(description: &str) -> String {
    let desc = description.to_lowercase();

    if desc.contains("reentrancy") || desc.contains("callback") {
        "reentrancy".to_string()
    } else if desc.contains("oracle") || desc.contains("price") || desc.contains("getprice") {
        "oracle".to_string()
    } else if desc.contains("delegatecall") || desc.contains("proxy") {
        "delegatecall".to_string()
    } else if desc.contains("access") || desc.contains("owner") || desc.contains("permission") {
        "access_control".to_string()
    } else if desc.contains("selfdestruct") || desc.contains("suicide") {
        "selfdestruct".to_string()
    } else if desc.contains("flashloan") || desc.contains("flash") {
        "flashloan".to_string()
    } else if desc.contains("overflow") || desc.contains("underflow") || desc.contains("arith") {
        "arithmetic".to_string()
    } else {
        "other".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_finding(
        severity: f64,
        confidence: f64,
        description: &str,
        has_calldata: bool,
    ) -> Finding {
        Finding {
            tool: ToolKind::Slither,
            severity,
            confidence,
            description: description.to_string(),
            target: Address::ZERO,
            calldata: if has_calldata {
                Some(Bytes::from(vec![0u8; 4]))
            } else {
                None
            },
            evidence: vec![],
        }
    }

    #[test]
    fn test_base_score_no_boosts() {
        let f = make_finding(5.0, 0.5, "some issue", false);
        let score = f.score();
        assert_eq!(score, 2.5, "base score = severity * confidence");
    }

    #[test]
    fn test_calldata_boost_30_percent() {
        let without = make_finding(5.0, 0.5, "issue", false);
        let with = make_finding(5.0, 0.5, "issue", true);
        let ratio = with.score() / without.score();
        assert!(
            (ratio - 1.30).abs() < 0.01,
            "calldata should give 30% boost"
        );
    }

    #[test]
    fn test_critical_function_boost() {
        let mut f = make_finding(5.0, 0.5, "withdraw all user funds", true);
        f.description = "function withdrawAll()".to_string();
        let score = f.score();
        assert!(
            score > 3.25,
            "critical function + calldata should get extra 25% boost"
        );
    }

    #[test]
    fn test_chain_keyword_boost() {
        let f = make_finding(5.0, 0.5, "cross-chain reentrancy", true);
        // Description has "chain" - gets 20% boost regardless of critical keywords
        let score = f.score();
        let expected = 5.0 * 0.5 * 1.30 * 1.20; // calldata + chain boost
        assert!((score - expected).abs() < 0.01, "chain adds 20% boost");
    }

    #[test]
    fn test_penalty_for_info_findings_without_calldata() {
        let f = make_finding(3.0, 0.5, "informational issue", false);
        let score = f.score();
        let base = 3.0 * 0.5;
        assert!(
            score < base,
            "info findings without calldata should get penalty"
        );
    }

    #[test]
    fn test_high_severity_no_penalty() {
        let f = make_finding(7.0, 0.8, "high severity", false);
        let score = f.score();
        let expected = 7.0 * 0.8; // no penalty for severity >= 4.0
        assert!(
            (score - expected).abs() < 0.01,
            "high severity should not be penalized"
        );
    }

    #[test]
    fn test_selfdestruct_high_severity() {
        let f = make_finding(10.0, 0.9, "selfdestruct can be called by anyone", false);
        let score = f.score();
        assert!(score >= 9.0, "selfdestruct should score high");
    }

    #[test]
    fn test_multiple_critical_keywords() {
        let f = make_finding(8.0, 0.7, "withdraw in callback can transfer tokens", true);
        // Description has "withdraw" (critical) but NOT "chain", so chain boost doesn't apply
        let score = f.score();
        let expected = 8.0 * 0.7 * 1.30 * 1.25; // calldata + critical, no chain
        assert!(
            (score - expected).abs() < 0.01,
            "critical + calldata = 1.30*1.25 boost"
        );
    }

    #[test]
    fn test_tool_kind_display() {
        assert_eq!(ToolKind::Slither.to_string(), "Slither");
        assert_eq!(ToolKind::Ityfuzz.to_string(), "Ityfuzz");
        assert_eq!(ToolKind::Wake.to_string(), "Wake");
    }

    #[test]
    fn test_verify_status_display() {
        assert_eq!(VerifyStatus::Verified.to_string(), "✅ Verified");
        assert_eq!(VerifyStatus::Partial.to_string(), "⚠️ Partial");
        assert_eq!(VerifyStatus::Reverted.to_string(), "❌ Reverted");
    }
}

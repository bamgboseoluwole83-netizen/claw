use alloy_primitives::U256;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity { Critical, High, Medium, Low, Informational }

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Critical => write!(f, "CRITICAL"),
            Severity::High => write!(f, "HIGH"),
            Severity::Medium => write!(f, "MEDIUM"),
            Severity::Low => write!(f, "LOW"),
            Severity::Informational => write!(f, "INFO"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence { Proven, High, Medium, Low }

pub struct Impact {
    pub profit_usd_scaled: U256,
    pub value_lost_usd_scaled: U256,
    pub tvl_pct_scaled: u64,
    pub requires_privilege: bool,
    pub permanent_loss: bool,
    pub protocol_insolvent: bool,
    pub affected_users: u64,
}

impl Impact {
    pub fn from_drain(amount: U256, _decimals: u8, _price: U256, _tvl: U256, _perm: bool, _insolvent: bool, _affected: u64, _priv: bool) -> Self {
        Self {
            profit_usd_scaled: amount,
            value_lost_usd_scaled: amount,
            tvl_pct_scaled: 0,
            requires_privilege: false,
            permanent_loss: false,
            protocol_insolvent: false,
            affected_users: 0,
        }
    }
}

pub fn classify(_impact: &Impact, _confidence: Confidence) -> (Severity, Confidence) {
    (Severity::Medium, Confidence::Proven)
}

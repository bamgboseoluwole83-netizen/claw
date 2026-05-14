pub mod finding;
pub mod hunt;
pub mod controller;
pub mod cross_contract;
pub mod notifier;
pub mod synthesizer;
pub mod wake;
pub mod chain;
pub mod tool_status;
pub mod economic;
pub mod poc_generator;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ScanMode {
    /// Fast static analysis only — no fuzzing
    Quick,
    /// Static analysis + Halmos + Medusa fuzzing (default)
    #[default]
    Standard,
    /// Full pipeline including Foray + Ityfuzz on-chain fuzzing
    Deep,
}

impl std::str::FromStr for ScanMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "quick" => Ok(Self::Quick),
            "standard" => Ok(Self::Standard),
            "deep" => Ok(Self::Deep),
            _ => Err(format!("Invalid scan mode '{}'. Options: quick, standard, deep", s)),
        }
    }
}

impl std::fmt::Display for ScanMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Quick => write!(f, "quick"),
            Self::Standard => write!(f, "standard"),
            Self::Deep => write!(f, "deep"),
        }
    }
}

use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Debug, Clone)]
pub struct FetchedSource {
    pub name: String,
    pub compiler_version: String,
    pub sources: Vec<SourceFile>,
    pub metadata: Option<serde_json::Value>,
    pub abi: Option<serde_json::Value>,
    pub provider: SourceProvider,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceProvider { Sourcify, Blockscout, Etherscan, None }

#[derive(Debug, Clone)]
pub struct SourceFile { pub path: String, pub content: String }

// ---------- JSON shapes ----------
#[derive(Debug, Deserialize)]
struct SourcifyResponse {
    files: Vec<SourcifyFileEntry>,
    metadata: Option<serde_json::Value>,
}
#[derive(Debug, Deserialize)]
struct SourcifyFileEntry { name: String, content: String }

#[derive(Debug, Deserialize)]
struct BlockscoutResponse { result: Vec<BlockscoutSource> }
#[derive(Debug, Deserialize)]
struct BlockscoutSource {
    #[serde(rename = "SourceCode")]
    source_code: String,
    #[serde(rename = "ContractName")]
    contract_name: String,
    #[serde(rename = "CompilerVersion")]
    compiler_version: String,
    #[serde(rename = "ABI")]
    abi: Option<String>,
}

pub struct SourceFetcher {
    client: Client,
    cache: Mutex<HashMap<String, Option<FetchedSource>>>,
}

impl SourceFetcher {
    pub fn new(_etherscan_key: Option<String>) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap(),
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub async fn fetch(&self, chain_id: u64, address: &str) -> Option<FetchedSource> {
        // stub – will implement the three-tier cascade later
        None
    }
}

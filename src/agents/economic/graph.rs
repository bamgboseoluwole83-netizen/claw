use std::collections::{HashMap, HashSet, VecDeque};

use alloy::primitives::Address;

use crate::agents::economic::amm::AMModel;
use crate::agents::economic::pool_state::PoolState;

const MAX_DEPTH: usize = 3;
const MAX_NODES: usize = 50;
const MIN_POOL_TVL_ETH: f64 = 0.1;

#[derive(Debug, Clone)]
pub struct ContractGraph {
    nodes: HashMap<Address, ContractNode>,
    edges: Vec<ContractEdge>,
}

#[derive(Debug, Clone)]
pub struct ContractNode {
    pub address: Address,
    pub bytecode: Vec<u8>,
    pub source: NodeSource,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeSource {
    Target,
    DelegateCall,
    ExtCodeCopy,
    ExtCodeHash,
    Call,
    StaticCall,
    EmbeddedAddress,
}

#[derive(Debug, Clone)]
pub struct ContractEdge {
    pub from: Address,
    pub to: Address,
    pub edge_type: EdgeType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EdgeType {
    DelegateCall,
    Call,
    StaticCall,
    ExtCodeCopy,
}

/// Classification of a contract based on bytecode patterns
#[derive(Debug, Clone, PartialEq)]
pub enum ContractClass {
    Unknown,
    DexPool,
    LendingPool,
    Token,
    Oracle,
    Proxy,
    Wallet,
    Governance,
}

impl ContractGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn from_bytecode(target: Address, bytecode: &[u8]) -> Self {
        let mut graph = Self::new();
        graph.add_node(target, bytecode.to_vec(), NodeSource::Target);
        graph
    }

    pub async fn build(
        target: Address,
        bytecode: &[u8],
        rpc_url: &str,
    ) -> Self {
        let mut graph = Self::from_bytecode(target, bytecode);
        graph.expand(rpc_url).await;
        graph
    }

    pub fn add_node(&mut self, addr: Address, bytecode: Vec<u8>, source: NodeSource) {
        if !self.nodes.contains_key(&addr) && self.nodes.len() < MAX_NODES {
            self.nodes.insert(addr, ContractNode { address: addr, bytecode, source });
        }
    }

    pub fn add_edge(&mut self, from: Address, to: Address, edge_type: EdgeType) {
        if from != to {
            self.edges.push(ContractEdge { from, to, edge_type });
        }
    }

    /// Expand graph by fetching bytecode for discovered addresses
    pub async fn expand(&mut self, rpc_url: &str) {
        let mut queue: VecDeque<Address> = self.nodes.keys().copied().collect();
        let mut visited: HashSet<Address> = self.nodes.keys().copied().collect();
        let mut depth = 0;

        while let Some(addr) = queue.pop_front() {
            if depth >= MAX_DEPTH || self.nodes.len() >= MAX_NODES {
                break;
            }

            let needs_fetch = self.nodes.get(&addr).map(|n| n.bytecode.len() <= 4).unwrap_or(false);
            if needs_fetch {
                if let Ok(code) = Self::fetch_bytecode(rpc_url, addr).await {
                    if let Some(n) = self.nodes.get_mut(&addr) {
                        n.bytecode = code;
                    }
                }
            }

            let bytecode = self.nodes.get(&addr).map(|n| n.bytecode.clone()).unwrap_or_default();
            if bytecode.is_empty() {
                depth += 1;
                continue;
            }

            let embedded = Self::extract_addresses(&bytecode);
            for embedded_addr in embedded {
                if visited.insert(embedded_addr) {
                    self.add_node(embedded_addr, Vec::new(), NodeSource::EmbeddedAddress);
                    self.add_edge(addr, embedded_addr, EdgeType::Call);
                    queue.push_back(embedded_addr);
                }
            }

            self.discover_call_targets_from_bytecode(addr, &bytecode);

            depth += 1;
        }
    }

    fn discover_call_targets_from_bytecode(&mut self, addr: Address, bytecode: &[u8]) {
        for target in self.resolve_delegatecall_targets(bytecode) {
            if !self.nodes.contains_key(&target) {
                self.add_node(target, Vec::new(), NodeSource::DelegateCall);
                self.add_edge(addr, target, EdgeType::DelegateCall);
            }
        }
        for target in self.resolve_call_targets(bytecode) {
            if !self.nodes.contains_key(&target) {
                self.add_node(target, Vec::new(), NodeSource::Call);
                self.add_edge(addr, target, EdgeType::Call);
            }
        }
    }

    /// Resolve DELEGATECALL targets from bytecode (PUSH20 before 0xf4)
    fn resolve_delegatecall_targets(&self, bytecode: &[u8]) -> Vec<Address> {
        let mut targets = Vec::new();
        let mut i = 0;
        while i < bytecode.len() {
            if bytecode[i] == 0xf4 {
                let scan_start = if i > 40 { i - 40 } else { 0 };
                let mut j = i;
                while j > scan_start {
                    j -= 1;
                    if bytecode[j] >= 0x60 && bytecode[j] <= 0x7f {
                        let push_len = (bytecode[j] - 0x60 + 1) as usize;
                        if push_len == 20 && j + 1 + 20 <= bytecode.len() {
                            let addr = Address::from_slice(&bytecode[j + 1..j + 1 + 20]);
                            if !addr.is_zero() && !targets.contains(&addr) {
                                targets.push(addr);
                            }
                            break;
                        }
                        // Non-20-byte PUSH, continue scanning backwards
                    }
                }
            }
            i += 1;
        }
        targets
    }

    /// Resolve CALL/STATICCALL targets from bytecode
    fn resolve_call_targets(&self, bytecode: &[u8]) -> Vec<Address> {
        let mut targets = Vec::new();
        let call_ops = [0xf1u8, 0xfau8, 0xf2u8];
        let mut i = 0;
        while i < bytecode.len() {
            if call_ops.contains(&bytecode[i]) {
                let scan_start = if i > 40 { i - 40 } else { 0 };
                let mut j = i;
                let mut found = false;
                while j > scan_start && !found {
                    j -= 1;
                    if bytecode[j] >= 0x60 && bytecode[j] <= 0x7f {
                        let push_len = (bytecode[j] - 0x60 + 1) as usize;
                        if push_len == 20 && j + 1 + 20 <= bytecode.len() {
                            let addr = Address::from_slice(&bytecode[j + 1..j + 1 + 20]);
                            if !addr.is_zero() && !targets.contains(&addr) {
                                targets.push(addr);
                            }
                            found = true;
                        } else if push_len >= 20 && j + 1 + push_len <= bytecode.len() {
                            let data = &bytecode[j + 1..j + 1 + push_len];
                            if data.len() >= 20 {
                                let addr_bytes = &data[data.len() - 20..];
                                let addr = Address::from_slice(addr_bytes);
                                if !addr.is_zero() && !targets.contains(&addr) {
                                    targets.push(addr);
                                }
                                found = true;
                            }
                        }
                    }
                }
            }
            i += 1;
        }
        targets
    }

    /// Extract all PUSH20 addresses from bytecode
    fn extract_addresses(bytecode: &[u8]) -> Vec<Address> {
        let mut addrs = Vec::new();
        let mut i = 0;
        while i < bytecode.len() {
            if bytecode[i] == 0x73 {
                if i + 1 + 20 <= bytecode.len() {
                    let addr = Address::from_slice(&bytecode[i + 1..i + 1 + 20]);
                    if !addr.is_zero() && !addrs.contains(&addr) {
                        addrs.push(addr);
                    }
                }
            }
            i += 1;
        }
        addrs
    }

    /// Fetch bytecode from RPC via eth_getCode
    async fn fetch_bytecode(rpc_url: &str, addr: Address) -> Result<Vec<u8>, String> {
        let client = reqwest::Client::new();
        let params = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_getCode",
            "params": [hex::encode(addr), "latest"]
        });

        let resp = client
            .post(rpc_url)
            .json(&params)
            .send()
            .await
            .map_err(|e| format!("RPC failed: {}", e))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse failed: {}", e))?;

        let result = body["result"]
            .as_str()
            .ok_or("No result")?;

        hex::decode(result.trim_start_matches("0x"))
            .map_err(|e| format!("Hex decode failed: {}", e))
    }

    pub fn classify_node(&self, addr: &Address) -> ContractClass {
        if let Some(node) = self.nodes.get(addr) {
            classify_bytecode(&node.bytecode)
        } else {
            ContractClass::Unknown
        }
    }

    /// Find all DEX pools in the graph with on-chain reserves
    pub async fn find_pools(&self, rpc_url: &str) -> Vec<PoolState> {
        let mut pools = Vec::new();
        for (addr, _node) in &self.nodes {
            if let Ok(pool) = PoolState::fetch(rpc_url, *addr).await {
                if pool.is_valid() {
                    let tvl = pool.amm_model().tvl(1.0, 1.0);
                    if tvl >= MIN_POOL_TVL_ETH {
                        pools.push(pool);
                    }
                }
            }
        }
        pools
    }

    /// Find paths between two contracts (e.g., token → pool → oracle)
    pub fn find_path(&self, from: Address, to: Address, max_depth: usize) -> Vec<Vec<Address>> {
        let mut paths = Vec::new();
        let mut current = vec![from];
        self.dfs(from, to, max_depth, &mut current, &mut paths);
        paths
    }

    fn dfs(
        &self,
        current: Address,
        target: Address,
        depth: usize,
        path: &mut Vec<Address>,
        paths: &mut Vec<Vec<Address>>,
    ) {
        if depth == 0 || path.len() > MAX_DEPTH {
            return;
        }
        if current == target {
            paths.push(path.clone());
            return;
        }
        for edge in &self.edges {
            if edge.from == current && !path.contains(&edge.to) {
                path.push(edge.to);
                self.dfs(edge.to, target, depth - 1, path, paths);
                path.pop();
            }
        }
    }

    pub fn all_nodes(&self) -> Vec<&ContractNode> {
        self.nodes.values().collect()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

impl Default for ContractGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Classify a contract by analyzing its bytecode
fn classify_bytecode(bytecode: &[u8]) -> ContractClass {
    let has_selfdestruct = bytecode.contains(&0xff);
    let has_delegatecall = bytecode.contains(&0xf4);
    let has_create = bytecode.contains(&0xf0) || bytecode.contains(&0xf5);
    let has_sstore = bytecode.contains(&0x55);

    // Proxy pattern: minimal bytecode with DELEGATECALL
    if bytecode.len() < 100 && has_delegatecall {
        return ContractClass::Proxy;
    }

    // Wallet / multisig: often has DELEGATECALL + CREATE
    if has_delegatecall && has_create {
        // Could be a wallet, could be governance
        if has_sstore && bytecode.len() > 1000 {
            return ContractClass::Governance;
        }
        return ContractClass::Wallet;
    }

    // DEX pool: small bytecode, transfer+swap selectors
    // Real Uniswap V2 pair is ~3KB
    if bytecode.len() > 500 && bytecode.len() < 5000 && has_sstore {
        // Check for common Uniswap V2 selectors in bytecode
        let uniswap_selectors = [
            [0x02, 0x29, 0x0a, 0xca], // mint
            [0x02, 0x27, 0x28, 0x40], // burn
            [0x02, 0x4e, 0xcf, 0xc8], // swap
        ];
        for sel in &uniswap_selectors {
            if bytecode.windows(4).any(|w| w == sel) {
                return ContractClass::DexPool;
            }
        }
    }

    // Lending pool: larger codebase with borrow/deposit selectors
    if bytecode.len() > 5000 {
        let lending_selectors = [
            [0xc5, 0xeb, 0xea, 0xec], // borrow
            [0xd0, 0xe3, 0x0d, 0xb0], // deposit
            [0x85, 0x38, 0x28, 0xb6], // withdrawAll
        ];
        let matching = lending_selectors.iter().filter(|sel| bytecode.windows(4).any(|w| w == *sel)).count();
        if matching >= 2 {
            return ContractClass::LendingPool;
        }
    }

    // Oracle: has setPrice/getPrice selectors
    let oracle_selectors = [
        [0xf2, 0x20, 0xdc, 0x04], // setPrice
        [0xfe, 0xaf, 0xfc, 0xef], // getPrice
    ];
    if oracle_selectors.iter().any(|sel| bytecode.windows(4).any(|w| w == *sel)) {
        return ContractClass::Oracle;
    }

    // Token: has transfer/approve/balanceOf
    let token_selectors = [
        [0xa9, 0x05, 0x9c, 0xbb], // transfer
        [0x09, 0x5e, 0xa7, 0xb3], // approve
        [0x70, 0xa0, 0x82, 0x31], // balanceOf
    ];
    if token_selectors.iter().filter(|sel| bytecode.windows(4).any(|w| w == *sel)).count() >= 2 {
        return ContractClass::Token;
    }

    ContractClass::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_addresses_empty() {
        assert!(ContractGraph::extract_addresses(&[]).is_empty());
    }

    #[test]
    fn test_extract_addresses_single() {
        let mut bytecode = vec![0x73u8];
        bytecode.extend(std::iter::repeat(0x11u8).take(20));
        let addrs = ContractGraph::extract_addresses(&bytecode);
        assert_eq!(addrs.len(), 1);
    }

    #[test]
    fn test_extract_addresses_duplicates() {
        let mut bytecode = Vec::new();
        for _ in 0..3 {
            bytecode.push(0x73);
            bytecode.extend(std::iter::repeat(0x11u8).take(20));
        }
        let addrs = ContractGraph::extract_addresses(&bytecode);
        assert_eq!(addrs.len(), 1, "should deduplicate");
    }

    #[test]
    fn test_resolve_delegatecall_targets() {
        let mut bytecode = Vec::new();
        bytecode.push(0x73);
        bytecode.extend(std::iter::repeat(0x22u8).take(20));
        bytecode.extend_from_slice(&[0x60, 0xff, 0xf4]);
        let graph = ContractGraph::new();
        let targets = graph.resolve_delegatecall_targets(&bytecode);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].as_slice(), &[0x22u8; 20]);
    }

    #[test]
    fn test_classify_proxy() {
        let bytecode = vec![0x60, 0x00, 0x60, 0x00, 0xf4, 0x00];
        assert_eq!(classify_bytecode(&bytecode), ContractClass::Proxy);
    }

    #[test]
    fn test_classify_unknown() {
        assert_eq!(classify_bytecode(&[0x00, 0x01, 0x02]), ContractClass::Unknown);
    }

    #[test]
    fn test_graph_build_empty() {
        let graph = ContractGraph::new();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_graph_from_bytecode() {
        let addr = Address::from_slice(&[0x11u8; 20]);
        let graph = ContractGraph::from_bytecode(addr, &[0x60, 0x01]);
        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn test_find_path_no_path() {
        let graph = ContractGraph::new();
        let a = Address::from_slice(&[0x11u8; 20]);
        let b = Address::from_slice(&[0x22u8; 20]);
        let paths = graph.find_path(a, b, 3);
        assert!(paths.is_empty());
    }

    #[test]
    fn test_classify_dex_pool() {
        let mut bytecode = vec![0x60u8; 500];
        bytecode.push(0x55); // SSTORE to set has_sstore
        bytecode.extend_from_slice(&[0x02, 0x4e, 0xcf, 0xc8]); // swap(uint256,uint256,address,bytes)
        bytecode.extend_from_slice(&[0x02, 0x29, 0x0a, 0xca]); // mint
        bytecode.extend_from_slice(&[0x02, 0x27, 0x28, 0x40]); // burn
        assert_eq!(classify_bytecode(&bytecode), ContractClass::DexPool);
    }

    #[test]
    fn test_classify_lending_pool() {
        let mut bytecode = vec![0x60u8; 6000];
        bytecode.extend_from_slice(&[0xc5, 0xeb, 0xea, 0xec]); // borrow
        bytecode.extend_from_slice(&[0xd0, 0xe3, 0x0d, 0xb0]); // deposit
        assert_eq!(classify_bytecode(&bytecode), ContractClass::LendingPool);
    }

    #[test]
    fn test_classify_token() {
        let mut bytecode = vec![0x60u8; 200];
        bytecode.extend_from_slice(&[0xa9, 0x05, 0x9c, 0xbb]); // transfer
        bytecode.extend_from_slice(&[0x09, 0x5e, 0xa7, 0xb3]); // approve
        assert_eq!(classify_bytecode(&bytecode), ContractClass::Token);
    }
}

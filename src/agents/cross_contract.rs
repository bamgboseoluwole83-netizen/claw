use alloy::primitives::Address;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct HeimdallResult {
    pub storage_slots: Vec<StorageSlotInfo>,
    pub function_signatures: Vec<FunctionSignature>,
    pub risk_indicators: Vec<String>,
    pub call_graph: CallGraph,
}

#[derive(Debug, Clone)]
pub struct StorageSlotInfo {
    pub slot: String,
    pub access_count: u64,
    pub is_written: bool,
    pub is_read: bool,
}

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub selector: String,
    pub name: String,
    pub parameters: String,
    pub visibility: String,
}

#[derive(Debug, Clone, Default)]
pub struct CallGraph {
    pub external_calls: Vec<ExternalCall>,
    pub extcodecopy_targets: Vec<String>,
    pub event_signatures: Vec<String>,
    pub embedded_addresses: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ExternalCall {
    pub target: Option<String>,
    pub call_type: CallType,
    pub context: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CallType {
    Call,
    DelegateCall,
    StaticCall,
    CallCode,
}

impl CallType {
    fn from_opcode(op: u8) -> Option<Self> {
        match op {
            0xf1 => Some(Self::Call),
            0xf4 => Some(Self::DelegateCall),
            0xfa => Some(Self::StaticCall),
            0xf2 => Some(Self::CallCode),
            _ => None,
        }
    }

    pub fn risk_label(self) -> &'static str {
        match self {
            Self::DelegateCall => "DELEGATECALL (context hijack)",
            Self::Call => "CALL (ETH transfer)",
            Self::StaticCall => "STATICCALL (read-only)",
            Self::CallCode => "CALLCODE (deprecated)",
        }
    }
}

pub struct HeimdallAnalyzer;

impl HeimdallAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze(&self, bytecode: &[u8], _address: Address) -> HeimdallResult {
        let storage_slots = self.analyze_storage(bytecode);
        let functions = self.extract_functions(bytecode);
        let call_graph = self.analyze_call_graph(bytecode);
        let risk_indicators =
            self.detect_risk_indicators(bytecode, &storage_slots, &functions, &call_graph);
        HeimdallResult {
            storage_slots,
            function_signatures: functions,
            risk_indicators,
            call_graph,
        }
    }

    /// Analyze storage access patterns (SLOAD/SSTORE)
    fn analyze_storage(&self, bytecode: &[u8]) -> Vec<StorageSlotInfo> {
        let mut slots: HashMap<String, (bool, bool, u64)> = HashMap::new();
        let mut i = 0;
        while i < bytecode.len() {
            let opcode = bytecode[i];
            if opcode >= 0x60 && opcode <= 0x7f {
                let push_len = (opcode - 0x60 + 1) as usize;
                if i + 1 + push_len <= bytecode.len() {
                    let slot_bytes = &bytecode[i + 1..i + 1 + push_len];
                    let next_opcode_idx = i + 1 + push_len;
                    if next_opcode_idx < bytecode.len() {
                        let next_opcode = bytecode[next_opcode_idx];
                        if next_opcode == 0x54 || next_opcode == 0x55 {
                            let mut padded = [0u8; 32];
                            let start = 32 - push_len.min(32);
                            padded[start..].copy_from_slice(&slot_bytes[..push_len.min(32)]);
                            let slot_hex = hex::encode(padded);
                            let entry = slots.entry(slot_hex).or_insert((false, false, 0));
                            if next_opcode == 0x54 {
                                entry.0 = true;
                            } else {
                                entry.1 = true;
                            }
                            entry.2 += 1;
                        }
                    }
                    i = next_opcode_idx;
                    continue;
                }
            }
            i += 1;
        }
        slots
            .into_iter()
            .map(|(slot, (read, write, count))| StorageSlotInfo {
                slot,
                access_count: count,
                is_written: write,
                is_read: read,
            })
            .collect()
    }

    /// Extract function selectors from PUSH4 + EQ/ISZERO patterns
    fn extract_functions(&self, bytecode: &[u8]) -> Vec<FunctionSignature> {
        let mut functions = Vec::new();
        let mut i = 0;
        while i + 5 < bytecode.len() {
            if bytecode[i] == 0x63 {
                let selector = &bytecode[i + 1..i + 5];
                let selector_hex = format!("0x{}", hex::encode(selector));
                let next_opcode = bytecode.get(i + 5);
                if next_opcode == Some(&0x14) || next_opcode == Some(&0x15) {
                    functions.push(FunctionSignature {
                        selector: selector_hex,
                        name: format!("unknown_{}", hex::encode(selector)),
                        parameters: "unknown".to_string(),
                        visibility: "external".to_string(),
                    });
                }
            }
            i += 1;
        }
        functions
    }

    /// Reconstruct call graph from bytecode:
    /// - External calls (CALL/DELEGATECALL/STATICCALL/CALLCODE) and their targets
    /// - EXTCODECOPY targets
    /// - Event signatures from LOG opcodes
    /// - Embedded PUSH20 addresses
    fn analyze_call_graph(&self, bytecode: &[u8]) -> CallGraph {
        let mut external_calls = Vec::new();
        let mut extcodecopy_targets = Vec::new();
        let mut event_signatures = Vec::new();
        let mut embedded_addresses = Vec::new();

        let mut i = 0;
        while i < bytecode.len() {
            let opcode = bytecode[i];

            if let Some(call_type) = CallType::from_opcode(opcode) {
                let target = self.resolve_call_target(bytecode, i);
                let ctx = match call_type {
                    CallType::DelegateCall => {
                        if target.is_some() {
                            format!("delegatecall -> {}", target.as_ref().unwrap())
                        } else {
                            "delegatecall -> (dynamic/storage)".to_string()
                        }
                    }
                    CallType::Call => {
                        if target.is_some() {
                            format!("call -> {}", target.as_ref().unwrap())
                        } else {
                            "call -> (dynamic/storage)".to_string()
                        }
                    }
                    CallType::StaticCall => {
                        if target.is_some() {
                            format!("staticcall -> {}", target.as_ref().unwrap())
                        } else {
                            "staticcall -> (dynamic/storage)".to_string()
                        }
                    }
                    CallType::CallCode => {
                        if target.is_some() {
                            format!("callcode -> {}", target.as_ref().unwrap())
                        } else {
                            "callcode -> (dynamic/storage)".to_string()
                        }
                    }
                };
                external_calls.push(ExternalCall {
                    target,
                    call_type,
                    context: ctx,
                });
            }

            // EXTCODECOPY: copies bytecode of another contract
            if opcode == 0x3c {
                let target = self.resolve_call_target(bytecode, i);
                if let Some(addr) = target {
                    extcodecopy_targets.push(addr);
                } else {
                    extcodecopy_targets.push("(dynamic)".to_string());
                }
            }

            // EXTCODEHASH/EXTCODESIZE: querying other contract code
            if opcode == 0x3f || opcode == 0x3b {
                let target = self.resolve_call_target(bytecode, i);
                if let Some(addr) = target {
                    extcodecopy_targets.push(addr);
                }
            }

            // LOG0-LOG4: event emission
            if opcode >= 0xa0 && opcode <= 0xa4 {
                let topic_count = (opcode - 0xa0) as usize;
                // Find event signature from PUSH values before LOG
                let sig = self.resolve_event_signature(bytecode, i, topic_count);
                event_signatures.push(if let Some(s) = sig {
                    s
                } else {
                    format!("LOG{} (unresolved)", topic_count)
                });
            }

            // Track embedded PUSH20 addresses
            if opcode == 0x73 {
                // PUSH20 = 0x73 (20 bytes)
                if i + 1 + 20 <= bytecode.len() {
                    let addr_bytes = &bytecode[i + 1..i + 1 + 20];
                    let addr_hex = format!("0x{}", hex::encode(addr_bytes));
                    if !embedded_addresses.contains(&addr_hex) {
                        embedded_addresses.push(addr_hex);
                    }
                }
            }

            i += 1;
        }

        CallGraph {
            external_calls,
            extcodecopy_targets,
            event_signatures,
            embedded_addresses,
        }
    }

    /// Try to resolve the target address pushed before a call opcode.
    /// Looks back up to 40 bytes for the nearest PUSH20 or larger PUSH containing an address.
    fn resolve_call_target(&self, bytecode: &[u8], call_idx: usize) -> Option<String> {
        let scan_start = if call_idx > 40 { call_idx - 40 } else { 0 };
        let mut i = call_idx;
        let mut candidates: Vec<Vec<u8>> = Vec::new();

        while i > scan_start {
            i -= 1;
            let opcode = bytecode[i];
            if opcode >= 0x60 && opcode <= 0x7f {
                let push_len = (opcode - 0x60 + 1) as usize;
                if i + 1 + push_len == call_idx || i + 1 + push_len < call_idx {
                    let data = bytecode[i + 1..(i + 1 + push_len).min(bytecode.len())].to_vec();
                    candidates.push(data);
                    if candidates.len() >= 5 {
                        break;
                    }
                }
            }
        }

        for c in &candidates {
            if c.len() == 20 {
                return Some(format!("0x{}", hex::encode(c)));
            }
        }

        for c in &candidates {
            if c.len() == 32 {
                let last_20 = &c[12..];
                if last_20.iter().any(|b| *b != 0) {
                    return Some(format!("0x{}", hex::encode(last_20)));
                }
            }
        }

        None
    }

    /// Resolve event signatures from PUSH32 values before LOG opcodes
    fn resolve_event_signature(
        &self,
        bytecode: &[u8],
        log_idx: usize,
        topic_count: usize,
    ) -> Option<String> {
        let scan_start = if log_idx > 40 { log_idx - 40 } else { 0 };
        let mut i = log_idx;

        let mut signatures: Vec<String> = Vec::new();

        while i > scan_start && signatures.len() < topic_count {
            i -= 1;
            let opcode = bytecode[i];
            if opcode >= 0x60 && opcode <= 0x7f {
                let push_len = (opcode - 0x60 + 1) as usize;
                if i + 1 + push_len == log_idx || i + 1 + push_len < log_idx {
                    if push_len == 32 {
                        let data = &bytecode[i + 1..(i + 1 + 32).min(bytecode.len())];
                        if data.len() == 32 {
                            signatures.push(format!("0x{}", hex::encode(data)));
                        }
                    }
                }
            }
        }

        if signatures.is_empty() {
            None
        } else {
            Some(signatures.join(","))
        }
    }

    fn detect_risk_indicators(
        &self,
        bytecode: &[u8],
        storage_slots: &[StorageSlotInfo],
        _functions: &[FunctionSignature],
        call_graph: &CallGraph,
    ) -> Vec<String> {
        let mut indicators = Vec::new();

        // --- Existing opcode-level indicators ---
        if bytecode.contains(&0xf4) {
            indicators.push(
                "DELEGATECALL detected - potential proxy or vulnerable call pattern".to_string(),
            );
        }
        if bytecode.contains(&0xf1) {
            indicators.push("External CALL detected - cross-contract interaction".to_string());
        }
        if bytecode.contains(&0xff) {
            indicators.push("SELFDESTRUCT opcode detected - critical risk".to_string());
        }
        if bytecode.contains(&0xf0) || bytecode.contains(&0xf5) {
            indicators.push("Contract creation detected - dynamic deployment".to_string());
        }
        let writable_slots: Vec<_> = storage_slots.iter().filter(|s| s.is_written).collect();
        if writable_slots.len() > 10 {
            indicators.push(format!(
                "Many writable slots: {} - potential access control issues",
                writable_slots.len()
            ));
        }

        // --- New multi-contract indicators ---

        // EXTCODECOPY: dynamic code loading (proxy patterns)
        if !call_graph.extcodecopy_targets.is_empty() {
            let targets: Vec<&str> = call_graph
                .extcodecopy_targets
                .iter()
                .map(|s| s.as_str())
                .collect();
            indicators.push(format!(
                "EXTCODECOPY detected - dynamic code loading from {} targets: [{}]",
                targets.len(),
                targets.join(", ")
            ));
        }

        // Multiple delegatecall targets
        let delegate_calls: Vec<&ExternalCall> = call_graph
            .external_calls
            .iter()
            .filter(|c| c.call_type == CallType::DelegateCall)
            .collect();
        if delegate_calls.len() > 1 {
            indicators.push(format!(
                "Multiple DELEGATECALL targets ({}): potential multi-proxy pattern",
                delegate_calls.len()
            ));
        }

        // Multiple external calls to different targets
        let unique_targets: Vec<&str> = call_graph
            .external_calls
            .iter()
            .filter_map(|c| c.target.as_deref())
            .collect();
        if unique_targets.len() > 3 {
            indicators.push(format!(
                "Multiple external call targets ({}): complex interaction surface",
                unique_targets.len()
            ));
        }

        // Dynamic call targets (address from storage)
        let dynamic_calls = call_graph
            .external_calls
            .iter()
            .filter(|c| c.target.is_none())
            .count();
        if dynamic_calls > 0 {
            indicators.push(format!(
                "{} external call(s) with dynamic targets (address from storage)",
                dynamic_calls
            ));
        }

        // Event-based interaction patterns
        if call_graph.event_signatures.len() >= 2 {
            indicators.push(format!(
                "Multiple event signatures ({}) - protocol interaction detected",
                call_graph.event_signatures.len()
            ));
        }

        // Embedded address count (potential contract dependencies)
        if call_graph.embedded_addresses.len() > 5 {
            indicators.push(format!(
                "Many embedded addresses ({}): potential multi-contract dependencies",
                call_graph.embedded_addresses.len()
            ));
        }

        indicators
    }
}

impl Default for HeimdallAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_empty_bytecode() {
        let analyzer = HeimdallAnalyzer::new();
        let result = analyzer.analyze(&[], Address::ZERO);
        assert!(result.storage_slots.is_empty());
        assert!(result.function_signatures.is_empty());
        assert!(result.risk_indicators.is_empty());
        assert!(result.call_graph.external_calls.is_empty());
        assert!(result.call_graph.extcodecopy_targets.is_empty());
        assert!(result.call_graph.event_signatures.is_empty());
        assert!(result.call_graph.embedded_addresses.is_empty());
    }

    #[test]
    fn test_extract_functions_simple() {
        let analyzer = HeimdallAnalyzer::new();
        let bytecode = vec![
            0x63, 0x00, 0x00, 0x00, 0x00, 0x14, // PUSH4 0 + EQ
            0x63, 0x12, 0x34, 0x56, 0x78, 0x15, // PUSH4 0x12345678 + ISZERO
        ];
        let funcs = analyzer.extract_functions(&bytecode);
        assert!(
            !funcs.is_empty(),
            "should detect functions with EQ/ISZERO after selector"
        );
    }

    #[test]
    fn test_detect_delegatecall_risk() {
        let analyzer = HeimdallAnalyzer::new();
        let bytecode = vec![0x60, 0x01, 0xf4]; // PUSH1 + DELEGATECALL
        let result = analyzer.analyze(&bytecode, Address::ZERO);
        assert!(
            result
                .risk_indicators
                .iter()
                .any(|s| s.contains("DELEGATECALL")),
            "should detect DELEGATECALL opcode"
        );
    }

    #[test]
    fn test_detect_selfdestruct_risk() {
        let analyzer = HeimdallAnalyzer::new();
        let bytecode = vec![0x60, 0x01, 0xff]; // PUSH1 + SELFDESTRUCT
        let result = analyzer.analyze(&bytecode, Address::ZERO);
        assert!(
            result
                .risk_indicators
                .iter()
                .any(|s| s.contains("SELFDESTRUCT")),
            "should detect SELFDESTRUCT opcode"
        );
    }

    #[test]
    fn test_detect_external_call_risk() {
        let analyzer = HeimdallAnalyzer::new();
        let bytecode = vec![0x60, 0x01, 0xf1]; // PUSH1 + CALL
        let result = analyzer.analyze(&bytecode, Address::ZERO);
        assert!(
            result
                .risk_indicators
                .iter()
                .any(|s| s.contains("External CALL")),
            "should detect external call"
        );
    }

    #[test]
    fn test_detect_contract_creation_risk() {
        let analyzer = HeimdallAnalyzer::new();
        let bytecode = vec![0x60, 0x01, 0xf0, 0x60, 0x02, 0xf5];
        let result = analyzer.analyze(&bytecode, Address::ZERO);
        assert!(
            result
                .risk_indicators
                .iter()
                .any(|s| s.contains("Contract creation")),
            "should detect contract creation"
        );
    }

    #[test]
    fn test_detect_high_storage_count() {
        let analyzer = HeimdallAnalyzer::new();
        let bytecode = vec![
            0x60, 0x00, 0x55, // PUSH1 0, SSTORE
            0x60, 0x01, 0x55, // PUSH1 1, SSTORE
            0x60, 0x02, 0x55, // PUSH1 2, SSTORE
        ];
        let result = analyzer.analyze(&bytecode, Address::ZERO);
        assert!(
            !result.storage_slots.is_empty(),
            "should track storage slots"
        );
    }

    #[test]
    fn test_detect_many_writable_slots() {
        let analyzer = HeimdallAnalyzer::new();
        let bytecode = vec![
            0x60, 0x00, 0x60, 0x01, 0x55, 0x60, 0x01, 0x60, 0x02, 0x55, 0x60, 0x02, 0x60, 0x03,
            0x55, 0x60, 0x03, 0x60, 0x04, 0x55, 0x60, 0x04, 0x60, 0x05, 0x55, 0x60, 0x05, 0x60,
            0x06, 0x55, 0x60, 0x06, 0x60, 0x07, 0x55, 0x60, 0x07, 0x60, 0x08, 0x55, 0x60, 0x08,
            0x60, 0x09, 0x55, 0x60, 0x09, 0x60, 0x0a, 0x55, 0x60, 0x0a, 0x60, 0x0b, 0x55,
        ];
        let result = analyzer.analyze(&bytecode, Address::ZERO);
        let writable: Vec<_> = result
            .storage_slots
            .iter()
            .filter(|s| s.is_written)
            .collect();
        assert!(writable.len() >= 5, "should detect multiple writable slots");
    }

    #[test]
    fn test_storage_slot_tracking() {
        let analyzer = HeimdallAnalyzer::new();
        let bytecode = vec![
            0x60, 0x00, 0x54, // PUSH1 0, SLOAD
            0x60, 0x00, 0x60, 0x01, 0x55, // PUSH1 0, PUSH1 1, SSTORE
        ];
        let result = analyzer.analyze(&bytecode, Address::ZERO);
        assert!(!result.storage_slots.is_empty());
    }

    #[test]
    fn test_no_risk_indicators_for_simple_bytecode() {
        let analyzer = HeimdallAnalyzer::new();
        let bytecode = vec![
            0x60, 0x00, 0x80, 0x60, 0x01, 0x01, 0x60, 0x14, 0x56, // Simple math
        ];
        let result = analyzer.analyze(&bytecode, Address::ZERO);
        assert!(result.risk_indicators.is_empty());
    }

    #[test]
    fn test_function_signature_extraction() {
        let analyzer = HeimdallAnalyzer::new();
        let bytecode = vec![
            0x63, 0xa9, 0x05, 0x9c, 0xbb, 0x14, // transfer(address,uint256) + EQ
        ];
        let funcs = analyzer.extract_functions(&bytecode);
        assert!(!funcs.is_empty(), "should detect transfer selector");
    }

    // ── Call Graph Tests ──

    #[test]
    fn test_detect_external_calls() {
        let analyzer = HeimdallAnalyzer::new();
        let mut bytecode = Vec::new();
        // PUSH20 0x1111111111111111111111111111111111111111
        bytecode.push(0x73);
        bytecode.extend(std::iter::repeat(0x11).take(20));
        // PUSH1 0xFF for gas
        bytecode.extend_from_slice(&[0x60, 0xff]);
        // DELEGATECALL
        bytecode.push(0xf4);

        let result = analyzer.analyze(&bytecode, Address::ZERO);
        assert_eq!(result.call_graph.external_calls.len(), 1);
        assert_eq!(
            result.call_graph.external_calls[0].call_type,
            CallType::DelegateCall
        );
        assert!(result.call_graph.external_calls[0].target.is_some());
        assert!(result.call_graph.external_calls[0]
            .context
            .contains("0x1111111111111111111111111111111111111111"));
    }

    #[test]
    fn test_detect_extcodecopy() {
        let analyzer = HeimdallAnalyzer::new();
        let mut bytecode = Vec::new();
        // PUSH20 for target address
        bytecode.extend_from_slice(&[0x73]);
        bytecode.extend_from_slice(&[0x22; 20]);
        // Some PUSH values for EXTCODECOPY args
        bytecode.extend_from_slice(&[0x60, 0x00, 0x60, 0x00, 0x60, 0x00]);
        // EXTCODECOPY
        bytecode.push(0x3c);

        let result = analyzer.analyze(&bytecode, Address::ZERO);
        assert!(
            !result.call_graph.extcodecopy_targets.is_empty(),
            "should detect EXTCODECOPY"
        );
    }

    #[test]
    fn test_detect_event_signatures() {
        let analyzer = HeimdallAnalyzer::new();
        let mut bytecode = Vec::new();
        // PUSH32 topic
        bytecode.extend_from_slice(&[0x7f]);
        bytecode.extend_from_slice(&[0xaa; 32]);
        // PUSH values for offset and size
        bytecode.extend_from_slice(&[0x60, 0x00, 0x60, 0x00]);
        // LOG1
        bytecode.push(0xa1);

        let result = analyzer.analyze(&bytecode, Address::ZERO);
        assert!(
            !result.call_graph.event_signatures.is_empty(),
            "should detect event signature"
        );
    }

    #[test]
    fn test_detect_embedded_addresses() {
        let analyzer = HeimdallAnalyzer::new();
        let mut bytecode = Vec::new();
        // PUSH20
        bytecode.extend_from_slice(&[0x73]);
        bytecode.extend_from_slice(&[0x33; 20]);
        // Another PUSH20
        bytecode.extend_from_slice(&[0x73]);
        bytecode.extend_from_slice(&[0x44; 20]);

        let result = analyzer.analyze(&bytecode, Address::ZERO);
        assert_eq!(result.call_graph.embedded_addresses.len(), 2);
    }

    #[test]
    fn test_extcodecopy_risk_indicator() {
        let analyzer = HeimdallAnalyzer::new();
        let mut bytecode = Vec::new();
        bytecode.extend_from_slice(&[
            0x73, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55,
            0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55,
        ]);
        bytecode.extend_from_slice(&[0x60, 0x00, 0x60, 0x00, 0x60, 0x00]);
        bytecode.push(0x3c); // EXTCODECOPY

        let result = analyzer.analyze(&bytecode, Address::ZERO);
        assert!(
            result
                .risk_indicators
                .iter()
                .any(|s| s.contains("EXTCODECOPY")),
            "should detect EXTCODECOPY risk"
        );
    }

    #[test]
    fn test_multi_delegatecall_indicator() {
        let analyzer = HeimdallAnalyzer::new();
        let mut bytecode = Vec::new();
        // DELEGATECALL to addr 0xaa...
        bytecode.push(0x73);
        bytecode.extend(std::iter::repeat(0xaa).take(20));
        bytecode.extend_from_slice(&[0x6a, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
        bytecode.push(0xf4);
        // DELEGATECALL to addr 0xbb...
        bytecode.push(0x73);
        bytecode.extend(std::iter::repeat(0xbb).take(20));
        bytecode.extend_from_slice(&[0x6a, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
        bytecode.push(0xf4);

        let result = analyzer.analyze(&bytecode, Address::ZERO);
        assert!(
            result
                .risk_indicators
                .iter()
                .any(|s| s.contains("Multiple DELEGATECALL")),
            "should detect multiple DELEGATECALL"
        );
    }

    #[test]
    fn test_call_type_from_opcode() {
        assert_eq!(CallType::from_opcode(0xf1), Some(CallType::Call));
        assert_eq!(CallType::from_opcode(0xf4), Some(CallType::DelegateCall));
        assert_eq!(CallType::from_opcode(0xfa), Some(CallType::StaticCall));
        assert_eq!(CallType::from_opcode(0xf2), Some(CallType::CallCode));
        assert_eq!(CallType::from_opcode(0xff), None);
    }

    #[test]
    fn test_call_type_risk_label() {
        assert!(CallType::DelegateCall.risk_label().contains("DELEGATECALL"));
        assert!(CallType::Call.risk_label().contains("CALL"));
        assert!(CallType::StaticCall.risk_label().contains("STATICCALL"));
        assert!(CallType::CallCode.risk_label().contains("CALLCODE"));
    }

    #[test]
    fn test_dynamic_call_indicator() {
        let analyzer = HeimdallAnalyzer::new();
        // CALL without a PUSH20 before it (address was pushed via SLOAD or CALLDATALOAD)
        let bytecode = vec![
            0x60, 0x00, // PUSH1 0
            0x54, // SLOAD (load address from storage)
            0x60, 0x00, 0x60, 0x00, 0x60, 0x00, 0x60, 0x00, 0x60, 0x00, // dummy args
            0xf1, // CALL
        ];

        let result = analyzer.analyze(&bytecode, Address::ZERO);
        assert!(
            result
                .risk_indicators
                .iter()
                .any(|s| s.contains("dynamic targets")),
            "should detect dynamic call targets"
        );
    }
}

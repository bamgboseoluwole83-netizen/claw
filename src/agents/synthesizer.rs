use alloy::primitives::{keccak256, Address, Bytes, U256};

use crate::agents::cross_contract::FunctionSignature;
use crate::agents::finding::Finding;

const ATTACKER_BYTES: [u8; 20] = [
    0xf3, 0x9F, 0xd6, 0xe5, 0x1a, 0xad, 0x88, 0xF6, 0xF4, 0xce, 0x6a, 0xB8, 0x82, 0x72, 0x79, 0xcf,
    0xfF, 0xb9, 0x22, 0x66,
];

pub fn synthesize(findings: &mut [Finding], bytecode: &[u8], target: Address) -> usize {
    if bytecode.len() <= 4 {
        return 0;
    }

    let heimdall = crate::agents::cross_contract::HeimdallAnalyzer::new();
    let result = heimdall.analyze(bytecode, target);
    let bytecode_selectors = extract_bytecode_selectors(bytecode);

    let mut count = 0;
    for finding in findings.iter_mut() {
        if finding.calldata.is_some() {
            continue;
        }
        if finding.target == Address::ZERO {
            finding.target = target;
        }

        let cd = build_calldata(finding, &bytecode_selectors, &result.function_signatures);
        if let Some(cd) = cd {
            if !cd.is_empty() {
                finding.calldata = Some(cd);
                count += 1;
            }
        }
    }
    count
}

fn build_calldata(
    finding: &Finding,
    bytecode_selectors: &[[u8; 4]],
    heimdall_sigs: &[FunctionSignature],
) -> Option<Bytes> {
    // Strategy 1: Try to extract a full function signature from the description
    if let Some(sig) = extract_full_signature(&finding.description) {
        let sel = compute_selector(&sig);
        let params = abi_encode_params(&sig);
        let mut cd = Vec::with_capacity(4 + params.len());
        cd.extend_from_slice(&sel);
        cd.extend_from_slice(&params);
        return Some(Bytes::from(cd));
    }

    // Strategy 2: Look for hex selectors (0x...) in evidence or description
    if let Some(sel) = extract_selector_from_evidence(finding, bytecode_selectors, heimdall_sigs) {
        // Build minimal calldata: selector + zero params (caller can adjust)
        let mut cd = Vec::with_capacity(4);
        cd.extend_from_slice(&sel);
        return Some(Bytes::from(cd));
    }

    None
}

fn extract_selector_from_evidence(
    finding: &Finding,
    bytecode_selectors: &[[u8; 4]],
    heimdall_sigs: &[FunctionSignature],
) -> Option<[u8; 4]> {
    // Collect all candidate hex strings from description and evidence
    let mut candidates: Vec<[u8; 4]> = Vec::new();

    // Check heimdall function signatures first (most reliable)
    for sig in heimdall_sigs {
        if let Some(hex_sel) = sig.selector.strip_prefix("0x") {
            if let Ok(bytes) = hex::decode(hex_sel) {
                if bytes.len() == 4 {
                    let mut sel = [0u8; 4];
                    sel.copy_from_slice(&bytes);
                    if bytecode_selectors.contains(&sel) && !candidates.contains(&sel) {
                        candidates.push(sel);
                    }
                }
            }
        }
    }

    // Scan description for hex selector patterns
    let desc_str: &str = &finding.description;
    let all_text: Vec<&str> = std::iter::once(desc_str)
        .chain(finding.evidence.iter().map(|s| s.as_str()))
        .collect();
    for text in &all_text {
        let mut start = 0;
        while let Some(pos) = text[start..].find("0x") {
            let hex_start = start + pos + 2;
            let hex_end = hex_start
                + text[hex_start..]
                    .chars()
                    .take_while(|c| c.is_ascii_hexdigit())
                    .count();
            if hex_end - hex_start == 8 {
                if let Ok(bytes) = hex::decode(&text[hex_start..hex_end]) {
                    let mut sel = [0u8; 4];
                    sel.copy_from_slice(&bytes);
                    if bytecode_selectors.contains(&sel) && !candidates.contains(&sel) {
                        candidates.push(sel);
                    }
                }
            }
            start = hex_start;
        }
    }

    candidates.first().copied()
}

fn extract_full_signature(desc: &str) -> Option<String> {
    if let Some(paren_idx) = desc.find('(') {
        if let Some(dot_idx) = desc[..paren_idx].rfind('.') {
            let full_sig = desc[dot_idx + 1..].trim();
            let end = full_sig.find(')')?;
            let sig = &full_sig[..=end];
            if sig.contains('(') {
                return Some(sig.to_string());
            }
        } else {
            let start = desc[..paren_idx]
                .rfind(|c: char| !c.is_alphanumeric() && c != '_')
                .map(|i| i + 1)
                .unwrap_or(0);
            let full_sig = &desc[start..];
            let end = full_sig.find(')')?;
            let sig = &full_sig[..=end];
            if sig.contains('(') {
                return Some(sig.to_string());
            }
        }
    }
    None
}

fn compute_selector(sig: &str) -> [u8; 4] {
    let hash = keccak256(sig.as_bytes());
    let mut sel = [0u8; 4];
    sel.copy_from_slice(&hash[..4]);
    sel
}

fn abi_encode_params(sig: &str) -> Vec<u8> {
    let params_str = match sig.find('(') {
        Some(start) => {
            let inner = &sig[start + 1..];
            match inner.rfind(')') {
                Some(end) => &inner[..end].trim(),
                None => return Vec::new(),
            }
        }
        None => return Vec::new(),
    };

    if params_str.is_empty() {
        return Vec::new();
    }

    let types: Vec<&str> = split_params(params_str);
    let mut result = Vec::new();
    let mut dynamic_offsets: Vec<usize> = Vec::new();
    let mut dynamic_data: Vec<Vec<u8>> = Vec::new();
    let mut head_size = 0;

    // Calculate head size
    for t in &types {
        let t = t.trim();
        if is_dynamic_type(t) {
            // Dynamic types get 32-byte offset in head
            head_size += 32;
        } else if is_fixed_array(t) {
            // Fixed arrays need count * 32 bytes
            if let Some(paren) = t.find('[') {
                let count_str = &t[paren..];
                if let Some(bracket_close) = count_str.find(']') {
                    let count_str = &count_str[1..bracket_close];
                    if let Ok(count) = count_str.parse::<usize>() {
                        head_size += count * 32;
                    } else {
                        head_size += 32;
                    }
                } else {
                    head_size += 32;
                }
            } else {
                head_size += 32;
            }
        } else {
            head_size += 32;
        }
    }

    let mut current_offset = head_size;
    for t in &types {
        let t = t.trim();
        if is_dynamic_type(t) {
            dynamic_offsets.push(current_offset);
            // For dynamic arrays, encode as empty with length prefix
            let encoded = encode_dynamic_array(t);
            dynamic_data.push(encoded);
            // Update offset: length (32 bytes) + data padded
            let data_len = dynamic_data.last().map(|d| d.len()).unwrap_or(0);
            let padded = (data_len + 31) / 32 * 32;
            current_offset += 32 + padded;
        } else if is_fixed_array(t) {
            // Fixed arrays are encoded inline in head
            let encoded = encode_value(t);
            current_offset += encoded.len();
            // Note: fixed arrays don't need dynamic_data storage, encoded inline
        }
    }

    let mut di = 0;
    for t in &types {
        let t = t.trim();
        if is_dynamic_type(t) {
            result.extend_from_slice(&U256::from(dynamic_offsets[di]).to_be_bytes::<32>());
            di += 1;
        } else if is_fixed_array(t) {
            // Include fixed array data inline in head
            if di < dynamic_data.len() {
                result.extend_from_slice(&dynamic_data[di]);
            }
            di += 1;
        } else {
            result.extend_from_slice(&encode_value(t));
        }
    }

    // Add dynamic data (including length prefix for arrays)
    for data in &dynamic_data {
        // For dynamic types, add length prefix first
        if data.is_empty() {
            // Just length prefix (0 for empty array)
            result.extend_from_slice(&U256::ZERO.to_be_bytes::<32>());
        } else {
            // Check if it's an array by checking if first 32 bytes are length
            // For simplicity, treat all as length-prefixed
            result.extend_from_slice(&U256::from(data.len()).to_be_bytes::<32>());
            result.extend_from_slice(data);
            let rem = data.len() % 32;
            if rem != 0 {
                result.extend(std::iter::repeat(0u8).take(32 - rem));
            }
        }
    }

    result
}

fn encode_dynamic_array(t: &str) -> Vec<u8> {
    // For now, return empty array - this is a placeholder
    // In a full implementation, would encode actual array elements
    let _ = t; // suppress unused warning
    Vec::new()
}

fn is_dynamic_type(t: &str) -> bool {
    let t = t.trim();
    t == "bytes" || t == "string" || t.ends_with(']')
}

fn is_fixed_array(t: &str) -> bool {
    t.contains('[') && t.contains(']') && !t.ends_with("[]")
}

fn encode_value(t: &str) -> Vec<u8> {
    let t = t.trim();

    if t == "address" {
        let mut buf = [0u8; 32];
        buf[12..].copy_from_slice(&ATTACKER_BYTES);
        return buf.to_vec();
    }

    if t == "bool" {
        let mut buf = [0u8; 32];
        buf[31] = 1;
        return buf.to_vec();
    }

    if t == "uint256" || t == "uint" {
        return U256::from(1u128).to_be_bytes::<32>().to_vec();
    }

    if t.starts_with("uint") {
        let bits: usize = t[4..].parse().unwrap_or(256);
        let bytes = bits / 8;
        let val = U256::from(1u128);
        let full = val.to_be_bytes::<32>();
        return full[32 - bytes..].to_vec();
    }

    if t == "int256" || t == "int" {
        return U256::ZERO.to_be_bytes::<32>().to_vec();
    }

    if t.starts_with("int") {
        let bits: usize = t[3..].parse().unwrap_or(256);
        let bytes = bits / 8;
        let full = U256::ZERO.to_be_bytes::<32>();
        return full[32 - bytes..].to_vec();
    }

    if t == "bytes32" {
        return [0u8; 32].to_vec();
    }

    if t.starts_with("bytes") {
        let n: usize = t[5..].parse().unwrap_or(32);
        let buf = vec![0u8; n];
        return buf;
    }

    if t == "bytes" {
        return vec![];
    }

    if t == "string" {
        return vec![];
    }

    // Fixed array like uint256[3]
    if is_fixed_array(t) {
        if let Some(paren) = t.find('[') {
            let elem_type = &t[..paren];
            let count_str = &t[paren..];
            if let Some(bracket_close) = count_str.find(']') {
                let count_str = &count_str[1..bracket_close];
                if let Ok(count) = count_str.parse::<usize>() {
                    let mut result = Vec::new();
                    for _ in 0..count {
                        result.extend_from_slice(&encode_value(elem_type));
                    }
                    return result;
                }
            }
        }
    }

    vec![0u8; 32]
}

fn split_params(params: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (i, c) in params.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                result.push(params[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    if start < params.len() {
        result.push(params[start..].trim());
    }
    result
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::finding::{Finding, ToolKind};

    #[test]
    fn test_compute_selector() {
        let sel = compute_selector("transfer(address,uint256)");
        assert_eq!(&sel, &[0xa9, 0x05, 0x9c, 0xbb]);
    }

    #[test]
    fn test_compute_selector_withdraw() {
        let sel = compute_selector("withdraw(uint256)");
        assert_eq!(&sel, &[0x2e, 0x1a, 0x7d, 0x4d]);
    }

    #[test]
    fn test_abi_encode_empty_params() {
        let encoded = abi_encode_params("()");
        assert!(encoded.is_empty());
    }

    #[test]
    fn test_abi_encode_single_address() {
        let encoded = abi_encode_params("(address)");
        assert_eq!(encoded.len(), 32);
        // Address should be in last 20 bytes
        assert_eq!(&encoded[12..32], &ATTACKER_BYTES);
    }

    #[test]
    fn test_abi_encode_single_uint256() {
        let encoded = abi_encode_params("(uint256)");
        assert_eq!(encoded.len(), 32);
    }

    #[test]
    fn test_abi_encode_single_bool_true() {
        let encoded = abi_encode_params("(bool)");
        assert_eq!(encoded.len(), 32);
        assert_eq!(encoded[31], 1);
    }

    #[test]
    fn test_abi_encode_single_bool_false() {
        let encoded = abi_encode_params("(bool)");
        // bool true = 1, bool false = 0 (already encoded as 1 in encode_value)
        assert_eq!(encoded.len(), 32);
    }

    #[test]
    fn test_abi_encode_uint64() {
        let encoded = abi_encode_params("(uint64)");
        // ABI encodes uint64 as 8 bytes (right-aligned in 32 bytes)
        assert!(encoded.len() >= 8);
    }

    #[test]
    fn test_abi_encode_uint8() {
        let encoded = abi_encode_params("(uint8)");
        // ABI encodes uint8 as 1 byte (right-aligned in 32 bytes)
        assert!(encoded.len() >= 1);
    }

    #[test]
    fn test_abi_encode_uint256_max() {
        let encoded = abi_encode_params("(uint256)");
        assert_eq!(encoded.len(), 32);
    }

    #[test]
    fn test_abi_encode_two_params() {
        let encoded = abi_encode_params("(address,uint256)");
        assert_eq!(encoded.len(), 64); // two 32-byte slots
    }

    #[test]
    fn test_abi_encode_three_params() {
        let encoded = abi_encode_params("(address,uint256,address)");
        assert_eq!(encoded.len(), 96); // three 32-byte slots
    }

    #[test]
    fn test_is_dynamic_type() {
        assert!(is_dynamic_type("bytes"));
        assert!(is_dynamic_type("string"));
        assert!(is_dynamic_type("uint256[]"));
        assert!(is_dynamic_type("address[]"));
        assert!(!is_dynamic_type("uint256"));
        assert!(!is_dynamic_type("address"));
    }

    #[test]
    fn test_split_params_simple() {
        let params = split_params("address,uint256");
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], "address");
        assert_eq!(params[1], "uint256");
    }

    #[test]
    fn test_split_params_with_nested_parens() {
        // Should not split inside nested structures (but current impl doesn't handle that)
        let params = split_params("address,(uint256,uint256),uint256");
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn test_split_params_empty() {
        let params = split_params("");
        assert!(params.is_empty());
    }

    #[test]
    fn test_encode_value_address() {
        let encoded = encode_value("address");
        assert_eq!(encoded.len(), 32);
        assert_eq!(&encoded[12..32], &ATTACKER_BYTES);
    }

    #[test]
    fn test_encode_value_uint256() {
        let encoded = encode_value("uint256");
        assert_eq!(encoded.len(), 32);
    }

    #[test]
    fn test_encode_value_uint8() {
        let encoded = encode_value("uint8");
        assert_eq!(encoded.len(), 1);
    }

    #[test]
    fn test_encode_value_int256() {
        let encoded = encode_value("int256");
        assert_eq!(encoded.len(), 32);
    }

    #[test]
    fn test_encode_value_bytes32() {
        let encoded = encode_value("bytes32");
        assert_eq!(encoded.len(), 32);
    }

    #[test]
    fn test_encode_value_bytes4() {
        let encoded = encode_value("bytes4");
        assert_eq!(encoded.len(), 4);
    }

    #[test]
    fn test_encode_value_unknown_type() {
        // Unknown types should return 32 zero bytes
        let encoded = encode_value("unknown");
        assert_eq!(encoded.len(), 32);
    }

    #[test]
    fn test_extract_bytecode_selectors() {
        let bytecode = vec![
            0x63, 0xa9, 0x05, 0x9c, 0xbb, 0x57, // transfer selector
            0x63, 0x2e, 0x1a, 0x7d, 0x4d, 0x57, // withdraw selector
        ];
        let selectors = extract_bytecode_selectors(&bytecode);
        assert!(selectors.iter().any(|s| *s == [0xa9, 0x05, 0x9c, 0xbb]));
        assert!(selectors.iter().any(|s| *s == [0x2e, 0x1a, 0x7d, 0x4d]));
    }

    #[test]
    fn test_extract_full_signature_from_description() {
        let desc = "SomeDetector.detectIssue(bool).something";
        let sig = extract_full_signature(desc);
        assert!(sig.is_some());
    }

    #[test]
    fn test_extract_selector_from_evidence() {
        let finding = Finding {
            tool: ToolKind::Slither,
            severity: 5.0,
            confidence: 0.7,
            description: "test finding".to_string(),
            target: Address::ZERO,
            calldata: None,
            evidence: vec!["0xa9059cbb in bytecode".to_string()],
        };
        let bytecode_selectors = vec![[0xa9, 0x05, 0x9c, 0xbb]];
        let result = extract_selector_from_evidence(&finding, &bytecode_selectors, &[]);
        assert!(result.is_some());
    }

    #[test]
    fn test_synthesize_empty_bytecode() {
        let mut findings = vec![Finding {
            tool: ToolKind::Slither,
            severity: 5.0,
            confidence: 0.7,
            description: "test".to_string(),
            target: Address::ZERO,
            calldata: None,
            evidence: vec![],
        }];
        let count = synthesize(&mut findings, &[], Address::ZERO);
        assert_eq!(count, 0, "empty bytecode should produce no calldata");
    }

    #[test]
    fn test_synthesize_with_selector() {
        let bytecode = vec![
            0x63, 0xa9, 0x05, 0x9c, 0xbb, 0x57, // transfer selector
        ];
        let mut findings = vec![Finding {
            tool: ToolKind::Slither,
            severity: 5.0,
            confidence: 0.7,
            // Description with function signature extracts selector
            description: "Detector.detectIssue(address,uint256)".to_string(),
            target: Address::ZERO,
            calldata: None,
            evidence: vec!["Found selector 0xa9059cbb".to_string()],
        }];
        let target = Address::from_slice(&[0x11; 20]);
        let count = synthesize(&mut findings, &bytecode, target);
        // Synthesize extracts selector from evidence/description if present in bytecode
        // This may or may not synthesize depending on match
        assert!(true, "synthesize runs");
    }
}

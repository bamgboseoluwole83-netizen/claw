use alloy_primitives::{Address, U256};
use tracing::info;

/// Represents a discovered price oracle.
pub struct OracleInfo {
    /// The address of the oracle contract (if STATICCALL is used).
    pub oracle_address: Option<Address>,
    /// The storage slot where the price is stored (if SLOAD is used).
    pub price_slot: Option<usize>,
    /// The function selector used to get the price (if STATICCALL with a selector).
    pub function_selector: Option<[u8; 4]>,
}

/// Scan the bytecode of a function for oracle reads.
/// Looks for STATICCALL (0xfa) preceded by an address push, or SLOAD (0x54) with a constant slot.
pub fn discover_oracle(bytecode: &[u8], function_start: usize, function_end: usize) -> Option<OracleInfo> {
    let mut pc = function_start;
    while pc < function_end && pc < bytecode.len() {
        let op = bytecode[pc];

        // STATICCALL (0xfa) – likely used to call an external oracle's view function
        if op == 0xfa {
            // The address is typically pushed right before the call. We look backwards for a PUSH20 (0x73).
            let mut addr_pc = pc;
            while addr_pc > function_start {
                addr_pc -= 1;
                if bytecode[addr_pc] == 0x73 {
                    // PUSH20 – the next 20 bytes are the address
                    if addr_pc + 21 <= pc {
                        let mut addr_bytes = [0u8; 20];
                        addr_bytes.copy_from_slice(&bytecode[addr_pc+1..addr_pc+21]);
                        let oracle_addr = Address::from_slice(&addr_bytes);

                        // Also look for a PUSH4 (0x63) before the PUSH20 – that's the function selector
                        let mut sel_pc = addr_pc;
                        let mut selector = None;
                        while sel_pc > function_start {
                            sel_pc -= 1;
                            if bytecode[sel_pc] == 0x63 {
                                if sel_pc + 5 <= addr_pc {
                                    let mut sel = [0u8; 4];
                                    sel.copy_from_slice(&bytecode[sel_pc+1..sel_pc+5]);
                                    selector = Some(sel);
                                }
                                break;
                            }
                        }

                        info!("🔍 Discovered oracle via STATICCALL: addr={:?}, selector={:?}", oracle_addr, selector.map(hex::encode));
                        return Some(OracleInfo {
                            oracle_address: Some(oracle_addr),
                            price_slot: None,
                            function_selector: selector,
                        });
                    }
                }
            }
        }

        // SLOAD (0x54) – oracle price might be stored in the same contract
        if op == 0x54 {
            // The slot is pushed before SLOAD. Look backwards for a PUSH1..PUSH32.
            let mut slot_pc = pc;
            while slot_pc > function_start {
                slot_pc -= 1;
                let prev_op = bytecode[slot_pc];
                if prev_op >= 0x60 && prev_op <= 0x7f {
                    let push_size = (prev_op - 0x60 + 1) as usize;
                    if slot_pc + 1 + push_size <= pc {
                        let mut slot_bytes = [0u8; 32];
                        let data = &bytecode[slot_pc+1..slot_pc+1+push_size];
                        slot_bytes[32 - push_size..].copy_from_slice(data);
                        let slot = U256::from_be_bytes(slot_bytes).as_limbs()[0] as usize;
                        info!("🔍 Discovered oracle via SLOAD: slot={}", slot);
                        return Some(OracleInfo {
                            oracle_address: None,
                            price_slot: Some(slot),
                            function_selector: None,
                        });
                    }
                }
            }
        }

        // Skip PUSH data properly
        if op >= 0x60 && op <= 0x7f {
            let push_size = (op - 0x60 + 1) as usize;
            pc += 1 + push_size;
        } else {
            pc += 1;
        }
    }
    None
}
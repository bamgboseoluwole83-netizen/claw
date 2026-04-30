use crate::agents::abi_router::FunctionBlock;
use std::collections::HashMap;
use tracing::info;

pub fn extract_functions(bytecode: &[u8]) -> Vec<FunctionBlock> {
    let mut functions = Vec::new();
    let mut seen = HashMap::new();
    let mut pc = 0;

    while pc + 5 <= bytecode.len() {
        if bytecode[pc] == 0x63 {
            let selector = [
                bytecode[pc + 1],
                bytecode[pc + 2],
                bytecode[pc + 3],
                bytecode[pc + 4],
            ];
            if selector == [0u8; 4] {
                pc += 5;
                continue;
            }

            let mut eq_pos = pc + 5;
            let mut found_eq = false;
            while eq_pos < bytecode.len() && eq_pos <= pc + 22 {
                if bytecode[eq_pos] == 0x14 {
                    found_eq = true;
                    break;
                }
                let op = bytecode[eq_pos];
                if op >= 0x60 && op <= 0x7f {
                    eq_pos += 1 + (op - 0x60 + 1) as usize;
                } else {
                    eq_pos += 1;
                }
            }
            if !found_eq {
                pc += 5;
                continue;
            }

            let mut jumpi_pos = eq_pos + 1;
            let mut entry = None;
            while jumpi_pos + 3 < bytecode.len() && jumpi_pos <= eq_pos + 15 {
                if bytecode[jumpi_pos] == 0x61 && bytecode[jumpi_pos + 3] == 0x57 {
                    entry = Some(
                        u16::from_be_bytes([
                            bytecode[jumpi_pos + 1],
                            bytecode[jumpi_pos + 2],
                        ]) as usize,
                    );
                    break;
                }
                let op = bytecode[jumpi_pos];
                if op >= 0x60 && op <= 0x7f {
                    jumpi_pos += 1 + (op - 0x60 + 1) as usize;
                } else {
                    jumpi_pos += 1;
                }
            }

            if let Some(entry) = entry {
                seen.entry(selector).or_insert_with(|| {
                    functions.push(FunctionBlock {
                        selector,
                        start_pc: entry,
                        end_pc: 0,
                    });
                });
            }
            pc += 5;
        } else if bytecode[pc] >= 0x60 && bytecode[pc] <= 0x7f {
            let n = (bytecode[pc] - 0x60 + 1) as usize;
            pc += 1 + n;
        } else {
            pc += 1;
        }
    }

    functions.sort_by_key(|f| f.start_pc);
    for i in 0..functions.len() {
        let next_start = if i + 1 < functions.len() {
            functions[i + 1].start_pc
        } else {
            bytecode.len()
        };
        functions[i].end_pc = next_start;
    }
    functions.retain(|f| f.end_pc > f.start_pc + 10);

    info!(
        target: "extractor",
        "🔍 Extracted {} functions with entry points",
        functions.len()
    );
    functions
}
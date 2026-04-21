use eyre::Result;

/// The parsed blueprint of a smart contract.
#[derive(Debug, Default)]
pub struct DisassemblyResult {
    pub sstore_offsets: Vec<usize>,
    pub external_call_offsets: Vec<usize>,
    pub jumpdest_offsets: Vec<usize>,
}

/// Raw, zero-dependency EVM bytecode parser.
/// Parses opcodes as raw u8 integers, exactly how the EVM silicon reads them.
pub fn disassemble(bytecode: &[u8]) -> Result<DisassemblyResult> {
    let mut result = DisassemblyResult::default();
    let mut pc = 0; // Program Counter (byte offset)

    while pc < bytecode.len() {
        let op = bytecode[pc];

        match op {
            // 0x55 = SSTORE
            0x55 => result.sstore_offsets.push(pc),
            
            // 0xF1 = CALL, 0xF2 = CALLCODE, 0xF4 = DELEGATECALL, 0xFA = STATICCALL
            0xf1 | 0xf2 | 0xf4 | 0xfa => result.external_call_offsets.push(pc),
            
            // 0x5B = JUMPDEST
            0x5b => result.jumpdest_offsets.push(pc),

            // 0x60 to 0x7F = PUSH1 through PUSH32
            0x60..=0x7f => {
                // Calculate how many data bytes to skip so our PC stays accurate.
                // PUSH1 (0x60) pushes 1 byte. PUSH32 (0x7F) pushes 32 bytes.
                let push_size = (op - 0x60 + 1) as usize;
                pc += push_size; 
            }

            _ => {} // Ignore arithmetic, logic, and memory opcodes
        }
        pc += 1; // Move to the next opcode
    }

    Ok(result)
}

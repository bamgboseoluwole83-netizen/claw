use eyre::Result;

/// The parsed blueprint of a smart contract.
#[derive(Debug, Default)]
pub struct DisassemblyResult {
    pub sstore_offsets: Vec<usize>,
    pub sload_offsets: Vec<usize>,
    pub external_call_offsets: Vec<usize>,
    pub jumpdest_offsets: Vec<usize>,
    pub math_offsets: Vec<usize>,      
    pub jumpi_offsets: Vec<usize>,     // Conditional jumps (If/Else gates)
    
    // NEW FOR THE BRAIN UPGRADE: Track the actual jump instructions themselves
    pub unconditional_jumps: Vec<usize>, // JUMP opcodes (breaks linear flow)
    pub conditional_jumps: Vec<usize>,   // JUMPI opcodes (splits flow)
}

/// Raw, zero-dependency EVM bytecode parser.
pub fn disassemble(bytecode: &[u8]) -> Result<DisassemblyResult> {
    let mut result = DisassemblyResult::default();
    let mut pc = 0; 

    while pc < bytecode.len() {
        let op = bytecode[pc];

        match op {
            0x55 => result.sstore_offsets.push(pc),
            0x54 => result.sload_offsets.push(pc), 
            0xf1 | 0xf2 | 0xf4 | 0xfa => result.external_call_offsets.push(pc),
            0x5b => result.jumpdest_offsets.push(pc),
            0x57 => {
                result.jumpi_offsets.push(pc);
                result.conditional_jumps.push(pc); // NEW
            },
            0x56 => {
                result.unconditional_jumps.push(pc); // NEW
            },
            0x04 | 0x05 | 0x06 => result.math_offsets.push(pc), 

            0x60..=0x7f => {
                let push_size = (op - 0x60 + 1) as usize;
                pc += push_size; 
            }
            _ => {} 
        }
        pc += 1; 
    }

    Ok(result)
}
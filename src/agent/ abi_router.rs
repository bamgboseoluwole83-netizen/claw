/// Represents a public function discovered in bytecode.
#[derive(Debug, Clone, Copy, Default)]
pub struct FunctionBlock {
    pub selector: [u8; 4],
    pub start_pc: usize,
    pub end_pc: usize,
}
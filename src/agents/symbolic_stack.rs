use alloy_primitives::U256;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArithmeticOp { Add, Sub, Mul, Div, Mod }

#[derive(Debug, Clone)]
pub enum SymbolicValue {
    Constant(U256),
    Sload(U256),
    Arithmetic { op: ArithmeticOp, left: Box<SymbolicValue>, right: Box<SymbolicValue> },
    Calldata { offset: usize },
    Unknown,
}

impl SymbolicValue {
    pub fn contains_div(&self) -> bool {
        match self {
            SymbolicValue::Arithmetic { op: ArithmeticOp::Div, .. } => true,
            SymbolicValue::Arithmetic { left, right, .. } => left.contains_div() || right.contains_div(),
            _ => false,
        }
    }
    pub fn has_calldata(&self) -> bool {
        match self {
            SymbolicValue::Calldata { .. } => true,
            SymbolicValue::Arithmetic { left, right, .. } => left.has_calldata() || right.has_calldata(),
            _ => false,
        }
    }
    pub fn has_sload(&self) -> bool {
        match self {
            SymbolicValue::Sload(_) => true,
            SymbolicValue::Arithmetic { left, right, .. } => left.has_sload() || right.has_sload(),
            _ => false,
        }
    }
    pub fn extract_divisor(&self) -> Option<U256> {
        if let SymbolicValue::Arithmetic { op: ArithmeticOp::Div, right, .. } = self {
            if let SymbolicValue::Constant(c) = **right { return Some(c); }
        }
        None
    }
    pub fn extract_multiplier(&self) -> Option<U256> {
        if let SymbolicValue::Arithmetic { op: ArithmeticOp::Mul, left, right } = self {
            if left.has_calldata() && !right.has_calldata() {
                if let SymbolicValue::Constant(c) = **right { return Some(c); }
                if let SymbolicValue::Sload(c) = **right { return Some(c); }
            }
            if right.has_calldata() && !left.has_calldata() {
                if let SymbolicValue::Constant(c) = **left { return Some(c); }
                if let SymbolicValue::Sload(c) = **left { return Some(c); }
            }
        }
        None
    }
}

#[derive(Clone)]
pub struct SymbolicStack { stack: Vec<SymbolicValue> }

impl SymbolicStack {
    pub fn new() -> Self { Self { stack: Vec::new() } }
    pub fn push(&mut self, val: SymbolicValue) { self.stack.push(val); }
    pub fn pop(&mut self) -> Option<SymbolicValue> { self.stack.pop() }
    pub fn peek(&self) -> Option<&SymbolicValue> { self.stack.last() }
    pub fn dup(&mut self, n: usize) { if self.stack.len() >= n { let val = self.stack[self.stack.len()-n].clone(); self.push(val); } }
    pub fn swap(&mut self, n: usize) { let len = self.stack.len(); if len > n { let top = len-1; let swap_idx = len-1-n; self.stack.swap(top, swap_idx); } }
}

use alloy::primitives::Address;
use std::str::FromStr;
fn main() {
    let s = "0x5FbDB2315678afecb367f032d93F642f64180aa3";
    println!("Input: '{}' len={}", s, s.len());
    match Address::from_str(s) {
        Ok(addr) => println!("OK: {:?}", addr),
        Err(e) => println!("PARSE ERROR: {}", e),
    }
    let s2 = "0x5fbdb2315678afecb367f032d93f642f64180aa3";
    println!("Input2: '{}' len={}", s2, s2.len());
    match Address::from_str(s2) {
        Ok(addr) => println!("OK: {:?}", addr),
        Err(e) => println!("PARSE ERROR: {}", e),
    }
}

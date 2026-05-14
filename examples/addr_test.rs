use alloy::primitives::Address;
use std::str::FromStr;
fn main() {
    let addr = Address::from_str("0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9").unwrap();
    println!("Debug: '{:?}'", addr);
    println!("LowerHex: '{:#x}'", addr);
}

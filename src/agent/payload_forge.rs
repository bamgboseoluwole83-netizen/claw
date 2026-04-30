use alloy_primitives::{U256, Address};
use tracing::info;

pub const SELECTOR_HITLIST: [[u8; 4]; 5] = [
    [0xd0, 0xe3, 0x0d, 0xb0], // deposit()
    [0x2e, 0x1a, 0x7d, 0x4d], // withdraw(uint256)
    [0x38, 0xed, 0x17, 0x39], // swap
    [0x85, 0x2a, 0x12, 0xe3], // flashLoan
    [0xf2, 0x4f, 0x5d, 0xd6], // exactInputSingle
];

pub struct PayloadForge;

impl PayloadForge {
    pub fn forge_assault_vectors(target_address: Address) -> Vec<Vec<u8>> {
        let mut payloads = Vec::new();
        let attack_amount = U256::from(1000u128 * 10u128.pow(18));
        let attack_amount_bytes = attack_amount.to_be_bytes::<32>();

        for selector in &SELECTOR_HITLIST {
            let mut stack_payload = Vec::with_capacity(36);
            stack_payload.extend_from_slice(selector);
            stack_payload.extend_from_slice(&attack_amount_bytes);
            info!(target: "forge", "Forged calldata for {:?}", target_address);
            payloads.push(stack_payload);
        }
        payloads
    }
}
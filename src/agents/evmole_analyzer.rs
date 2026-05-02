use alloy_primitives::U256;

/// Stub – evmole 0.8.4 lacks variable names in storage layout.
/// The controller uses `target.price_slot` from JSON as fallback.
pub fn discover_price_slot(_code: &[u8]) -> Option<U256> {
    None
}

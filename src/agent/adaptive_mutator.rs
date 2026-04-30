use alloy_primitives::U256;
use tracing::warn;

pub struct AdaptiveMutator;

impl AdaptiveMutator {
    /// The Feedback Loop: If chaos caused a revert, try inverting the mask.
    /// E.g., If +1 wei causes an underflow, maybe -1 wei causes a precision surplus.
    pub fn calculate_retry_mask(original_mask: U256, honest_success: bool, chaos_success: bool) -> Option<U256> {
        if honest_success && !chaos_success {
            // The chaos broke the TX logic. Invert the bits and try again.
            let inverted_mask = !original_mask; // Bitwise NOT
            warn!(target: "mutator", "⚠️ CHAOS REVERT DETECTED. Inverting mask to {:?} for retry.", inverted_mask);
            return Some(inverted_mask);
        }
        None
    }
}
//! Anchor discriminator computation.
//!
//! Anchor derives 8-byte instruction and account tags by taking the first 8
//! bytes of `SHA-256` over a fixed-prefix preimage:
//!
//! - Instruction: `sha256("global:<name>")[..8]`
//! - Account: `sha256("account:<name>")[..8]`
//!
//! The well-known vector `sha256("global:initialize")[..8] = afaf6d1f0d989bed`
//! is asserted in this module's unit test as a regression guard against
//! breakage of the SHA-256 dependency or the prefix scheme.

use sha2::{Digest, Sha256};

/// Compute the 8-byte Anchor instruction discriminator for `name`.
///
/// Equivalent to `sha256("global:<name>")[..8]`.
pub fn instruction_discriminator(name: &str) -> [u8; 8] {
    discriminator_from_preimage(&format!("global:{}", name))
}

/// Compute the 8-byte Anchor account discriminator for `name`.
///
/// Equivalent to `sha256("account:<name>")[..8]`.
pub fn account_discriminator(name: &str) -> [u8; 8] {
    discriminator_from_preimage(&format!("account:{}", name))
}

fn discriminator_from_preimage(preimage: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(preimage.as_bytes());
    let result = hasher.finalize();
    let mut out = [0u8; 8];
    out.copy_from_slice(&result[..8]);
    out
}

/// Format a byte slice as a lowercase hexadecimal string with no separator.
pub fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_instruction_discriminator_matches_known_vector() {
        // sha256("global:initialize")[..8] is a canonical Anchor vector.
        let d = instruction_discriminator("initialize");
        assert_eq!(hex(&d), "afaf6d1f0d989bed");
    }

    #[test]
    fn account_discriminator_changes_with_name() {
        assert_ne!(
            account_discriminator("Pool"),
            account_discriminator("PoolV2")
        );
    }
}

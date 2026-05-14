use sha2::{Digest, Sha256};

pub fn instruction_discriminator(name: &str) -> [u8; 8] {
    discriminator_from_preimage(&format!("global:{}", name))
}

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

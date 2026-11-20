//! Proof of Work — used here as the *admission* gate into the PoA validator set.
//!
//! Block production is cheap (PoA). Joining is expensive (PoW). That means the
//! network stays fast while Sybil resistance is still paid for in joules.

use sha2::{Digest, Sha256};

use crate::types::Hash;

/// Leading zero *bits* required. 20 bits ~= 1M attempts on average.
pub const MAX_DIFFICULTY: u32 = 255;

/// Hash(seed || nonce).
#[inline(always)]
pub fn hash_with_nonce(seed: &Hash, nonce: u64) -> Hash {
    let mut h = Sha256::new();
    h.update(seed);
    h.update(nonce.to_le_bytes());
    h.finalize().into()
}

/// True when `hash` has at least `difficulty` leading zero bits.
#[inline]
pub fn meets_difficulty(hash: &Hash, difficulty: u32) -> bool {
    if difficulty == 0 {
        return true;
    }
    let mut bits = 0u32;
    for byte in hash.iter() {
        if *byte == 0 {
            bits += 8;
        } else {
            bits += byte.leading_zeros();
            break;
        }
        if bits >= difficulty {
            return true;
        }
    }
    bits >= difficulty
}

/// Brute-force a nonce satisfying `difficulty`. Returns (nonce, hash).
pub fn mine(seed: &Hash, difficulty: u32) -> (u64, Hash) {
    let mut nonce = 0u64;
    loop {
        let h = hash_with_nonce(seed, nonce);
        if meets_difficulty(&h, difficulty) {
            return (nonce, h);
        }
        nonce = nonce.wrapping_add(1);
    }
}

/// Cheap check for anyone receiving an admission proof.
#[inline]
pub fn verify(seed: &Hash, nonce: u64, difficulty: u32) -> bool {
    meets_difficulty(&hash_with_nonce(seed, nonce), difficulty)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_difficulty_always_passes() {
        assert!(meets_difficulty(&[0xff; 32], 0));
    }

    #[test]
    fn mine_then_verify() {
        let seed = [3u8; 32];
        let (nonce, hash) = mine(&seed, 12);
        assert!(verify(&seed, nonce, 12));
        assert!(meets_difficulty(&hash, 12));
    }

    #[test]
    fn wrong_nonce_fails_usually() {
        let seed = [3u8; 32];
        let (nonce, _) = mine(&seed, 16);
        // The nonce one less should (almost certainly) not satisfy 16 bits.
        if nonce > 0 {
            assert!(!verify(&seed, nonce - 1, 16));
        }
    }
}

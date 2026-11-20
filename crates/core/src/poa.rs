//! Proof of Authority — fast, deterministic finality.
//!
//! A fixed set of validators takes turns producing blocks (round-robin leader
//! schedule). A block is final once >= 2/3 of the set has signed it.
//!
//! Membership is gated by PoW admission (see `pow.rs`), so "authority" is
//! earned, not granted — that is the hybrid.

use ed25519_dalek::{Signature, Verifier, VerifyingKey};

/// An ordered validator set with a 2/3 quorum.
#[derive(Clone)]
pub struct ValidatorSet {
    validators: Vec<VerifyingKey>,
    quorum: usize,
}

impl ValidatorSet {
    /// Builds the set and computes quorum = floor(2n/3) + 1.
    /// Keys are sorted so the leader schedule is deterministic across nodes.
    pub fn new(mut validators: Vec<VerifyingKey>) -> Self {
        validators.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        let quorum = if validators.is_empty() {
            0
        } else {
            validators.len() * 2 / 3 + 1
        };
        Self { validators, quorum }
    }

    pub fn len(&self) -> usize {
        self.validators.len()
    }

    pub fn is_empty(&self) -> bool {
        self.validators.is_empty()
    }

    pub fn quorum(&self) -> usize {
        self.quorum
    }

    pub fn contains(&self, key: &VerifyingKey) -> bool {
        self.validators
            .binary_search_by(|v| v.as_bytes().cmp(key.as_bytes()))
            .is_ok()
    }

    /// Round-robin leader for a slot. Every node computes the same answer.
    pub fn leader_for_slot(&self, slot: u64) -> Option<&VerifyingKey> {
        if self.validators.is_empty() {
            return None;
        }
        Some(&self.validators[(slot as usize) % self.validators.len()])
    }

    /// Number of *distinct* validators that validly signed `msg`.
    pub fn count_signatures(&self, msg: &[u8], sigs: &[[u8; 64]]) -> usize {
        let mut seen = vec![false; self.validators.len()];
        for sig_bytes in sigs {
            let sig = Signature::from_bytes(sig_bytes);
            for (i, vk) in self.validators.iter().enumerate() {
                if !seen[i] && vk.verify(msg, &sig).is_ok() {
                    seen[i] = true;
                }
            }
        }
        seen.iter().filter(|s| **s).count()
    }

    /// Finality check.
    pub fn has_quorum(&self, msg: &[u8], sigs: &[[u8; 64]]) -> bool {
        self.count_signatures(msg, sigs) >= self.quorum
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn keys(n: usize) -> Vec<SigningKey> {
        (0..n)
            .map(|i| SigningKey::from_bytes(&[i as u8 + 1; 32]))
            .collect()
    }

    #[test]
    fn quorum_is_two_thirds_plus_one() {
        let set = ValidatorSet::new(keys(4).iter().map(|k| k.verifying_key()).collect());
        assert_eq!(set.quorum(), 3);
        let set = ValidatorSet::new(keys(3).iter().map(|k| k.verifying_key()).collect());
        assert_eq!(set.quorum(), 3);
        let set = ValidatorSet::new(keys(7).iter().map(|k| k.verifying_key()).collect());
        assert_eq!(set.quorum(), 5);
    }

    #[test]
    fn leader_schedule_is_round_robin() {
        let ks = keys(3);
        let set = ValidatorSet::new(ks.iter().map(|k| k.verifying_key()).collect());
        let a = set.leader_for_slot(0).unwrap().to_bytes();
        let b = set.leader_for_slot(1).unwrap().to_bytes();
        let c = set.leader_for_slot(3).unwrap().to_bytes();
        assert_ne!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn quorum_requires_enough_signers() {
        let ks = keys(4);
        let set = ValidatorSet::new(ks.iter().map(|k| k.verifying_key()).collect());
        let msg = b"block-hash";
        let two: Vec<[u8; 64]> = ks[0..2].iter().map(|k| k.sign(msg).to_bytes()).collect();
        assert!(!set.has_quorum(msg, &two));

        let three: Vec<[u8; 64]> = ks[0..3].iter().map(|k| k.sign(msg).to_bytes()).collect();
        assert!(set.has_quorum(msg, &three));
    }

    #[test]
    fn duplicate_signatures_do_not_count_twice() {
        let ks = keys(4);
        let set = ValidatorSet::new(ks.iter().map(|k| k.verifying_key()).collect());
        let msg = b"x";
        let one = ks[0].sign(msg).to_bytes();
        let dupes = vec![one; 3];
        assert_eq!(set.count_signatures(msg, &dupes), 1);
    }
}

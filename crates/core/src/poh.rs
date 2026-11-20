//! Proof of History.
//!
//! A PoH sequence is a chain of SHA-256 hashes. Because each link requires the
//! previous one, the sequence *cannot* be computed in parallel — the length of
//! the chain is a cryptographic proof that time has elapsed.
//!
//! It is **not** a consensus mechanism. It gives the network a shared clock and
//! a total order for events. PoA decides which blocks are final.

use sha2::{Digest, Sha256};

use crate::types::Hash;

/// Appends to a running PoH sequence.
#[derive(Clone, Debug)]
pub struct PohRecorder {
    hash: Hash,
    ticks: u64,
}

impl PohRecorder {
    pub fn new(seed: Hash) -> Self {
        Self {
            hash: seed,
            ticks: 0,
        }
    }

    /// Current chain head.
    pub fn hash(&self) -> Hash {
        self.hash
    }

    /// How many steps have been taken (a proxy for elapsed time).
    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    /// Advance the clock with no payload.
    pub fn tick(&mut self) -> Hash {
        self.hash = hash_pair(&self.hash, &[]);
        self.ticks += 1;
        self.hash
    }

    /// Advance the clock and mix `data` (tx hashes, slot roots) into it.
    pub fn record(&mut self, data: &[u8]) -> Hash {
        self.hash = hash_pair(&self.hash, data);
        self.ticks += 1;
        self.hash
    }
}

/// One step: SHA256(prev || data).
#[inline(always)]
fn hash_pair(prev: &Hash, data: &[u8]) -> Hash {
    let mut h = Sha256::new();
    h.update(prev);
    h.update(data);
    h.finalize().into()
}

/// Replay a sequence from a seed. `None` means a bare tick, `Some(d)` means a
/// recorded entry. Returns the final hash.
///
/// This is how a light client verifies a PoH proof without trusting the node:
/// it re-hashes the sequence and checks it lands on the claimed value.
pub fn replay(seed: Hash, entries: &[Option<&[u8]>]) -> Hash {
    let mut cur = seed;
    for entry in entries {
        cur = match entry {
            Some(data) => hash_pair(&cur, data),
            None => hash_pair(&cur, &[]),
        };
    }
    cur
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_are_deterministic() {
        let mut a = PohRecorder::new([0u8; 32]);
        let mut b = PohRecorder::new([0u8; 32]);
        for _ in 0..100 {
            assert_eq!(a.tick(), b.tick());
        }
        assert_eq!(a.ticks(), 100);
    }

    #[test]
    fn replay_matches_recorder() {
        let seed = [9u8; 32];
        let mut r = PohRecorder::new(seed);
        r.tick();
        r.record(b"tx-root");
        r.tick();

        let replayed = replay(seed, &[None, Some(b"tx-root".as_slice()), None]);
        assert_eq!(r.hash(), replayed);
    }

    #[test]
    fn order_matters() {
        let seed = [0u8; 32];
        let a = replay(seed, &[Some(b"a"), Some(b"b")]);
        let b = replay(seed, &[Some(b"b"), Some(b"a")]);
        assert_ne!(a, b);
    }
}

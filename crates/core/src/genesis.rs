//! Genesis: the one file every node must agree on.
//!
//! A devnet lives or dies on this being portable. It is plain JSON so anyone
//! can read it, diff it, and fork it.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::types::Hash;

/// Smallest unit. 1 CHRONYX = 1_000_000_000 quanta.
pub const QUANTA_PER_CHRONYX: u64 = 1_000_000_000;

/// Fixed supply: 600,000,000 CHRONYX.
///
/// Solana currently runs ~638M total and ~592M circulating, so 600M is a
/// number people already have a feel for. v1 has no inflation and no runtime
/// minting, so this is a hard cap: every unit exists in genesis or not at all.
pub const TOTAL_SUPPLY_CHRONYX: u64 = 600_000_000;

/// Bootstrap balance each genesis validator starts with: 25M CHRONYX.
/// The rest of the supply sits in the devnet faucet.
pub const VALIDATOR_ALLOCATION_CHRONYX: u64 = 25_000_000;

/// Devnet faucet seed. Deterministic so every node can derive it.
pub const FAUCET_SEED: [u8; 32] = [0xFA; 32];

/// Deterministic dev validator seed for index `i` (0-based).
pub fn dev_validator_seed(i: usize) -> [u8; 32] {
    [(i as u8).wrapping_add(1); 32]
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenesisValidator {
    /// Ed25519 identity pubkey, hex encoded.
    pub identity: String,
    pub name: String,
    pub commission_bps: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenesisAllocation {
    /// Ed25519 pubkey, hex encoded.
    pub pubkey: String,
    pub quanta: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Genesis {
    pub chain_id: String,
    pub genesis_time_ms: u64,
    pub slot_ms: u64,
    pub ticks_per_slot: u64,
    pub pow_difficulty: u32,
    /// Pubkey that funds airdrops on devnet. `None` disables the faucet.
    pub faucet: Option<String>,
    pub validators: Vec<GenesisValidator>,
    pub allocations: Vec<GenesisAllocation>,
}

impl Genesis {
    /// A ready-to-run devnet with `validator_count` deterministic validators and
    /// a funded faucet.
    pub fn devnet(validator_count: usize) -> Self {
        use ed25519_dalek::SigningKey;

        let count = validator_count.max(1);
        let mut validators = Vec::with_capacity(count);
        let mut allocations = Vec::new();

        // Keep the arithmetic exact for any validator count: split the fixed
        // supply evenly across validators + the faucet, and let the faucet
        // absorb the rounding remainder.
        let per_validator =
            VALIDATOR_ALLOCATION_CHRONYX.min(TOTAL_SUPPLY_CHRONYX / (count as u64 + 1));

        for i in 0..count {
            let sk = SigningKey::from_bytes(&dev_validator_seed(i));
            let pk = hex::encode(sk.verifying_key().to_bytes());
            validators.push(GenesisValidator {
                identity: pk.clone(),
                name: format!("dev-{}", i + 1),
                commission_bps: 500,
            });
            allocations.push(GenesisAllocation {
                pubkey: pk,
                quanta: per_validator * QUANTA_PER_CHRONYX,
            });
        }

        let faucet_key = SigningKey::from_bytes(&FAUCET_SEED);
        let faucet = hex::encode(faucet_key.verifying_key().to_bytes());
        allocations.push(GenesisAllocation {
            pubkey: faucet.clone(),
            quanta: (TOTAL_SUPPLY_CHRONYX - per_validator * count as u64) * QUANTA_PER_CHRONYX,
        });

        Self {
            chain_id: "chronyx-devnet-1".to_string(),
            genesis_time_ms: now_ms(),
            slot_ms: 400,
            ticks_per_slot: 8,
            pow_difficulty: 16,
            faucet: Some(faucet),
            validators,
            allocations,
        }
    }

    /// Canonical hash. Field order is declaration order, so this is stable.
    pub fn hash(&self) -> Hash {
        let bytes = serde_json::to_vec(self).expect("genesis serializes");
        let mut h = Sha256::new();
        h.update(&bytes);
        h.finalize().into()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.chain_id.trim().is_empty() {
            return Err("chain_id must not be empty".into());
        }
        if self.slot_ms == 0 {
            return Err("slot_ms must be > 0".into());
        }
        if self.ticks_per_slot == 0 {
            return Err("ticks_per_slot must be > 0".into());
        }
        if self.validators.is_empty() {
            return Err("at least one validator is required".into());
        }
        for v in &self.validators {
            parse_key(&v.identity)?;
            if v.commission_bps > 10_000 {
                return Err(format!("validator {} commission > 100%", v.name));
            }
        }
        for a in &self.allocations {
            parse_key(&a.pubkey)?;
        }
        if let Some(f) = &self.faucet {
            parse_key(f)?;
        }
        Ok(())
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("genesis serializes")
    }

    pub fn from_json(s: &str) -> Result<Self, String> {
        serde_json::from_str(s).map_err(|e| e.to_string())
    }

    /// Sum of every genesis allocation, in quanta. Equal to the fixed supply.
    pub fn total_quanta(&self) -> u64 {
        self.allocations.iter().map(|a| a.quanta).sum()
    }

    pub fn validator_identities(&self) -> Result<Vec<[u8; 32]>, String> {
        self.validators
            .iter()
            .map(|v| parse_key(&v.identity))
            .collect()
    }
}

pub fn parse_key(s: &str) -> Result<[u8; 32], String> {
    let bytes = hex::decode(s).map_err(|e| format!("bad hex pubkey: {e}"))?;
    bytes
        .try_into()
        .map_err(|_| "pubkey must be exactly 32 bytes".to_string())
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devnet_is_valid_and_stable() {
        let a = Genesis::devnet(3);
        let b = Genesis::devnet(3);
        a.validate().unwrap();
        assert_eq!(a.validators.len(), 3);
        assert_eq!(a.validator_identities().unwrap().len(), 3);
        // Same validators => same hash (time differs, so compare that field only)
        assert_eq!(a.validators, b.validators);
        assert_eq!(a.faucet, b.faucet);
    }

    #[test]
    fn roundtrips_through_json() {
        let g = Genesis::devnet(2);
        let back = Genesis::from_json(&g.to_json()).unwrap();
        assert_eq!(g, back);
    }

    #[test]
    fn devnet_supply_is_fixed() {
        for n in [1usize, 3, 7, 23] {
            let g = Genesis::devnet(n);
            assert_eq!(
                g.total_quanta(),
                TOTAL_SUPPLY_CHRONYX * QUANTA_PER_CHRONYX,
                "supply drifted for {n} validators"
            );
            assert!(g.allocations.iter().all(|a| a.quanta > 0));
        }
    }

    #[test]
    fn rejects_empty_validators() {
        let mut g = Genesis::devnet(1);
        g.validators.clear();
        assert!(g.validate().is_err());
    }
}

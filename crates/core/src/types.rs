use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A 32-byte digest. Used for block hashes, PoH hashes, merkle roots.
pub type Hash = [u8; 32];

/// A signed value transfer. `from` is an Ed25519 public key.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Transaction {
    pub from: [u8; 32],
    pub to: [u8; 32],
    pub amount: u64,
    /// Monotonic per-sender counter; prevents replay.
    pub nonce: u64,
    pub signature: Signature,
}

impl Transaction {
    /// Canonical bytes that are actually signed (everything except the signature).
    pub fn signing_payload(&self) -> [u8; 112] {
        let mut buf = [0u8; 112];
        buf[0..32].copy_from_slice(&self.from);
        buf[32..64].copy_from_slice(&self.to);
        buf[64..72].copy_from_slice(&self.amount.to_le_bytes());
        buf[72..80].copy_from_slice(&self.nonce.to_le_bytes());
        buf
    }

    pub fn hash(&self) -> Hash {
        let mut h = Sha256::new();
        h.update(self.signing_payload());
        h.update(self.signature.to_bytes());
        h.finalize().into()
    }

    /// Build and sign a transfer with `sk`. Nonce must match the sender's
    /// current account nonce.
    pub fn sign(sk: &SigningKey, to: [u8; 32], amount: u64, nonce: u64) -> Self {
        let mut tx = Self {
            from: sk.verifying_key().to_bytes(),
            to,
            amount,
            nonce,
            signature: Signature::from_bytes(&[0u8; 64]),
        };
        tx.signature = sk.sign(&tx.signing_payload());
        tx
    }

    /// Verify the Ed25519 signature against `from`.
    pub fn verify(&self) -> Result<(), TxError> {
        let vk = VerifyingKey::from_bytes(&self.from).map_err(|_| TxError::BadKey)?;
        vk.verify(&self.signing_payload(), &self.signature)
            .map_err(|_| TxError::BadSignature)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TxError {
    #[error("invalid sender public key")]
    BadKey,
    #[error("invalid signature")]
    BadSignature,
    #[error("bad nonce: expected {expected}, got {got}")]
    BadNonce { expected: u64, got: u64 },
    #[error("insufficient funds")]
    InsufficientFunds,
    #[error("amount overflow")]
    Overflow,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockHeader {
    pub slot: u64,
    pub parent: Hash,
    /// PoH chain value at the moment this block was sealed.
    pub poh_hash: Hash,
    /// Merkle root over `transactions`.
    pub tx_root: Hash,
    /// Ed25519 pubkey of the scheduled leader.
    pub leader: [u8; 32],
    pub timestamp_ms: u64,
}

impl BlockHeader {
    /// Hash the header field-by-field. No serialization, no allocation.
    pub fn hash(&self) -> Hash {
        let mut h = Sha256::new();
        h.update(self.slot.to_le_bytes());
        h.update(self.parent);
        h.update(self.poh_hash);
        h.update(self.tx_root);
        h.update(self.leader);
        h.update(self.timestamp_ms.to_le_bytes());
        h.finalize().into()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
    /// PoA finality votes. Valid when >= 2/3 of the validator set signed.
    pub signatures: Vec<Signature>,
}

impl Block {
    pub fn hash(&self) -> Hash {
        self.header.hash()
    }

    /// Signing payload for PoA votes: the header hash.
    pub fn vote_payload(&self) -> Hash {
        self.header.hash()
    }
}

/// Simple binary merkle root over transaction hashes.
/// Duplicates the last node on odd counts (Bitcoin-style).
pub fn merkle_root(txs: &[Transaction]) -> Hash {
    if txs.is_empty() {
        return [0u8; 32];
    }

    let mut layer: Vec<Hash> = txs.iter().map(|t| t.hash()).collect();

    while layer.len() > 1 {
        if layer.len() % 2 == 1 {
            let last = *layer.last().expect("non-empty");
            layer.push(last);
        }
        layer = layer
            .chunks(2)
            .map(|pair| {
                let mut h = Sha256::new();
                h.update(pair[0]);
                h.update(pair[1]);
                h.finalize().into()
            })
            .collect();
    }

    layer[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tx(nonce: u64) -> Transaction {
        Transaction {
            from: [1u8; 32],
            to: [2u8; 32],
            amount: 10,
            nonce,
            signature: Signature::from_bytes(&[0u8; 64]),
        }
    }

    #[test]
    fn empty_merkle_root_is_zero() {
        assert_eq!(merkle_root(&[]), [0u8; 32]);
    }

    #[test]
    fn merkle_root_changes_with_content() {
        let a = merkle_root(&[tx(1), tx(2)]);
        let b = merkle_root(&[tx(1), tx(3)]);
        assert_ne!(a, b);
    }

    #[test]
    fn single_tx_root_equals_tx_hash() {
        let t = tx(1);
        assert_eq!(merkle_root(std::slice::from_ref(&t)), t.hash());
    }
}

//! Native account state.
//!
//! This is the "token" from scratch: no contracts, no ERC-20, no VM. An account
//! is a balance plus a nonce. A transfer is a signed message that moves quanta
//! and bumps the nonce. That's the whole model — deliberately.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::genesis::Genesis;
use crate::types::{Transaction, TxError};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Account {
    pub quanta: u64,
    pub nonce: u64,
}

#[derive(Clone, Debug, Default)]
pub struct State {
    accounts: BTreeMap<[u8; 32], Account>,
}

impl State {
    pub fn from_genesis(genesis: &Genesis) -> Result<Self, String> {
        let mut state = Self::default();
        for allocation in &genesis.allocations {
            let key = crate::genesis::parse_key(&allocation.pubkey)?;
            state.credit(&key, allocation.quanta);
        }
        Ok(state)
    }

    pub fn account(&self, key: &[u8; 32]) -> Account {
        self.accounts.get(key).copied().unwrap_or_default()
    }

    pub fn balance(&self, key: &[u8; 32]) -> u64 {
        self.account(key).quanta
    }

    pub fn nonce(&self, key: &[u8; 32]) -> u64 {
        self.account(key).nonce
    }

    pub fn credit(&mut self, key: &[u8; 32], quanta: u64) {
        let entry = self.accounts.entry(*key).or_default();
        entry.quanta = entry.quanta.saturating_add(quanta);
    }

    /// Verify and apply a transfer. Atomic: on error nothing changes.
    pub fn apply(&mut self, tx: &Transaction) -> Result<(), TxError> {
        tx.verify()?;

        let sender = self.account(&tx.from);
        if tx.nonce != sender.nonce {
            return Err(TxError::BadNonce {
                expected: sender.nonce,
                got: tx.nonce,
            });
        }
        if sender.quanta < tx.amount {
            return Err(TxError::InsufficientFunds);
        }

        let mut next_sender = sender;
        next_sender.quanta -= tx.amount;
        next_sender.nonce = sender.nonce.checked_add(1).ok_or(TxError::Overflow)?;

        self.accounts.insert(tx.from, next_sender);
        self.credit(&tx.to, tx.amount);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.accounts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }

    pub fn total_supply(&self) -> u64 {
        self.accounts.values().map(|a| a.quanta).sum()
    }

    /// Sorted snapshot, for RPC / explorers.
    pub fn snapshot(&self) -> Vec<(String, Account)> {
        self.accounts
            .iter()
            .map(|(k, v)| (hex::encode(k), *v))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genesis::{Genesis, QUANTA_PER_CHRONYX};
    use ed25519_dalek::SigningKey;

    fn setup() -> (State, SigningKey, SigningKey) {
        let g = Genesis::devnet(1);
        let state = State::from_genesis(&g).unwrap();
        let alice = SigningKey::from_bytes(&[9u8; 32]);
        let bob = SigningKey::from_bytes(&[10u8; 32]);
        (state, alice, bob)
    }

    #[test]
    fn genesis_allocations_are_credited() {
        let g = Genesis::devnet(2);
        let state = State::from_genesis(&g).unwrap();
        for v in &g.validators {
            let key = crate::genesis::parse_key(&v.identity).unwrap();
            assert_eq!(state.balance(&key), 1_000 * QUANTA_PER_CHRONYX);
        }
    }

    #[test]
    fn signed_transfer_moves_funds_and_bumps_nonce() {
        let (mut state, alice, bob) = setup();
        state.credit(&alice.verifying_key().to_bytes(), 1_000);

        let tx = Transaction::sign(&alice, bob.verifying_key().to_bytes(), 400, 0);
        state.apply(&tx).unwrap();

        assert_eq!(state.balance(&alice.verifying_key().to_bytes()), 600);
        assert_eq!(state.balance(&bob.verifying_key().to_bytes()), 400);
        assert_eq!(state.nonce(&alice.verifying_key().to_bytes()), 1);
    }

    #[test]
    fn replay_is_rejected() {
        let (mut state, alice, bob) = setup();
        state.credit(&alice.verifying_key().to_bytes(), 1_000);
        let tx = Transaction::sign(&alice, bob.verifying_key().to_bytes(), 100, 0);
        state.apply(&tx).unwrap();
        assert!(state.apply(&tx).is_err());
    }

    #[test]
    fn overspend_is_rejected() {
        let (mut state, alice, bob) = setup();
        state.credit(&alice.verifying_key().to_bytes(), 50);
        let tx = Transaction::sign(&alice, bob.verifying_key().to_bytes(), 100, 0);
        assert!(state.apply(&tx).is_err());
    }

    #[test]
    fn forged_signature_is_rejected() {
        let (mut state, alice, bob) = setup();
        state.credit(&alice.verifying_key().to_bytes(), 1_000);
        let mut tx = Transaction::sign(&alice, bob.verifying_key().to_bytes(), 100, 0);
        tx.amount = 900; // tamper after signing
        assert!(state.apply(&tx).is_err());
    }

    #[test]
    fn self_transfer_only_bumps_nonce() {
        let (mut state, alice, _) = setup();
        let key = alice.verifying_key().to_bytes();
        state.credit(&key, 1_000);
        let tx = Transaction::sign(&alice, key, 500, 0);
        state.apply(&tx).unwrap();
        assert_eq!(state.balance(&key), 1_000);
        assert_eq!(state.nonce(&key), 1);
    }
}

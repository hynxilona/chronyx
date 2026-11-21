//! Supply invariants.
//!
//! These tests are the monetary policy expressed as code. If one of them has
//! to change, that is not a refactor — it is a decision about how many tokens
//! exist, and it needs its own review.

use chronyx_core::{
    Genesis, State, Transaction, QUANTA_PER_CHRONYX, TOTAL_SUPPLY_CHRONYX,
    VALIDATOR_ALLOCATION_CHRONYX,
};
use ed25519_dalek::SigningKey;

fn cap_quanta() -> u64 {
    TOTAL_SUPPLY_CHRONYX * QUANTA_PER_CHRONYX
}

#[test]
fn genesis_never_exceeds_the_cap() {
    for validators in [1, 2, 3, 4, 7, 12, 23] {
        let genesis = Genesis::devnet(validators);
        assert_eq!(
            genesis.total_quanta(),
            cap_quanta(),
            "{validators} validators must still sum to the fixed supply"
        );

        let state = State::from_genesis(&genesis).unwrap();
        assert_eq!(state.total_supply(), cap_quanta());
    }
}

#[test]
fn validator_allocation_is_25m_each() {
    let genesis = Genesis::devnet(4);
    assert_eq!(
        VALIDATOR_ALLOCATION_CHRONYX * QUANTA_PER_CHRONYX,
        25_000_000 * QUANTA_PER_CHRONYX
    );
    assert_eq!(
        genesis
            .validators
            .len()
            .saturating_mul(VALIDATOR_ALLOCATION_CHRONYX as usize),
        100_000_000,
        "devnet v1 allocates 25M to each of 4 validators"
    );
}

#[test]
fn transfers_move_quanta_without_creating_any() {
    let mut state = State::from_genesis(&Genesis::devnet(3)).unwrap();
    let before = state.total_supply();

    let alice = SigningKey::from_bytes(&[21u8; 32]);
    let bob = SigningKey::from_bytes(&[22u8; 32]);
    state.credit(&alice.verifying_key().to_bytes(), 1_000);

    let funded = state.total_supply();
    assert_eq!(funded, before + 1_000);

    for nonce in 0..5u64 {
        let tx = Transaction::sign(&alice, bob.verifying_key().to_bytes(), 100, nonce);
        state.apply(&tx).unwrap();
        assert_eq!(
            state.total_supply(),
            funded,
            "a transfer must conserve supply (nonce {nonce})"
        );
    }

    assert_eq!(state.balance(&alice.verifying_key().to_bytes()), 500);
    assert_eq!(state.balance(&bob.verifying_key().to_bytes()), 500);
}

#[test]
fn a_transfer_cannot_pay_out_more_than_it_holds() {
    let mut state = State::from_genesis(&Genesis::devnet(1)).unwrap();
    let alice = SigningKey::from_bytes(&[31u8; 32]);
    let bob = SigningKey::from_bytes(&[32u8; 32]);
    state.credit(&alice.verifying_key().to_bytes(), 10);

    let before = state.total_supply();
    let tx = Transaction::sign(&alice, bob.verifying_key().to_bytes(), u64::MAX, 0);
    assert!(state.apply(&tx).is_err());
    assert_eq!(
        state.total_supply(),
        before,
        "a rejected tx changes nothing"
    );
    assert_eq!(state.balance(&alice.verifying_key().to_bytes()), 10);
    assert_eq!(state.balance(&bob.verifying_key().to_bytes()), 0);
}

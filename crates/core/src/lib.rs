//! Chronyx core: PoH clock, PoW admission, PoA finality, native accounts.
//!
//! Design contract:
//!   * PoH  -> ordering + verifiable time
//!   * PoA  -> fast finality over a known validator set
//!   * PoW  -> permissionless admission into that set
//!
//! Nothing in here does I/O. It is pure, deterministic, and testable.

pub mod genesis;
pub mod keys;
pub mod poa;
pub mod poh;
pub mod pow;
pub mod state;
pub mod types;

pub use genesis::{Genesis, GenesisAllocation, GenesisValidator, QUANTA_PER_CHRONYX};
pub use keys::KeypairFile;
pub use state::{Account, State};
pub use types::{Block, BlockHeader, Hash, Transaction, TxError};

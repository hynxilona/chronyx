//! Node state, ledger replay, and the slot clock.
//!
//! Slot number is derived from the PoH tick count, not from block height. That
//! way every node agrees on which slot it is in without talking to anyone — and
//! a missing leader just leaves a gap instead of corrupting the chain.

use std::sync::Arc;
use std::time::{Duration, Instant};

use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use serde::Serialize;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use tachyon_core::genesis::{now_ms, FAUCET_SEED};
use tachyon_core::poa::ValidatorSet;
use tachyon_core::poh::PohRecorder;
use tachyon_core::types::merkle_root;
use tachyon_core::{Block, BlockHeader, Genesis, Hash, KeypairFile, State, Transaction};

use crate::config::NodeConfig;
use crate::ledger::Ledger;
use crate::p2p::{Hub, Msg};

const RING_SIZE: usize = 1024;

/// Maximum blocks returned in one gossip page. Small enough to stay well under
/// `MAX_FRAME`, large enough that catch-up converges quickly.
pub const BLOCKS_PAGE: usize = 1024;

pub struct Node {
    pub config: NodeConfig,
    pub genesis: Genesis,
    pub genesis_hash: Hash,
    pub poh: Mutex<PohRecorder>,
    pub height: Mutex<u64>,
    pub blocks: Mutex<Vec<Block>>,
    pub mempool: Mutex<Vec<Transaction>>,
    pub state: Mutex<State>,
    pub validators: ValidatorSet,
    pub local: SigningKey,
    pub faucet_key: Option<SigningKey>,
    pub difficulty: u32,
    pub started: Instant,
    pub ledger: Ledger,
    pub hub: Hub,
    pub tick_ms: u64,
    pub ticks_per_slot: u64,
    pub slot_ms: u64,
}

#[derive(Serialize)]
pub struct Status {
    pub chain: &'static str,
    pub chain_id: String,
    pub genesis_hash: String,
    pub poh_hash: String,
    pub ticks: u64,
    pub slot: u64,
    pub height: u64,
    pub mempool: usize,
    pub validators: usize,
    pub quorum: usize,
    pub last_block: Option<String>,
    pub supply: u64,
    pub accounts: usize,
    pub peers: usize,
    pub uptime_secs: u64,
    pub slot_ms: u64,
    pub local_validator: String,
    pub is_validator: bool,
}

impl Node {
    pub fn open(config: NodeConfig) -> anyhow::Result<Self> {
        // ---- genesis (create a devnet one if absent) ----
        let genesis = if config.genesis_path.exists() {
            let text = std::fs::read_to_string(&config.genesis_path)?;
            Genesis::from_json(&text).map_err(|e| anyhow::anyhow!(e))?
        } else {
            let g = Genesis::devnet(3);
            if let Some(parent) = config.genesis_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&config.genesis_path, g.to_json())?;
            g
        };
        genesis.validate().map_err(|e| anyhow::anyhow!(e))?;
        let genesis_hash = genesis.hash();

        // ---- identity ----
        let keypair = if config.identity_path.exists() {
            KeypairFile::load(&config.identity_path)?
        } else {
            let kp = KeypairFile::generate();
            kp.save(&config.identity_path)?;
            kp
        };
        let local = keypair.signing_key().map_err(|e| anyhow::anyhow!(e))?;

        // ---- validator set from genesis ----
        let identities = genesis
            .validator_identities()
            .map_err(|e| anyhow::anyhow!(e))?;
        let vkeys: Vec<VerifyingKey> = identities
            .iter()
            .map(|b| VerifyingKey::from_bytes(b).map_err(|e| anyhow::anyhow!(e)))
            .collect::<anyhow::Result<_>>()?;
        let validators = ValidatorSet::new(vkeys);

        // ---- state ----
        let mut state = State::from_genesis(&genesis).map_err(|e| anyhow::anyhow!(e))?;

        // ---- replay persisted blocks ----
        let ledger = Ledger::open(&config.data_dir)?;
        let persisted = ledger.load()?;
        let mut blocks: Vec<Block> = Vec::with_capacity(persisted.len().min(RING_SIZE));
        let mut height = 0u64;
        let mut poh_seed = genesis_hash;
        for block in persisted {
            for tx in &block.transactions {
                let _ = state.apply(tx);
            }
            height = height.max(block.header.slot);
            poh_seed = block.header.poh_hash;
            blocks.push(block);
        }
        if blocks.len() > RING_SIZE {
            blocks.drain(0..blocks.len() - RING_SIZE);
        }

        // ---- devnet faucet (only if genesis points at the canonical key) ----
        let faucet_key = if config.faucet_enabled {
            let fk = SigningKey::from_bytes(&FAUCET_SEED);
            let expected = hex::encode(fk.verifying_key().to_bytes());
            if genesis.faucet.as_deref() == Some(expected.as_str()) {
                Some(fk)
            } else {
                warn!("genesis faucet does not match dev faucet key; faucet disabled");
                None
            }
        } else {
            None
        };

        let tick_ms = (genesis.slot_ms / genesis.ticks_per_slot).max(1);

        info!(
            chain_id = %genesis.chain_id,
            genesis = %hex::encode(genesis_hash),
            validators = validators.len(),
            replayed = blocks.len(),
            height,
            "node opened"
        );

        Ok(Self {
            difficulty: genesis.pow_difficulty,
            tick_ms,
            ticks_per_slot: genesis.ticks_per_slot,
            slot_ms: genesis.slot_ms,
            config,
            genesis,
            genesis_hash,
            poh: Mutex::new(PohRecorder::new(poh_seed)),
            height: Mutex::new(height),
            blocks: Mutex::new(blocks),
            mempool: Mutex::new(Vec::new()),
            state: Mutex::new(state),
            validators,
            local,
            faucet_key,
            started: Instant::now(),
            ledger,
            hub: Hub::default(),
        })
    }

    pub async fn tip(&self) -> Hash {
        let blocks = self.blocks.lock().await;
        blocks.last().map(|b| b.hash()).unwrap_or([0u8; 32])
    }

    pub async fn height(&self) -> u64 {
        *self.height.lock().await
    }

    /// Slot index for "now", anchored to genesis time.
    ///
    /// This is what makes a multi-node cluster agree: every node derives the
    /// same slot from the same genesis timestamp plus its own wall clock, so
    /// leader schedules line up without any coordination round-trip. PoH still
    /// ticks locally and its hash is committed into every header — it is the
    /// ordering record, not the wall clock.
    pub fn current_slot(&self) -> u64 {
        let now = now_ms();
        if now <= self.genesis.genesis_time_ms {
            return 0;
        }
        (now - self.genesis.genesis_time_ms) / self.slot_ms.max(1)
    }

    /// Full block validation. Cheap enough to run on every gossip message.
    pub fn validate_block(&self, block: &Block) -> Result<(), String> {
        if merkle_root(&block.transactions) != block.header.tx_root {
            return Err("tx_root does not match transactions".into());
        }
        let scheduled = self
            .validators
            .leader_for_slot(block.header.slot)
            .map(|k| k.to_bytes())
            .ok_or_else(|| "no leader scheduled for slot".to_string())?;
        if scheduled != block.header.leader {
            return Err(format!(
                "slot {} leader mismatch: expected {}, got {}",
                block.header.slot,
                hex::encode(scheduled),
                hex::encode(block.header.leader)
            ));
        }
        let vk = VerifyingKey::from_bytes(&block.header.leader)
            .map_err(|_| "bad leader key".to_string())?;
        let vote = block
            .signatures
            .first()
            .ok_or_else(|| "block has no PoA vote".to_string())?;
        vk.verify(&block.vote_payload(), vote)
            .map_err(|_| "bad block signature".to_string())?;
        Ok(())
    }

    pub async fn blocks_from(&self, from: u64) -> Vec<Block> {
        // Fast path: everything requested is still in the in-memory ring.
        {
            let blocks = self.blocks.lock().await;
            if let Some(first) = blocks.first() {
                if from >= first.header.slot {
                    return blocks
                        .iter()
                        .filter(|b| b.header.slot >= from)
                        .take(BLOCKS_PAGE)
                        .cloned()
                        .collect();
                }
            }
        }

        // Slow path: the requester is further behind than the ring reaches, so
        // serve the page from the persisted ledger. This is what lets a brand
        // new node sync a devnet that has been running for hours.
        match self.ledger.load_from(from, BLOCKS_PAGE) {
            Ok(blocks) => blocks,
            Err(e) => {
                warn!(error = %e, from, "ledger read failed during catch-up");
                Vec::new()
            }
        }
    }

    pub async fn block_at(&self, slot: u64) -> Option<Block> {
        let blocks = self.blocks.lock().await;
        blocks.iter().find(|b| b.header.slot == slot).cloned()
    }

    /// Accept a block from a peer. Idempotent: older slots are ignored.
    ///
    /// Checks the merkle root, the scheduled leader, the PoA vote, and that the
    /// block extends our current tip. State is applied deterministically in
    /// block order, so every node ends up with the same account table.
    pub async fn accept_block(&self, block: Block) {
        let slot = block.header.slot;

        if let Err(e) = self.validate_block(&block) {
            warn!(slot, error = %e, "rejected block");
            return;
        }

        {
            let height = *self.height.lock().await;
            if slot <= height {
                return;
            }
        }

        {
            let blocks = self.blocks.lock().await;
            let tip = blocks.last().map(|b| b.hash()).unwrap_or([0u8; 32]);
            if block.header.parent != tip {
                // We are behind: this block is valid, just not the next one we
                // can apply. The catch-up loop pulls the missing range from the
                // ledger, so this is expected during sync, not an error.
                debug!(
                    slot,
                    expected = %hex::encode(tip),
                    got = %hex::encode(block.header.parent),
                    "block ahead of tip; waiting for catch-up"
                );
                return;
            }
        }

        {
            let mut state = self.state.lock().await;
            for tx in &block.transactions {
                let _ = state.apply(tx);
            }
        }

        {
            let mut height = self.height.lock().await;
            if slot > *height {
                *height = slot;
            }
        }

        if let Err(e) = self.ledger.append(&block) {
            warn!(error = %e, "ledger append failed");
        }

        let hash = block.hash();
        {
            let mut blocks = self.blocks.lock().await;
            blocks.push(block.clone());
            if blocks.len() > RING_SIZE {
                blocks.remove(0);
            }
        }

        info!(slot, hash = %hex::encode(hash), "accepted remote block");
        self.hub.broadcast(Msg::Block(block));
    }

    pub async fn status(&self) -> Status {
        let (poh_hash, ticks) = {
            let poh = self.poh.lock().await;
            (hex::encode(poh.hash()), poh.ticks())
        };
        let height = *self.height.lock().await;
        let mempool = self.mempool.lock().await.len();
        let (supply, accounts) = {
            let state = self.state.lock().await;
            (state.total_supply(), state.len())
        };
        let last_block = {
            let blocks = self.blocks.lock().await;
            blocks.last().map(|b| hex::encode(b.hash()))
        };
        let current_slot = self.current_slot();

        Status {
            chain: "tachyon",
            chain_id: self.genesis.chain_id.clone(),
            genesis_hash: hex::encode(self.genesis_hash),
            poh_hash,
            ticks,
            slot: current_slot,
            height,
            mempool,
            validators: self.validators.len(),
            quorum: self.validators.quorum(),
            last_block,
            supply,
            accounts,
            peers: self.hub.count(),
            uptime_secs: self.started.elapsed().as_secs(),
            slot_ms: self.slot_ms,
            local_validator: hex::encode(self.local.verifying_key().to_bytes()),
            is_validator: self.validators.contains(&self.local.verifying_key()),
        }
    }
}

/// The heartbeat: tick PoH, and if this node leads the current slot, seal it.
pub async fn run_slot_clock(node: Arc<Node>) {
    let mut interval = tokio::time::interval(Duration::from_millis(node.tick_ms));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Burst);

    loop {
        interval.tick().await;

        // 1. Advance the verifiable clock. PoH gives us an ordering record that
        //    can be replayed by anyone; it is not the wall clock.
        {
            let mut poh = node.poh.lock().await;
            poh.tick();
        }

        // 2. Which slot is it? Derived from genesis time on every node, so the
        //    leader schedule lines up cluster-wide without coordination.
        let slot = node.current_slot();
        if slot == 0 || slot <= node.height().await {
            continue;
        }

        // 3. Are we the leader for this slot?
        let leader = node
            .validators
            .leader_for_slot(slot)
            .map(|k| k.to_bytes())
            .unwrap_or([0u8; 32]);
        if node.local.verifying_key().to_bytes() != leader {
            continue;
        }

        // 4. Drain the mempool, applying only valid transfers.
        let pending = {
            let mut mempool = node.mempool.lock().await;
            std::mem::take(&mut *mempool)
        };
        let mut accepted = Vec::with_capacity(pending.len());
        {
            let mut state = node.state.lock().await;
            for tx in pending {
                if state.apply(&tx).is_ok() {
                    accepted.push(tx);
                }
            }
        }

        // 5. Seal.
        let parent = node.tip().await;
        let tx_count = accepted.len();
        let tx_root = merkle_root(&accepted);
        let poh_hash = {
            let mut poh = node.poh.lock().await;
            poh.record(&tx_root)
        };

        let header = BlockHeader {
            slot,
            parent,
            poh_hash,
            tx_root,
            leader,
            timestamp_ms: now_ms(),
        };
        let mut block = Block {
            header,
            transactions: accepted,
            signatures: Vec::new(),
        };
        block
            .signatures
            .push(node.local.sign(&block.vote_payload()));

        let hash = block.hash();
        {
            let mut height = node.height.lock().await;
            if slot > *height {
                *height = slot;
            }
        }
        if let Err(e) = node.ledger.append(&block) {
            warn!(error = %e, "ledger append failed");
        }
        {
            let mut blocks = node.blocks.lock().await;
            blocks.push(block.clone());
            if blocks.len() > RING_SIZE {
                blocks.remove(0);
            }
        }
        node.hub.broadcast(Msg::Block(block));

        info!(slot, txs = tx_count, hash = %hex::encode(hash), "sealed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use tachyon_core::genesis::{GenesisAllocation, GenesisValidator, LAMPORTS_PER_TACHYON};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let dir =
                std::env::temp_dir().join(format!("tachyon-test-{tag}-{}-{n}", std::process::id()));
            std::fs::create_dir_all(&dir).expect("temp dir");
            Self(dir)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn test_node(tag: &str) -> (Arc<Node>, TempDir) {
        let dir = TempDir::new(tag);
        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let pubkey = hex::encode(sk.verifying_key().to_bytes());

        let mut genesis = Genesis::devnet(1);
        genesis.validators.push(GenesisValidator {
            identity: pubkey.clone(),
            name: "test".into(),
            commission_bps: 100,
        });
        genesis.allocations.push(GenesisAllocation {
            pubkey,
            lamports: 100 * LAMPORTS_PER_TACHYON,
        });
        genesis.validate().unwrap();

        let genesis_path = dir.path().join("genesis.json");
        std::fs::write(&genesis_path, genesis.to_json()).unwrap();

        let identity_path = dir.path().join("validator.json");
        KeypairFile::from_signing_key(&sk)
            .save(&identity_path)
            .unwrap();

        let config = NodeConfig {
            rpc: "127.0.0.1:0".parse().unwrap(),
            p2p: "127.0.0.1:0".parse().unwrap(),
            data_dir: dir.path().to_path_buf(),
            genesis_path,
            identity_path,
            peers: Vec::new(),
            faucet_enabled: true,
        };

        let node = Arc::new(Node::open(config).expect("node opens"));
        (node, dir)
    }

    /// First slot at or after `from` where the local key is the leader.
    fn local_slot(node: &Node, from: u64) -> u64 {
        let me = node.local.verifying_key().to_bytes();
        (from..from + 64)
            .find(|slot| {
                node.validators
                    .leader_for_slot(*slot)
                    .map(|k| k.to_bytes() == me)
                    .unwrap_or(false)
            })
            .expect("local key is in the validator set")
    }

    fn seal(node: &Node, slot: u64, parent: Hash, txs: Vec<Transaction>) -> Block {
        let tx_root = merkle_root(&txs);
        let mut block = Block {
            header: BlockHeader {
                slot,
                parent,
                poh_hash: [1u8; 32],
                tx_root,
                leader: node.local.verifying_key().to_bytes(),
                timestamp_ms: now_ms(),
            },
            transactions: txs,
            signatures: Vec::new(),
        };
        block
            .signatures
            .push(node.local.sign(&block.vote_payload()));
        block
    }

    #[test]
    fn config_roundtrips_through_toml() {
        let dir = TempDir::new("config");
        let path = dir.path().join("node.toml");
        let mut cfg = NodeConfig::devnet(dir.path());
        cfg.peers.push("10.0.0.1:9000".parse().unwrap());
        cfg.save(&path).unwrap();

        let loaded = NodeConfig::load(&path).unwrap();
        assert_eq!(loaded.rpc, cfg.rpc);
        assert_eq!(loaded.p2p, cfg.p2p);
        assert_eq!(loaded.peers, cfg.peers);
        assert!(loaded.faucet_enabled);
    }

    #[test]
    fn ledger_roundtrips_and_counts() {
        let dir = TempDir::new("ledger");
        let ledger = Ledger::open(dir.path()).unwrap();

        let (node, _dir) = test_node("ledger-node");
        let slot = local_slot(&node, 1);
        let block = seal(&node, slot, [0u8; 32], Vec::new());

        ledger.append(&block).unwrap();
        ledger.append(&block).unwrap();

        let loaded = ledger.load().unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].header.slot, slot);
        assert_eq!(loaded[0].hash(), block.hash());
    }

    #[test]
    fn ledger_load_from_filters_and_caps() {
        let dir = TempDir::new("loadfrom");
        let ledger = Ledger::open(dir.path()).unwrap();
        let (node, _node_dir) = test_node("loadfrom-node");

        let mut parent = [0u8; 32];
        let mut slots = Vec::new();
        for _ in 0..5 {
            let from = slots.last().copied().unwrap_or(0) + 1;
            let slot = local_slot(&node, from);
            let block = seal(&node, slot, parent, Vec::new());
            parent = block.hash();
            ledger.append(&block).unwrap();
            slots.push(slot);
        }

        let page = ledger.load_from(slots[1], 2).unwrap();
        assert_eq!(page.len(), 2);
        assert_eq!(page[0].header.slot, slots[1]);
        assert_eq!(page[1].header.slot, slots[2]);

        assert_eq!(ledger.load_from(0, 100).unwrap().len(), 5);
        assert!(ledger.load_from(0, 0).unwrap().is_empty());
        assert!(ledger.load_from(slots[4] + 1, 10).unwrap().is_empty());
    }

    #[tokio::test]
    async fn blocks_from_serves_from_the_ring() {
        let (node, _dir) = test_node("ring");
        let mut parent = [0u8; 32];
        let mut slots = Vec::new();
        for _ in 0..3 {
            let from = slots.last().copied().unwrap_or(0) + 1;
            let slot = local_slot(&node, from);
            let block = seal(&node, slot, parent, Vec::new());
            parent = block.hash();
            node.accept_block(block).await;
            slots.push(slot);
        }

        let page = node.blocks_from(slots[1]).await;
        assert_eq!(page.len(), 2);
        assert_eq!(page[0].header.slot, slots[1]);
        assert!(node.blocks_from(slots[2] + 1).await.is_empty());
    }

    #[tokio::test]
    async fn blocks_from_falls_back_to_the_ledger() {
        let (node, _dir) = test_node("fallback");
        let mut parent = [0u8; 32];
        let mut slots = Vec::new();
        for _ in 0..3 {
            let from = slots.last().copied().unwrap_or(0) + 1;
            let slot = local_slot(&node, from);
            let block = seal(&node, slot, parent, Vec::new());
            parent = block.hash();
            node.accept_block(block).await;
            slots.push(slot);
        }

        // Simulate a node whose ring has evicted its history.
        node.blocks.lock().await.clear();

        let page = node.blocks_from(0).await;
        assert_eq!(page.len(), 3);
        assert_eq!(page[0].header.slot, slots[0]);
        assert_eq!(page[2].header.slot, slots[2]);
    }

    #[test]
    fn validate_block_checks_root_leader_and_vote() {
        let (node, _dir) = test_node("validate");
        let slot = local_slot(&node, 1);
        let block = seal(&node, slot, [0u8; 32], Vec::new());
        node.validate_block(&block).expect("valid block");

        let mut bad_root = block.clone();
        bad_root.header.tx_root = [9u8; 32];
        assert!(node.validate_block(&bad_root).is_err());

        let mut bad_leader = block.clone();
        bad_leader.header.leader = [3u8; 32];
        assert!(node.validate_block(&bad_leader).is_err());

        let mut no_vote = block;
        no_vote.signatures.clear();
        assert!(node.validate_block(&no_vote).is_err());
    }

    #[tokio::test]
    async fn accept_block_applies_transfers_and_persists() {
        let (node, dir) = test_node("accept");
        let faucet = node.faucet_key.clone().expect("devnet faucet");
        let recipient = SigningKey::from_bytes(&[9u8; 32])
            .verifying_key()
            .to_bytes();

        let tx = Transaction::sign(&faucet, recipient, 5_000, 0);
        let slot = local_slot(&node, 1);
        let block = seal(&node, slot, [0u8; 32], vec![tx]);
        let hash = block.hash();

        node.accept_block(block).await;

        assert_eq!(node.height().await, slot);
        assert_eq!(node.state.lock().await.balance(&recipient), 5_000);
        assert_eq!(node.ledger.count(), 1);
        assert_eq!(node.tip().await, hash);

        // Replaying the ledger reconstructs the same state.
        let reopened = Node::open(node.config.clone()).unwrap();
        assert_eq!(reopened.state.lock().await.balance(&recipient), 5_000);
        assert_eq!(reopened.status().await.height, slot);
        drop(dir);
    }

    #[tokio::test]
    async fn accept_block_rejects_a_broken_parent() {
        let (node, _dir) = test_node("parent");
        let slot = local_slot(&node, 1);
        let block = seal(&node, slot, [4u8; 32], Vec::new());

        node.accept_block(block).await;

        assert_eq!(node.height().await, 0);
        assert!(node.blocks.lock().await.is_empty());
    }
}

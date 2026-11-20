# Tachyon

> **PoH orders. PoA finalizes. PoW admits.**

A small, fast Rust chain built from scratch — no framework, no VM, no ERC-20.
Native accounts, signed transfers, real persistence, real gossip.

| Mechanism | Role | Why |
|---|---|---|
| **PoH** | Verifiable clock + total order | Sequential SHA-256 chain; proves time passed |
| **PoA** | Fast finality | Round-robin leaders over a known validator set → 400 ms slots |
| **PoW** | Permissionless admission | Joining the validator set costs work, not a token |

PoA alone is permissioned. PoW alone is slow. PoH alone is just a clock.
Stacked correctly, each covers the others' weakness.

---

## Status: v1.5 — public devnet

Working today:

- ✅ **PoH recorder** — 50 ms ticks, replayable
- ✅ **PoA slots** — 400 ms, deterministic round-robin leaders, genesis-time anchored
- ✅ **Block validation** — merkle root, scheduled leader, Ed25519 vote, parent link
- ✅ **Native token** — Ed25519-signed transfers, per-account nonce, atomic apply
- ✅ **Append-only ledger** — `blocks.jsonl`, replayed on restart, state reconstructed
- ✅ **TCP gossip** — hello/height handshake, catch-up (`GetBlocks`), live `Block` fan-out
- ✅ **Devnet faucet** — nonce-safe under concurrent requests, 1 000 TACHYON per request
- ✅ **HTTP RPC** — 14 routes, JSON in/out
- ✅ **CLI** — ~50 commands, Solana-style namespacing
- ✅ **30 tests** — PoH, PoA, PoW, state, block validation, ledger, restart replay, catch-up

Not in v1: dynamic validator admission, QUIC, light clients, explorer.

---

## Quickstart

Build:

```bash
cargo build --release
```

Create a devnet and start the node:

```bash
# 1. write genesis.json, an identity, node.toml, and point the CLI at them
tachyon devnet init --dir ./devnet --validators 3

# 2. run the node (adds your identity to the genesis validator set)
tachyond --config ./devnet/node.toml
```

`tachyon devnet init` also writes `~/.tachyon/cli.toml`, so the rest is
zero-config:

```bash
tachyon cluster info       # chain id, genesis hash, height, slot, supply
tachyon cluster validators # set, quorum, next leader
tachyon doctor             # is everything wired up?
```

Fund a wallet and move coins:

```bash
tachyon keypair new --out ./devnet/alice.json
tachyon airdrop <alice-pubkey> 10000000000      # 10 TACHYON
tachyon account balance <alice-pubkey>
tachyon transfer ./devnet/alice.json <bob-pubkey> 2500000000
```

Add a second node that follows the same chain:

```bash
tachyon devnet join http://127.0.0.1:8899 --dir ./node2 --peer 127.0.0.1:9000
tachyond --config ./node2/node.toml --rpc 127.0.0.1:8900 --p2p 127.0.0.1:9001
```

It handshakes, pulls every block it missed, applies them in order, and stays
within one slot of the leader. Catch-up is paginated and served from the
persisted ledger, so a brand-new node can join a devnet that has been running
for hours — it does not depend on the in-memory block ring.

Default ports are **8899** (HTTP RPC) and **9000** (p2p). Override them per
node with `--rpc` / `--p2p`, and point the CLI with `--url` or `TACHYON_RPC`.

---

## How a slot works

```
tick (50 ms) ──► PoH hash ──► current_slot = (now - genesis_time) / 400ms
                                   │
                    is this node the scheduled leader?
                                   │ yes
                 drain mempool ──► apply valid txs ──► seal ──► broadcast
```

- **Slot number is wall-clock**, anchored to `genesis_time_ms`. Every node
  derives the same slot independently, so leader schedules line up without
  coordination. PoH supplies the ordering record that goes into every header.
- **Leaders rotate round-robin** over the genesis validator set. A missing
  leader just leaves a gap — the next slot's leader continues.
- **Downtime shows up as sparse slot numbers.** Slot numbers come from the wall
  clock, so a restart after downtime jumps ahead. The ledger keeps one block per
  produced slot, not per elapsed slot, so gaps in slot numbers are normal; every
  block still names its parent, and the chain stays linked across a gap.
- **A block is accepted only if** its merkle root matches, its leader is the
  one scheduled for that slot, the leader's Ed25519 vote verifies, and its
  parent is our current tip. No forks, no reorgs in v1.
- **State is applied in block order** by every node, so balances and nonces
  converge exactly.

## Why PoW

PoA gets you 400 ms finality, but it needs a membership answer. Hashcash-style
PoW is that answer: admission costs work, not capital, so there is no token sale
gate and no stake requirement. `tachyon pow mine --difficulty 20` produces a
proof in ~14 s on one core and verifies in microseconds; it prints the exact
`tachyon pow verify …` command to check it. v1 exposes the gate as a CLI/RPC
primitive; wiring proofs into an automatic join transaction is the v2 headline.

## Token model

Only two fields per account:

```rust
struct Account { lamports: u64, nonce: u64 }
```

A transfer is a signed message `(from, to, amount, nonce)`; applying it checks
the signature, the nonce, and the balance, then moves lamports and bumps the
nonce. 1 TACHYON = 1 000 000 000 lamports. Supply is created only by genesis
allocations; nothing mints at runtime. This is a native asset, not a contract.

---

## CLI

Global flag: `-u, --url <RPC>` (also `TACHYON_RPC`) overrides the stored endpoint.

| Group | Commands |
|---|---|
| `config` | `get` `set` `show` `path` `reset` |
| `keypair` | `new` `show` `address` `sign` `verify` `list` |
| `genesis` | `new` `show` `hash` `validate` `add-validator` `add-allocation` |
| `cluster` | `info` `health` `version` `slot` `block-height` `validators` `peers` `genesis` `supply` |
| `account` | `balance` `show` `list` |
| `tx` | `new` `send` `status` |
| `block` | `get` `height` `list` |
| `poh` | `head` `ticks` |
| `pow` | `mine` `verify` |
| `validator` | `identity` `info` `list` `join` |
| `devnet` | `init` `join` `publish` `info` |
| `bench` | `pow` `poh` |
| top level | `airdrop` `transfer` `doctor` `version` |

## RPC

| Method | Path | Purpose |
|---|---|---|
| GET | `/health` | liveness — `{"ok":true,"chain_id":…,"uptime_secs":…}` |
| GET | `/status` | chain id, genesis hash, slot, height, mempool, peers, supply |
| GET | `/genesis` | genesis JSON + its hash |
| GET | `/validators` | set, quorum, next leader |
| GET | `/supply` | total lamports, account count |
| GET | `/peers` | connected peer count |
| GET | `/account/{pubkey}` | lamports + nonce |
| GET | `/accounts` | full account snapshot |
| GET | `/balance/{pubkey}` | lamports |
| GET | `/block/{slot}` | one block |
| GET | `/blocks` | height + the in-memory ring |
| POST | `/tx` | submit a signed `Transaction` |
| POST | `/faucet` | `{ "pubkey", "lamports" }` |
| GET | `/mine` | mine a PoW admission proof (demo) |

---

## Layout

```
crates/
  core/          pure, no I/O — deterministic and testable
    src/types.rs     Block, Header, Transaction, TxError, merkle root
    src/poh.rs       PoH recorder + replay
    src/pow.rs       hashcash admission gate
    src/poa.rs       validator set, leader schedule, quorum
    src/genesis.rs   genesis format, devnet defaults, faucet key
    src/state.rs     native accounts, signed transfer application
    src/keys.rs      on-disk keypair format
  node/          runtime, persistence, gossip, RPC
    src/node.rs      node state, block validation, slot clock
    src/ledger.rs    append-only blocks.jsonl + replay
    src/p2p.rs       length-prefixed JSON TCP gossip
    src/config.rs    node.toml
    src/rpc.rs       axum routes
    src/main.rs      tachyond binary
  cli/           `tachyon` binary (~50 commands)
```

## Design notes

**Genesis time is the clock.** Slot = `(now − genesis_time) / slot_ms`. All
nodes agree without a round-trip, and a stalled leader self-heals on the next
slot. PoH stays off the consensus path where it belongs.

**One writer per clock.** `run_slot_clock` is the only task that ticks PoH or
seals. Everything else reads. That is why slots land consistently.

**No serialization in header hashes.** Fields are hashed directly — no
allocation, no schema drift.

**Gossip is boring on purpose.** `u32` length + JSON. You can watch it with
`nc`, and it needs no handshake library. QUIC is a v2 performance change, not a
correctness one.

**Crash recovery is replay.** The ledger is the source of truth; state is
derived. Restarting a node replays `blocks.jsonl` and lands on the same
balances.

---

## Joining as a validator

v1 uses a **fixed validator set defined in genesis**. That is deliberate: it
makes block validation stateless and auditable, and it means a devnet cannot be
hijacked by a stranger with a laptop.

To grow the set today, add the identity and start a new devnet generation:

```bash
tachyon validator join ./my-keypair.json     # prints the exact commands
tachyon genesis add-validator genesis.json <pubkey> --name alice
# every node restarts against the new genesis hash
```

Everyone else can still run a **full node**: they sync every block, verify
every signature, serve RPC, and can be promoted in the next generation.

Automatic, PoW-gated admission — a `Join` transaction carrying a valid
hashcash proof, validator set updates applied by epoch — is the v2 headline.

## Roadmap

- **v1.5** ✅ multi-node devnet, ledger replay, faucet, CLI, full block validation
- **v2** PoW-gated `Join` tx + epoch-based validator set rotation
- **v2** QUIC gossip (`quinn`), batched signature verification
- **v2** `redb` index, pruned state snapshots
- **v3** JSON-RPC (`jsonrpsee`), explorer, light-client PoH proofs
- **v3** colocated bare-metal tuning: `io_uring`, kernel bypass, CPU pinning

## Honest limitations

- **Fixed validator set in v1.** New validators require a genesis change.
- **Single-signer blocks.** The PoA quorum primitive exists and is tested, but
  v1 blocks carry the leader's vote only; multi-party finality voting is v2.
- **No stake, no slashing, no economics.** Validators are unpaid.
- **Devnet faucet is a dev key.** Never enable it where funds have value.
- **Not audited. Not a mainnet. Do not put value on this.**
- Windows uses IOCP; deploy on Linux for `io_uring`-class tail latency.

## License

MIT

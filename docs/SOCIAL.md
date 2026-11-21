# Post copy

Ready to paste. Change the first line so it does not read as a template, and
keep the devnet disclaimer in every post.

## Pinned thread (X / Bluesky / Mastodon)

**1/**

I built a blockchain from scratch in Rust. No framework, no VM, no ERC-20.

PoH orders. PoA finalizes. PoW admits.

Devnet is live: https://hynxilona.github.io/chronyx/

**2/**

The design bet:

PoA alone is permissioned.
PoW alone is slow.
PoH alone is just a clock.

Stacked, each one covers the others' weakness — ordering, finality, admission.

**3/**

In practice:

· 50 ms PoH ticks, replayable by anyone
· 400 ms slots, round-robin leaders
· PoW proofs gate validator entry — work, not capital
· native Ed25519 accounts, append-only ledger, TCP gossip

**4/**

600,000,000 CHRONYX. Fixed. No inflation, no mint function, no presale.

Every unit either exists in genesis or does not exist at all. A test fails if a
transfer ever creates quanta.

**5/**

Running a follower node is two commands:

chronyx devnet join http://SEED:8899 --peer SEED:9000 --dir ./my-node
chronyxd --config ./my-node/node.toml

Check the genesis hash first. Different hash, different chain.

**6/**

It is a devnet. Not audited, not mainnet, no value. Faucet is open on purpose.

MIT, 35 tests, ~50 CLI commands, 15 RPC routes, prebuilt binaries for Linux,
macOS and Windows.

If you break it, tell me. That is what a devnet is for.
https://github.com/hynxilona/chronyx

## Show HN

**Title**

> Show HN: Chronyx – PoH orders, PoA finalizes, PoW admits

**Body**

> I wanted to see what a chain looks like when you write the whole thing
> yourself: no framework, no VM, no token contract, no consensus library.
>
> The split:
>
> - PoH is the clock. A sequential SHA-256 chain at 50 ms per tick proves time
>   passed and gives everyone the same order.
> - PoA is finality. A known validator set takes round-robin slots every 400 ms.
> - PoW is admission. Joining costs hashcash-style work that is cheap to verify.
>
> PoA alone is permissioned, PoW alone is slow, PoH alone is just a clock. Each
> mechanism does one job.
>
> Built so far: Ed25519 accounts, signed transfers with per-account nonces,
> append-only JSONL ledger replayed on restart, TCP gossip with catch-up, 15 RPC
> routes, a namespaced CLI, 35 tests. Supply is fixed at 600M with no mint path.
>
> It is a devnet: not audited, no value, faucet open on purpose. I would rather
> hear what breaks than pretend it is finished.
>
> https://github.com/hynxilona/chronyx

## r/rust

**Title**

> Chronyx: a blockchain written from scratch in Rust — PoH for ordering, PoA for
> finality, PoW for admission

**Body**

> I spent the last while writing a chain end to end instead of gluing libraries
> together. Native accounts and signed transfers, no VM, no contracts.
>
> Interesting parts:
>
> - **Verifiable ordering.** A SHA-256 chain ticks every 50 ms; two nodes that
>   agree on the hash agree on the order.
> - **Deterministic finality.** Round-robin leaders over a fixed validator set,
>   400 ms slots. Boring on purpose.
> - **Permissionless entry.** Hashcash-style proof to join the set, so admission
>   costs compute rather than a token or an allowlist. (Roadmap — v1 validators
>   are genesis-defined, and I say so in the README.)
> - **Persistence without a database.** Append-only `blocks.jsonl`, replayed at
>   startup; state is rebuilt, not stored.
> - **Gossip.** Plain TCP: hello/height handshake, `GetBlocks` catch-up, live
>   block fan-out.
>
> Workspace: `core` (state, block, PoH, PoA, PoW, ledger), `node` (RPC, gossip),
> `cli`. 35 tests, clippy `-D warnings`, CI on every push, binaries on tags.
>
> Fair warnings: devnet, not audited, no value, no dynamic validator admission,
> no light clients, no explorer.
>
> https://github.com/hynxilona/chronyx

## Validator call

> Chronyx is looking for 4 more devnet validators.
>
> 2 vCPU, 400 ms slots, no rewards (devnet), your name in `cluster validators`,
> a first look at the mainnet bootstrap set.
>
> Two commands to join as a follower, one issue to apply for a signing slot:
> https://github.com/hynxilona/chronyx/blob/main/docs/VALIDATORS.md
>
> If it breaks, you get to tell me exactly how.

## Weekly devlog template

> **Chronyx devlog — week N**
>
> Shipped: <3 bullets, links to commits>
> Broke: <1 thing that broke, honestly>
> Next: <1 thing, no date>
> Numbers: supply 600,000,000 · validators 4 · tests 35
>
> Devnet, not audited, no value.

## One-liners

- A Rust chain where PoH orders, PoA finalizes, and PoW admits. 600M fixed supply, public devnet.
- No framework, no VM, no ERC-20. Just 5k lines of Rust and three mechanisms with one job each.
- The clock, the judge, and the door. PoH, PoA, PoW — written from scratch.

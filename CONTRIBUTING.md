# Contributing

Chronyx is a from-scratch Rust chain, and the whole point is that a stranger can
read a change and know what it does. Small, honest diffs beat clever ones.

Every PR goes through the same gate. Nothing merges because it looks fine.

## The path a change takes

1. **Propose** — open an issue with the problem, not the patch. If the change
   touches consensus, balances, or the supply, say so in the first line.
2. **Scope** — one PR, one idea. Consensus changes get a design note in the
   issue before code.
3. **Automated checks** — the PR must pass all of these:

   | Check | What it stops |
   |---|---|
   | `cargo fmt` / `clippy -D warnings` | unreadable diffs, sloppy code |
   | `cargo test --workspace` | behaviour regressions |
   | `scripts/guard-supply.sh` | new minting paths, moved supply constants |
   | `cargo test --test invariants` | any change that alters total supply |
   | `scripts/check-pr-files.sh` | binary blobs, giant unreviewable diffs |
   | gitleaks | keys, tokens, credentials committed by accident |
   | `cargo audit` | known-vulnerable dependencies |
4. **Review** — `.github/CODEOWNERS` covers `genesis.rs`, `state.rs`, `types.rs`,
   the PoH/PoA/PoW modules, the ledger, and the release workflows. Those files
   cannot merge without the maintainer reading them.
5. **Merge** — squash, with the reasoning kept in the message.

A PR that cannot explain *why it cannot move anyone's balance* is a PR that
does not touch balances.

## What gets rejected on sight

- Anything that creates quanta outside `genesis.rs` and `state.rs`.
- A change to `TOTAL_SUPPLY_CHRONYX` or `QUANTA_PER_CHRONYX` without a public
  proposal and its own review.
- Binary files, archives, or minified blobs in the diff.
- `unsafe` without a comment explaining the invariant it upholds.
- Network calls or filesystem access inside `chronyx-core` — it must stay pure.
- Code that only makes sense when the reviewer cannot see it. Obfuscated
  identifiers, encoded strings, and "temporary" debug paths are rejected.
- Telemetry of any kind. The node talks to peers and the CLI talks to the node.
  That is the whole list.

## Build and test

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
bash scripts/guard-supply.sh
```

CI runs exactly those, plus the security workflows in
`.github/workflows/security.yml`. If they pass locally, the PR should pass.

## Layout

| Crate | What lives there |
|---|---|
| `crates/core` | PoH, PoA, PoW, state, blocks — pure, no I/O |
| `crates/node` | ledger, TCP gossip, axum RPC, slot clock |
| `crates/cli` | the `chronyx` binary |

Rules of thumb:

- `chronyx-core` must stay deterministic and I/O free.
- Changes to block validation, state transitions, or genesis serialization need
  a test in the same PR.
- New RPC routes go in `crates/node/src/rpc.rs` and get a row in the README
  route table.
- New CLI commands go in `crates/cli/src/cmd.rs`, namespaced under an existing
  group unless there is a clear reason not to.

## Good first issues

- README typos, clearer error messages, better `--help` text
- RPC pagination edge cases in `/blocks`
- More CLI tests around `tx status` and `pow verify`
- Benchmarks that run on one core
- Docs: a page for your language, or a diagram of the slot lifecycle

## Devnet etiquette

- The public devnet is a test network: no value, no promises, wipes allowed.
- Do not expose the faucet on a host you care about; it is unauthenticated by
  design.
- Announce destructive changes (genesis resets, port changes) before merging.

## Commits

- One logical change per commit.
- Imperative subject, body explains *why*.
- `cargo fmt` before committing.

## License

MIT. By contributing you agree your changes are licensed under the same terms.

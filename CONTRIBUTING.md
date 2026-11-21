# Contributing

Chronyx is a from-scratch Rust chain. Small, readable changes beat big clever
ones.

## Build and test

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

CI runs exactly those four commands. If they pass locally, the PR should pass.

## Layout

| Crate | What lives there |
|---|---|
| `crates/core` | PoH, PoA, PoW, state, blocks — pure, no I/O |
| `crates/node` | ledger, TCP gossip, axum RPC, slot clock |
| `crates/cli` | the `chronyx` binary |

Rules of thumb:

- `chronyx-core` must stay deterministic and I/O free.
- Changes to block validation, state transitions, or genesis serialization
  need a test in the same PR.
- New RPC routes go in `crates/node/src/rpc.rs` and get a row in the README
  route table.
- New CLI commands go in `crates/cli/src/cmd.rs`, namespaced under an existing
  group unless there is a clear reason not to.

## Good first issues

- README typos, clearer error messages, better `--help` text
- RPC pagination edge cases in `/blocks`
- More CLI tests around `tx status` and `pow verify`
- Benchmarks that run on one core

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

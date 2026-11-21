# Security policy

Chronyx is a **devnet**. It is not audited, it holds no value, and you should
assume it can lose or corrupt state. Reports are still welcome — devnet bugs
are exactly the ones worth fixing before any of this matters.

## Reporting

Please do **not** open a public issue for a vulnerability. Use GitHub's private
reporting flow instead:

**[Report a vulnerability](https://github.com/hynxilona/chronyx/security/advisories/new)**

Include what you did, what you expected, what happened, and the commit you
tested. A repro against a local devnet is ideal.

## In scope

- Consensus: PoH tick chain, PoA slot/leader/vote rules, PoW admission checks
- Block and transaction validation: merkle root, signatures, nonces, balances
- Ledger: replay on restart, catch-up from peers, fork handling
- RPC and faucet: authentication assumptions, nonce reuse, resource exhaustion
- CLI: key handling, path traversal, unsafe file permissions

## Out of scope

- Anything requiring physical access to the machine
- Denial of service through sheer traffic volume
- The faucet being open — that is deliberate on devnet
- Reports that only affect a version older than `main`

## What to expect

This is a one-maintainer project funded by nobody, so there is no SLA. Reports
get an acknowledgement when they are read, and credit in the fix commit unless
you ask otherwise. There is no bug bounty.

#!/usr/bin/env bash
# Monetary-policy guard.
#
# The chain has a fixed supply: 600,000,000 CHRONYX, every unit created in
# genesis. That promise is only worth something if it is enforced by CI and not
# by good intentions, so this script fails a build when:
#
#   1. something outside the account state creates quanta,
#   2. the supply constants move,
#   3. a mint/inflate-style function appears anywhere.
#
# If a change legitimately needs an exception, it is not a code change: it is a
# monetary-policy change. Open an issue, get it reviewed, then update this
# script in the same PR so the diff is impossible to miss.

set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
fail=0

say() { printf '%s\n' "$*"; }

# 1. Quanta may only be created in the account state module (once from genesis
#    allocations, once as the receiving half of a transfer) and in test code,
#    which never ships in a node binary.
while IFS= read -r hit; do
    case "$hit" in
        crates/core/src/state.rs:*|crates/core/tests/*) ;;
        *)
            say "FAIL  quanta created outside state.rs: $hit"
            fail=1
            ;;
    esac
done < <(grep -rn --include='*.rs' -E '\.credit\(' crates || true)

# 2. The supply constants are the policy. They may not drift.
if ! grep -qE 'pub const TOTAL_SUPPLY_CHRONYX: u64 = 600_000_000;' crates/core/src/genesis.rs; then
    say "FAIL  TOTAL_SUPPLY_CHRONYX is not 600_000_000"
    fail=1
fi
if ! grep -qE 'pub const QUANTA_PER_CHRONYX: u64 = 1_000_000_000;' crates/core/src/genesis.rs; then
    say "FAIL  QUANTA_PER_CHRONYX is not 1_000_000_000"
    fail=1
fi

# 3. No minting surfaces, whatever they are called.
if hits=$(grep -rnE --include='*.rs' 'fn[[:space:]]+(mint|mint_to|inflate|seigniorage|emission)[[:space:]]*\(' crates); then
    say "FAIL  mint-like function found:"
    say "$hits"
    fail=1
fi

if [ "$fail" -eq 0 ]; then
    say "ok  supply guard: no minting paths, constants intact"
fi
exit "$fail"

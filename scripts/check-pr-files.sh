#!/usr/bin/env bash
# Review guard for pull requests.
#
# A PR that touches a chain should be readable in one sitting. This flags what
# a reviewer would otherwise have to notice by eye:
#
#   * binary files added or removed (the usual place malware hides),
#   * very large diffs,
#   * changes to the files that decide balances, consensus or the supply.
#
# It fails on binaries — this project does not accept them — and on nothing
# else. The rest is context for the human doing the review.

set -euo pipefail

base="${BASE_SHA:-}"
if [ -z "$base" ]; then
    echo "ok  no BASE_SHA, nothing to compare (not a pull request)"
    exit 0
fi

echo "diff against $base"

binaries=0
while IFS=$'\t' read -r added removed path; do
    if [ "$added" = "-" ] || [ "$removed" = "-" ]; then
        echo "FAIL  binary file in diff: $path"
        binaries=1
        continue
    fi
    total=$((added + removed))
    if [ "$total" -gt 800 ]; then
        echo "note  large diff: $path (+$added/-$removed)"
    fi
done < <(git diff --numstat "$base"...HEAD)

sensitive='crates/core/src/(genesis|state|types|poa|poh|pow)\.rs|crates/node/src/(ledger|node)\.rs|crates/core/tests/invariants\.rs'
echo
echo "files under consensus / balance control:"
changed=$(git diff --name-only "$base"...HEAD | grep -E "$sensitive" || true)
if [ -n "$changed" ]; then
    echo "$changed" | sed 's/^/  /'
    echo
    echo "These need a CODEOWNERS review and a sentence in the PR about what"
    echo "changed and why it cannot move anyone's balance."
else
    echo "  (none)"
fi

exit "$binaries"

# Funding

Chronyx has no token sale, no presale, no investors, and no revenue. The devnet
token is worth nothing and will stay worth nothing. Anything raised here buys
time and audits, and every unit of it is accounted for in public.

If that sounds like a strange pitch for a chain, good — it is the only version
of this that is honest.

## What mainnet actually costs

| Item | Realistic range | Why it is on the list |
|---|---|---|
| External audit, consensus + ledger | $25k–$60k | The code that decides balances gets read by someone who is paid to break it |
| Second audit, RPC + crypto edges | $8k–$20k | Faucet, nonce handling, key parsing, DoS surface |
| Public testnet infra, 6 months | $2k–$5k | 4–8 small VPS + backups + monitoring |
| Mainnet bootstrap validators, 12 months | $6k–$12k | Operators should not pay out of pocket to keep the chain alive |
| Domain, signing keys, hardware wallet, incident buffer | $1k–$2k | Boring, necessary |
| **Floor to launch responsibly** | **~$45k** | Below this, the honest answer is "wait" |

No salaries are in that table. Right now the work is unpaid.

## Where funding comes from, in order of preference

1. **GitHub Sponsors** — recurring, public, no strings. Once the account is
   enrolled the Sponsor button appears on the repo.
2. **Direct grants and ecosystem funds** — applied for with the audit quote in
   hand, not with a pitch deck.
3. **Validator sponsorships** — a company pays for a bootstrap validator slot
   for a year and gets its name in `cluster validators` and the README. No
   influence over the protocol, ever.
4. **Bounties** — small, specific, paid on merge: an RPC route, a fuzz target, a
   client library. Cheapest way to get useful work done.
5. **Donations** — one address, published here, every inflow listed.

Not on the list: token sales, private allocations, "strategic partners" who
expect supply, or anything that makes a devnet token tradeable.

## Sponsor tiers

| | Amount | What it buys | What it does not buy |
|---|---|---|---|
| Supporter | $5–25 / mo | The site stays up, CI stays green | Nothing else |
| Backer | $100 / mo | One audit week per quarter | A logo, if you want it |
| Validator sponsor | $500 / mo | A bootstrap slot for a year | A say in consensus |
| Audit partner | one-off | Your name on the audit report | Any protocol control |

Sponsors get their name in `docs/SPONSORS.md` (opt-in), never in the protocol.

## Transparency

- Every expense over $100 gets a line in `docs/SPONSORS.md` with date, amount,
  and purpose.
- Every audit report is published in full, including the parts that make the
  maintainer look bad.
- If the money runs out, that gets said out loud instead of quietly shipping a
  half-audited mainnet.

## What the money is *not* for

- Marketing spend. Posts are free; the work is the marketing.
- Paying for listings, "partnerships", or market makers.
- Anything that requires the token to have a price.

## Current status

Unfunded. Applying to GitHub Sponsors and two grant programs; audit quotes
requested. This file is updated the day anything changes, not when it is
convenient.

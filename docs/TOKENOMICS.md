# CHRONYX tokenomics (v1 devnet)

> Devnet only. Nothing here is an offering, a promise of value, or financial
> advice. CHRONYX on `chronyx-devnet-1` is a test asset with no market.

## Supply

| | CHRONYX |
|---|---:|
| Total supply (fixed) | **600,000,000** |
| Inflation | **0%** |
| Runtime minting | none |
| Presale / ICO | none |

Solana's live supply is roughly **638M total** and **592M circulating** — 600M
sits between the two and is easy to quote. Unlike Solana, Chronyx v1 has no
disinflation schedule: the supply is fixed in genesis and can never grow. If
coins are lost, circulating supply only falls.

Base unit: **1 CHRONYX = 1,000,000,000 quanta** (`u64`). Every amount in the
RPC and CLI is quoted in quanta unless it says otherwise.

## Where the 600M sits on devnet

`chronyx devnet init` writes a genesis with exactly 600M:

| Holder | CHRONYX | Purpose |
|---|---:|---|
| Devnet faucet | 500,000,000 | funds `airdrop` requests, 1,000 CHRONYX each |
| 4 genesis validators | 25,000,000 each (100,000,000) | bootstrap balances for testing transfers |

The node that runs `devnet init` gets one of those validator slots; its 25M is
moved out of the faucet, so the total stays at 600M.

## Planned mainnet split (not live)

Same 600M cap, different holders:

| Bucket | Share | CHRONYX | Notes |
|---|---:|---:|---|
| Community + validator incentives | 55% | 330,000,000 | faucet, testnet rewards, grants |
| Validator bootstrap | 15% | 90,000,000 | distributed with the validator set |
| Treasury | 15% | 90,000,000 | 3-of-5 multisig, public spend log |
| Team | 10% | 60,000,000 | 1-year cliff, 4-year linear vest |
| Liquidity | 5% | 30,000,000 | market making, if a market ever exists |

Nothing above is implemented in v1. Genesis only knows flat pubkey → quanta
allocations; vesting and multisig are v2 work. Any mainnet launch would ship
its own genesis and its own audit.

## Why fixed instead of Solana's inflation

- PoA finality means validators are known; they do not need block rewards to
  be paid in newly minted coins in v1.
- PoW is used for **admission**, not for security spend, so there is no hash
  subsidy to fund.
- A fixed cap is trivial to verify: sum the genesis allocations, compare to
  600,000,000 CHRONYX. No inflation schedule to trust.

If v2 adds staking rewards, they should come from the treasury bucket rather
than from new minting, so the cap stays honest.

## Verifying supply yourself

```bash
chronyx devnet info      # supply 600000000
curl -s localhost:8899/supply
```

`/supply` returns `quanta` (total applied supply) and
`quanta_per_chronyx`. On a fresh devnet it must equal `600000000000000000`.

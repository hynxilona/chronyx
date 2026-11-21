# Validators

A validator is a node that signs blocks. On Chronyx that means: it holds one of
the genesis identities, it is scheduled by the PoA leader rotation, and its vote
is what makes a slot final.

This page is the honest version. Read the **Status** section before you buy a
server.

## Status (devnet v1)

| | |
|---|---|
| Chain | `chronyx-devnet-1` |
| Genesis | `37808ee8935440fdcc90ec86c2962cf803da656be4a1ad596eb7bff279a083cd` |
| Validators | 4, fixed at genesis |
| Slot | 400 ms, one block every 4th slot per validator |
| Rewards | none — devnet has no inflation and no fees |
| Admission | manual, by genesis, for now |

You can run a **follower node** today, in about two minutes. It syncs, gossips,
serves RPC, and does everything except sign blocks.

Becoming a **validator** currently means being written into the genesis file
before a devnet reset. That is the one part of the design that is not finished:
PoW admission — mine a proof, submit it, get scheduled — is the next milestone.
Until it ships, validator slots are handed out by the maintainer, and a restart
is announced in the repo first.

## Run a follower

```bash
chronyx devnet join http://SEED_HOST:8899 --peer SEED_HOST:9000 --dir ./my-node
chronyxd --config ./my-node/node.toml
```

Check the genesis hash the join prints against the one above. Different hash,
different chain — stop and ask.

Then verify from your own node rather than trusting anyone:

```bash
chronyx -u http://127.0.0.1:8899 cluster health
chronyx -u http://127.0.0.1:8899 cluster supply
chronyx -u http://127.0.0.1:8899 cluster validators
```

`cluster supply` should read `600000000000000000` quanta. It does that only if
your node replayed the same ledger as everyone else.

## What a validator box looks like

- 2 vCPU, 4 GB RAM, 40 GB SSD is enough for devnet. The node is single-threaded
  per component and does not need a GPU.
- A stable public IP with TCP 9000 reachable; 8899 for RPC if you want to expose
  it (behind a proxy, with rate limits).
- A systemd unit or a container, with `Restart=always`. The ledger is append-only
  `blocks.jsonl` — restart is safe, replay is tested.
- The identity key file. Back it up, never paste it in an issue.

## Applying for a slot

1. Open an issue titled `validator application: <handle>`.
2. Include: the machine region and specs, uptime commitment, the Ed25519
   identity pubkey you generated (`chronyx keypair new --out ./validator.json`),
   and whether you want your name shown in `cluster validators`.
3. Generate the PoW proof now, even though admission is manual. It is the thing
   that will matter after v1, and it proves your box can do the work:

   ```bash
   chronyx pow mine --difficulty 20
   chronyx pow verify <proof>
   ```

4. Slots for the current devnet are full at 4. Applications are queued for the
   next reset, and applicants get pinged before it happens.

## What validators get

Devnet: a line in `cluster validators`, a name in the repo, and the first look
at the mainnet bootstrap list. No tokens, no rewards, no promises.

Mainnet, when it exists: the plan is a 15% bootstrap allocation split across the
operators who were actually up during devnet — not sold, not airdropped to
strangers. Details stay in `docs/TOKENOMICS.md` and change only by public
proposal.

## Operating notes

- Watch `cluster peers`. Two is healthy on a small devnet; one means you are
  talking to a single neighbour and should not trust your view.
- The node logs every sealed slot. If your node stops sealing, check the clock —
  slots are wall-clock anchored and a drifting host will sit outside the window.
- Do not run two nodes with the same identity. Double-signing is the one mistake
  that has no good story afterwards.

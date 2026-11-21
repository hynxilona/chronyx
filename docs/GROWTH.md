# Launch and growth kit

Everything needed to put Chronyx in front of people without sounding like a
token launch. Post copy lives in `docs/SOCIAL.md`; this file is the checklist.

Ground rule for every post: **devnet, no value, not audited.** Say it before
someone asks. It is also true, which makes it easy.

## 1. Handles to claim

Claim the same name everywhere, today, even on platforms you will not use yet.
Consistency is what makes a project look real.

| Platform | Handle to try | Notes |
|---|---|---|
| GitHub | `hynxilona` (have) | Rename org later only if you want `chronyx` |
| X | `chronyx`, `chronyxdev`, `getchronyx` | Whichever is free; keep it short |
| Bluesky | `chronyx.dev` or `chronyx` | Domain handle is worth it |
| Mastodon | `hachyderm.io` or `fosstodon.org` | Rust people live on the second |
| Reddit | `u/chronyx_dev` | Reddit punishes new accounts; age it before posting |
| Discord | server `Chronyx` | Invite link on the site |
| Telegram | `chronyxdev` | Announcement-only channel |
| YouTube | `@chronyx` | For a 90-second demo later, not now |
| Hacker News | any | No profile needed; karma does the work |
| Dev.to / Hashnode | `chronyx` | Repost the technical write-ups |
| Product Hunt | later | Only when there is something to try |

**Domain:** `chronyx.dev` if free, else `chronyx.xyz` / `getchronyx.com`.
Point it at the Pages site with a CNAME — a real domain is the single biggest
credibility upgrade available for $10/year.

## 2. Profile text

Short bio (X, Bluesky, Mastodon — under 160 chars):

> A Rust blockchain built from scratch. PoH orders, PoA finalizes, PoW admits.
> Public devnet. Not audited, not mainnet, no value.

GitHub description (have):

> PoH orders. PoA finalizes. PoW admits. A Rust blockchain built from scratch.

Longer bio (Reddit, Dev.to, Discord):

> Chronyx is a from-scratch Rust chain: no framework, no VM, no ERC-20. Proof of
> History gives every node the same order, PoA gives 400 ms finality, PoW is how
> validators earn a slot. Fixed 600,000,000 supply, devnet only, MIT.

Link-in-bio order: site → GitHub → release → docs/VALIDATORS.md → Discord.

## 3. Images

| Use | Size | File |
|---|---|---|
| X / Bluesky banner | 1500×500 | `assets/social/x-banner.png` |
| Avatar (all platforms) | 400×400 | `assets/social/avatar-400.png` |
| GitHub social preview | 1200×630 | `assets/chronyx-og.png` |
| Discord server banner | 960×540 | crop of the X banner |
| Open Graph for pages | 1200×630 | `assets/chronyx-og.png` |

Upload the avatar first, the banner second. A profile with the default egg and
a good banner reads worse than the reverse.

## 4. Launch sequence

Post in this order. Each step assumes the one before it exists.

**Day 0 — the ground is ready**
- Repo public, release with binaries, site live, Social preview set
- Claim handles, upload avatar + banner, set bios and links
- Pin one post: the 6-tweet thread from `docs/SOCIAL.md`

**Day 1 — the people who already care**
- Post the thread on X, Bluesky, Mastodon (same text, no cross-posting tools)
- Reply to your own thread with the `git clone` block, not a link dump
- Post in the Rust Discord `#showcase` channel and in `r/rust` (weekday, 14:00 UTC)

**Day 2 — builders**
- Open 5 `good first issue`s and link them from the thread
- Post the validator call in `r/rust`, `r/opensource`, and two Rust Discords
- Submit to [This Week in Rust](https://this-week-in-rust.org/) — the project
  section is free and read by exactly the right people

**Day 3 — the wider internet**
- Show HN, Tuesday–Thursday, 13:00–15:00 UTC. Title is in `docs/SOCIAL.md`.
  Stay in the thread for six hours and answer everything, including the hostile
  comments about consensus design.
- Lobsters (`/newest`), Dev.to cross-post

**Day 4–7 — proof of life**
- Post one real artifact per day: the PoH demo GIF, the supply invariant test,
  the two-node slot log, the binary download numbers
- DM 10 Rust developers individually. Not a blast: one sentence about what they
  build, one about what you need reviewed. Ask for a review, not a follow.
- Submit `awesome-rust` and `awesome-blockchain` PRs

**Week 2+**
- Weekly devlog post: what merged, what broke, what is next
- Every merge that matters becomes a post. The repo is the content pipeline.
- Product Hunt only after a public testnet with a working explorer.

## 5. Where to post, and the rules

| Place | Fits | Do not |
|---|---|---|
| r/rust | The Rust engineering story | Mention price, tokens, or "investment" once |
| r/opensource | The contribution model | Ask for stars |
| Hacker News | "Show HN" with a real artifact | Ask for upvotes, ever — instant ban |
| Lobsters | Systems design discussion | Post twice in a week |
| This Week in Rust | Project updates | Spam every week with nothing new |
| Rust Discord `#showcase` | Demo + repo | Drop and run |
| r/cryptotechnology | Consensus design only | Call it an investment |
| X crypto circles | Avoid until mainnet | Paid shills, engagement pods |

If a community bans self-promotion, respect it the first time. One ban follows
you across the whole launch.

## 6. Recruiting validators

Target: 6–8 operators for the next devnet reset, 4 of them outside your
timezone. A chain whose validators all sleep at the same time is a demo.

Where: Rust Discords, `r/selfhosted`, `r/homelab`, university blockchain clubs,
the `#validators` channels of chains you do not compete with.

Template (edit the first line every time, or it reads as spam):

> Hey — you run <thing>, so you probably have a box doing nothing. I built a
> Rust chain from scratch (PoH ordering, PoA finality, PoW admission) and I am
> looking for 4 more devnet validators to prove the set is not just me.
> 2 vCPU, 400 ms slots, no rewards yet, your name in `cluster validators`. The
> whole onboarding is two commands: <link to docs/VALIDATORS.md>. If it breaks,
> you get to tell me exactly how.

## 7. Numbers worth posting

Only post numbers the reader can verify themselves:

- `cluster supply` → `600000000000000000`
- `cluster validators` → 4 identities, quorum 3
- slot time from the `/status` payload → 400
- test count from `cargo test` → 35
- release asset sizes from the GitHub release page

Never quote a TPS number without the command that produced it.

## 8. What not to do

- No paid ads. $0 budget is a feature; say it.
- No fake GitHub stars, no bought followers, no engagement pods.
- No "partnership" announcements with projects that have not agreed.
- No roadmap dates. "When it is audited" is the only honest answer.
- No airdrop farming bait. It attracts exactly the users you do not want.

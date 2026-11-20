use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{anyhow, Context};
use clap::{Parser, Subcommand};
use ed25519_dalek::{Signature, Signer, Verifier, VerifyingKey};
use serde::Serialize;
use serde_json::{json, Value};

use tachyon_core::genesis::{
    parse_key, Genesis, GenesisAllocation, GenesisValidator, LAMPORTS_PER_TACHYON,
};
use tachyon_core::poh::PohRecorder;
use tachyon_core::{pow, Block, KeypairFile, Transaction};

use crate::cfg::{self, home, CliConfig};
use crate::net;

#[derive(Parser, Debug)]
#[command(name = "tachyon", version, about = "Tachyon devnet CLI")]
pub struct Cli {
    /// RPC endpoint. Overrides the stored config for this invocation.
    #[arg(long, short = 'u', global = true, env = "TACHYON_RPC")]
    pub url: Option<String>,
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand, Debug)]
pub enum Cmd {
    /// Print CLI version.
    Version,
    /// Manage CLI settings.
    Config {
        #[command(subcommand)]
        cmd: ConfigCmd,
    },
    /// Create, inspect, sign with, and verify keypairs.
    Keypair {
        #[command(subcommand)]
        cmd: KeypairCmd,
    },
    /// Create and edit genesis files.
    Genesis {
        #[command(subcommand)]
        cmd: GenesisCmd,
    },
    /// Talk to a running node.
    Cluster {
        #[command(subcommand)]
        cmd: ClusterCmd,
    },
    /// Inspect accounts and balances.
    Account {
        #[command(subcommand)]
        cmd: AccountCmd,
    },
    /// Request devnet funds from the faucet.
    Airdrop { pubkey: String, lamports: u64 },
    /// Sign and submit a transfer.
    Transfer {
        keypair: PathBuf,
        to: String,
        lamports: u64,
    },
    /// Transaction utilities.
    Tx {
        #[command(subcommand)]
        cmd: TxCmd,
    },
    /// Inspect blocks.
    Block {
        #[command(subcommand)]
        cmd: BlockCmd,
    },
    /// Inspect the PoH clock.
    Poh {
        #[command(subcommand)]
        cmd: PohCmd,
    },
    /// Mine and verify PoW admission proofs.
    Pow {
        #[command(subcommand)]
        cmd: PowCmd,
    },
    /// Validator operations.
    Validator {
        #[command(subcommand)]
        cmd: ValidatorCmd,
    },
    /// Bootstrap and join a devnet.
    Devnet {
        #[command(subcommand)]
        cmd: DevnetCmd,
    },
    /// Local performance measurements.
    Bench {
        #[command(subcommand)]
        cmd: BenchCmd,
    },
    /// Check your local setup.
    Doctor,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCmd {
    Get { key: String },
    Set { key: String, value: String },
    Show,
    Path,
    Reset,
}

#[derive(Subcommand, Debug)]
pub enum KeypairCmd {
    New {
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        force: bool,
    },
    Show {
        path: Option<PathBuf>,
    },
    Address {
        path: Option<PathBuf>,
    },
    Sign {
        path: Option<PathBuf>,
        message: String,
    },
    Verify {
        pubkey: String,
        message: String,
        signature: String,
    },
    List,
}

#[derive(Subcommand, Debug)]
pub enum GenesisCmd {
    New {
        #[arg(long, default_value = "genesis.json")]
        out: PathBuf,
        #[arg(long)]
        chain_id: Option<String>,
        #[arg(long, default_value_t = 3)]
        validators: usize,
    },
    Show {
        #[arg(default_value = "genesis.json")]
        path: PathBuf,
    },
    Hash {
        #[arg(default_value = "genesis.json")]
        path: PathBuf,
    },
    Validate {
        #[arg(default_value = "genesis.json")]
        path: PathBuf,
    },
    AddValidator {
        #[arg(default_value = "genesis.json")]
        path: PathBuf,
        pubkey: String,
        #[arg(long, default_value = "validator")]
        name: String,
        #[arg(long, default_value_t = 500)]
        commission_bps: u16,
    },
    AddAllocation {
        #[arg(default_value = "genesis.json")]
        path: PathBuf,
        pubkey: String,
        lamports: u64,
    },
}

#[derive(Subcommand, Debug)]
pub enum ClusterCmd {
    Info,
    Health,
    Version,
    Slot,
    BlockHeight,
    Validators,
    Peers,
    Genesis,
    Supply,
}

#[derive(Subcommand, Debug)]
pub enum AccountCmd {
    Balance { pubkey: String },
    Show { pubkey: String },
    List,
}

#[derive(Subcommand, Debug)]
pub enum TxCmd {
    New {
        keypair: PathBuf,
        to: String,
        lamports: u64,
    },
    Send {
        file: PathBuf,
    },
    Status {
        hash: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum BlockCmd {
    Get {
        slot: u64,
    },
    Height,
    List {
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
}

#[derive(Subcommand, Debug)]
pub enum PohCmd {
    Head,
    Ticks,
}

#[derive(Subcommand, Debug)]
pub enum PowCmd {
    Mine {
        #[arg(long, default_value_t = 16)]
        difficulty: u32,
        #[arg(long, default_value = "tachyon")]
        seed: String,
    },
    Verify {
        seed: String,
        nonce: u64,
        difficulty: u32,
    },
}

#[derive(Subcommand, Debug)]
pub enum ValidatorCmd {
    Identity,
    Info { pubkey: String },
    List,
    Join { keypair: Option<PathBuf> },
}

#[derive(Subcommand, Debug)]
pub enum DevnetCmd {
    Init {
        #[arg(long)]
        dir: Option<PathBuf>,
        #[arg(long, default_value_t = 3)]
        validators: usize,
    },
    Join {
        seed: String,
        #[arg(long)]
        dir: Option<PathBuf>,
        #[arg(long)]
        peer: Option<String>,
    },
    Publish,
    Info,
}

#[derive(Subcommand, Debug)]
pub enum BenchCmd {
    Pow {
        #[arg(long, default_value_t = 16)]
        difficulty: u32,
        #[arg(long, default_value_t = 20)]
        iterations: u32,
    },
    Poh {
        #[arg(long, default_value_t = 100_000)]
        ticks: u64,
    },
}

#[derive(Serialize)]
struct NodeToml {
    rpc: String,
    p2p: String,
    data_dir: String,
    genesis_path: String,
    identity_path: String,
    peers: Vec<String>,
    faucet_enabled: bool,
}

pub fn run(cmd: Cmd, url: Option<String>) -> anyhow::Result<()> {
    let mut cfg = CliConfig::load();
    if let Some(url) = url {
        cfg.rpc_url = url;
    }

    match cmd {
        Cmd::Version => {
            println!("tachyon-cli {}", env!("CARGO_PKG_VERSION"));
            println!("core        {}", env!("CARGO_PKG_VERSION"));
        }

        Cmd::Config { cmd } => config_cmd(&cfg, cmd)?,
        Cmd::Keypair { cmd } => keypair_cmd(&cfg, cmd)?,
        Cmd::Genesis { cmd } => genesis_cmd(cmd)?,
        Cmd::Cluster { cmd } => cluster_cmd(&cfg, cmd)?,
        Cmd::Account { cmd } => account_cmd(&cfg, cmd)?,

        Cmd::Airdrop { pubkey, lamports } => {
            let resp = net::post(
                &cfg,
                "/faucet",
                json!({ "pubkey": pubkey, "lamports": lamports }),
            )?;
            print_json(&resp);
        }

        Cmd::Transfer {
            keypair,
            to,
            lamports,
        } => transfer(&cfg, &keypair, &to, lamports)?,

        Cmd::Tx { cmd } => tx_cmd(&cfg, cmd)?,
        Cmd::Block { cmd } => block_cmd(&cfg, cmd)?,
        Cmd::Poh { cmd } => poh_cmd(&cfg, cmd)?,
        Cmd::Pow { cmd } => pow_cmd(cmd)?,
        Cmd::Validator { cmd } => validator_cmd(&cfg, cmd)?,
        Cmd::Devnet { cmd } => devnet_cmd(&cfg, cmd)?,
        Cmd::Bench { cmd } => bench_cmd(cmd)?,
        Cmd::Doctor => doctor(&cfg)?,
    }

    Ok(())
}

// ---------------------------------------------------------------- helpers

fn print_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
    );
}

fn load_keypair(path: &Path) -> anyhow::Result<(KeypairFile, ed25519_dalek::SigningKey)> {
    let kp =
        KeypairFile::load(path).with_context(|| format!("reading keypair {}", path.display()))?;
    let sk = kp.signing_key().map_err(|e| anyhow!(e))?;
    Ok((kp, sk))
}

fn write_node_toml(
    dir: &Path,
    rpc: &str,
    p2p: &str,
    genesis: &Path,
    identity: &Path,
    peers: Vec<String>,
) -> anyhow::Result<()> {
    let config = NodeToml {
        rpc: rpc.to_string(),
        p2p: p2p.to_string(),
        data_dir: dir.display().to_string(),
        genesis_path: genesis.display().to_string(),
        identity_path: identity.display().to_string(),
        peers,
        faucet_enabled: true,
    };
    std::fs::write(dir.join("node.toml"), toml::to_string_pretty(&config)?)?;
    Ok(())
}

// ---------------------------------------------------------------- config

fn config_cmd(cfg: &CliConfig, cmd: ConfigCmd) -> anyhow::Result<()> {
    match cmd {
        ConfigCmd::Get { key } => match cfg.get(&key) {
            Some(v) => println!("{v}"),
            None => anyhow::bail!("unknown config key `{key}`"),
        },
        ConfigCmd::Set { key, value } => {
            let mut cfg = cfg.clone();
            cfg.set(&key, &value)?;
            cfg.save()?;
            println!("{key} = {value}");
        }
        ConfigCmd::Show => println!("{}", toml::to_string_pretty(cfg)?),
        ConfigCmd::Path => println!("{}", cfg::config_path().display()),
        ConfigCmd::Reset => {
            let fresh = CliConfig::default();
            fresh.save()?;
            println!("reset to defaults at {}", cfg::config_path().display());
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- keypair

fn keypair_cmd(cfg: &CliConfig, cmd: KeypairCmd) -> anyhow::Result<()> {
    match cmd {
        KeypairCmd::New { out, force } => {
            let path = out.unwrap_or_else(|| cfg.keypair_path.clone());
            if path.exists() && !force {
                anyhow::bail!(
                    "{} already exists (use --force to overwrite)",
                    path.display()
                );
            }
            let kp = KeypairFile::generate();
            kp.save(&path)?;
            println!("secret {}", kp.secret);
            println!("public {}", kp.public);
            println!("path   {}", path.display());
        }
        KeypairCmd::Show { path } | KeypairCmd::Address { path } => {
            let path = path.unwrap_or_else(|| cfg.keypair_path.clone());
            let (kp, _) = load_keypair(&path)?;
            println!("{}", kp.public);
        }
        KeypairCmd::Sign { path, message } => {
            let path = path.unwrap_or_else(|| cfg.keypair_path.clone());
            let (_, sk) = load_keypair(&path)?;
            let sig = sk.sign(message.as_bytes());
            println!("{}", hex::encode(sig.to_bytes()));
        }
        KeypairCmd::Verify {
            pubkey,
            message,
            signature,
        } => {
            let key = parse_key(&pubkey).map_err(|e| anyhow!(e))?;
            let vk = VerifyingKey::from_bytes(&key).map_err(|e| anyhow!(e))?;
            let bytes: [u8; 64] = hex::decode(&signature)
                .map_err(|e| anyhow!(e))?
                .try_into()
                .map_err(|_| anyhow!("signature must be 64 bytes"))?;
            let sig = Signature::from_bytes(&bytes);
            vk.verify(message.as_bytes(), &sig)
                .map_err(|_| anyhow!("invalid signature"))?;
            println!("valid");
        }
        KeypairCmd::List => {
            // Look where the CLI keeps its own keypair and in the default
            // home, so devnet identities show up even when `cli.toml` points
            // somewhere else.
            let mut dirs = vec![home()];
            if let Some(parent) = cfg.keypair_path.parent() {
                if !dirs.iter().any(|d| d.as_path() == parent) {
                    dirs.push(parent.to_path_buf());
                }
            }

            let mut found = 0;
            let mut seen = std::collections::HashSet::new();
            for dir in &dirs {
                let mut paths: Vec<_> = match std::fs::read_dir(dir) {
                    Ok(entries) => entries
                        .flatten()
                        .map(|e| e.path())
                        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
                        .collect(),
                    Err(_) => Vec::new(),
                };
                paths.sort();
                for path in paths {
                    let Ok(kp) = KeypairFile::load(&path) else {
                        continue;
                    };
                    if seen.insert(kp.public.clone()) {
                        println!("{}  {}", kp.public, path.display());
                        found += 1;
                    }
                }
            }
            if found == 0 {
                let listed: Vec<_> = dirs.iter().map(|d| d.display().to_string()).collect();
                println!("no keypairs in {}", listed.join(", "));
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- genesis

fn genesis_cmd(cmd: GenesisCmd) -> anyhow::Result<()> {
    match cmd {
        GenesisCmd::New {
            out,
            chain_id,
            validators,
        } => {
            let mut g = Genesis::devnet(validators);
            if let Some(id) = chain_id {
                g.chain_id = id;
            }
            g.validate().map_err(|e| anyhow!(e))?;
            std::fs::write(&out, g.to_json())?;
            println!("wrote   {}", out.display());
            println!("chain   {}", g.chain_id);
            println!("hash    {}", hex::encode(g.hash()));
            println!("validators {}", g.validators.len());
        }
        GenesisCmd::Show { path } => {
            let g = load_genesis(&path)?;
            println!("{}", g.to_json());
        }
        GenesisCmd::Hash { path } => {
            let g = load_genesis(&path)?;
            println!("{}", hex::encode(g.hash()));
        }
        GenesisCmd::Validate { path } => {
            let g = load_genesis(&path)?;
            g.validate().map_err(|e| anyhow!(e))?;
            println!("valid ({} validators)", g.validators.len());
        }
        GenesisCmd::AddValidator {
            path,
            pubkey,
            name,
            commission_bps,
        } => {
            let mut g = load_genesis(&path)?;
            parse_key(&pubkey).map_err(|e| anyhow!(e))?;
            g.validators.push(GenesisValidator {
                identity: pubkey,
                name,
                commission_bps,
            });
            g.validate().map_err(|e| anyhow!(e))?;
            std::fs::write(&path, g.to_json())?;
            println!("validators {}", g.validators.len());
            println!("hash       {}", hex::encode(g.hash()));
        }
        GenesisCmd::AddAllocation {
            path,
            pubkey,
            lamports,
        } => {
            let mut g = load_genesis(&path)?;
            parse_key(&pubkey).map_err(|e| anyhow!(e))?;
            g.allocations.push(GenesisAllocation { pubkey, lamports });
            g.validate().map_err(|e| anyhow!(e))?;
            std::fs::write(&path, g.to_json())?;
            println!("allocations {}", g.allocations.len());
            println!("hash        {}", hex::encode(g.hash()));
        }
    }
    Ok(())
}

fn load_genesis(path: &Path) -> anyhow::Result<Genesis> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    Genesis::from_json(&text).map_err(|e| anyhow!(e))
}

// ---------------------------------------------------------------- cluster

fn cluster_cmd(cfg: &CliConfig, cmd: ClusterCmd) -> anyhow::Result<()> {
    match cmd {
        ClusterCmd::Info => print_json(&net::get(cfg, "/status")?),
        ClusterCmd::Health => {
            println!("{}", net::get(cfg, "/health")?);
        }
        ClusterCmd::Version => {
            let status = net::get(cfg, "/status")?;
            println!("chain_id     {}", status["chain_id"]);
            println!("genesis_hash {}", status["genesis_hash"]);
            println!("slot_ms      {}", status["slot_ms"]);
        }
        ClusterCmd::Slot => {
            let status = net::get(cfg, "/status")?;
            println!("{}", status["slot"]);
        }
        ClusterCmd::BlockHeight => {
            let status = net::get(cfg, "/status")?;
            println!("{}", status["height"]);
        }
        ClusterCmd::Validators => print_json(&net::get(cfg, "/validators")?),
        ClusterCmd::Peers => print_json(&net::get(cfg, "/peers")?),
        ClusterCmd::Genesis => print_json(&net::get(cfg, "/genesis")?),
        ClusterCmd::Supply => print_json(&net::get(cfg, "/supply")?),
    }
    Ok(())
}

// ---------------------------------------------------------------- accounts

fn account_cmd(cfg: &CliConfig, cmd: AccountCmd) -> anyhow::Result<()> {
    match cmd {
        AccountCmd::Balance { pubkey } => {
            let resp = net::get(cfg, &format!("/balance/{pubkey}"))?;
            let lamports = resp["lamports"].as_u64().unwrap_or(0);
            println!(
                "{lamports} lamports ({:.9} TACHYON)",
                lamports as f64 / LAMPORTS_PER_TACHYON as f64
            );
        }
        AccountCmd::Show { pubkey } => print_json(&net::get(cfg, &format!("/account/{pubkey}"))?),
        AccountCmd::List => print_json(&net::get(cfg, "/accounts")?),
    }
    Ok(())
}

fn transfer(cfg: &CliConfig, keypair: &Path, to: &str, lamports: u64) -> anyhow::Result<()> {
    let (_, sk) = load_keypair(keypair)?;
    let from = hex::encode(sk.verifying_key().to_bytes());

    let account = net::get(cfg, &format!("/account/{from}"))?;
    let nonce = account["nonce"].as_u64().unwrap_or(0);
    let to_key = parse_key(to).map_err(|e| anyhow!(e))?;

    let tx = Transaction::sign(&sk, to_key, lamports, nonce);
    let body = serde_json::to_value(&tx)?;
    let resp = net::post(cfg, "/tx", body)?;
    print_json(&resp);
    Ok(())
}

// ---------------------------------------------------------------- tx / block

fn tx_cmd(cfg: &CliConfig, cmd: TxCmd) -> anyhow::Result<()> {
    match cmd {
        TxCmd::New {
            keypair,
            to,
            lamports,
        } => {
            let (_, sk) = load_keypair(&keypair)?;
            let from = hex::encode(sk.verifying_key().to_bytes());
            let account = net::get(cfg, &format!("/account/{from}"))?;
            let nonce = account["nonce"].as_u64().unwrap_or(0);
            let to_key = parse_key(&to).map_err(|e| anyhow!(e))?;
            let tx = Transaction::sign(&sk, to_key, lamports, nonce);
            println!("{}", serde_json::to_string_pretty(&tx)?);
        }
        TxCmd::Send { file } => {
            let text = std::fs::read_to_string(&file)?;
            let value: Value = serde_json::from_str(&text)?;
            print_json(&net::post(cfg, "/tx", value)?);
        }
        TxCmd::Status { hash } => {
            let all = net::get(cfg, "/blocks")?;
            let empty = Vec::new();
            let blocks = all["blocks"].as_array().unwrap_or(&empty);
            for block in blocks {
                if let Some(txs) = block["transactions"].as_array() {
                    for tx in txs {
                        if let Ok(parsed) = serde_json::from_value::<Transaction>(tx.clone()) {
                            if hex::encode(parsed.hash()) == hash {
                                print_json(&json!({
                                    "found": true,
                                    "tx": hash,
                                    "slot": block["header"]["slot"],
                                }));
                                return Ok(());
                            }
                        }
                    }
                }
            }
            print_json(&json!({ "found": false, "tx": hash }));
        }
    }
    Ok(())
}

fn block_cmd(cfg: &CliConfig, cmd: BlockCmd) -> anyhow::Result<()> {
    match cmd {
        BlockCmd::Get { slot } => {
            let value = net::get(cfg, &format!("/block/{slot}"))?;
            let block_value = value.get("block").cloned().unwrap_or_else(|| value.clone());
            match serde_json::from_value::<Block>(block_value) {
                Ok(block) => print_block(&block),
                // Fall back to raw JSON if the node returns something unexpected.
                Err(_) => print_json(&value),
            }
        }
        BlockCmd::Height => {
            let status = net::get(cfg, "/status")?;
            println!("{}", status["height"]);
        }
        BlockCmd::List { limit } => {
            let value = net::get(cfg, "/blocks")?;
            let empty = Vec::new();
            let raw = value["blocks"].as_array().unwrap_or(&empty);
            let blocks: Vec<Block> = raw
                .iter()
                .filter_map(|b| serde_json::from_value(b.clone()).ok())
                .collect();

            println!(
                "{:>10}  {:<16}  {:<16}  {:>4}  {:>13}",
                "slot", "hash", "leader", "txs", "time_ms"
            );
            for block in blocks.iter().rev().take(limit).rev() {
                let h = &block.header;
                println!(
                    "{:>10}  {:<16}  {:<16}  {:>4}  {:>13}",
                    h.slot,
                    short(&hex::encode(block.hash())),
                    short(&hex::encode(h.leader)),
                    block.transactions.len(),
                    h.timestamp_ms
                );
            }
        }
    }
    Ok(())
}

fn print_block(block: &Block) {
    let h = &block.header;
    println!("slot      {}", h.slot);
    println!("hash      {}", hex::encode(block.hash()));
    println!("leader    {}", hex::encode(h.leader));
    println!("parent    {}", hex::encode(h.parent));
    println!("poh       {}", hex::encode(h.poh_hash));
    println!("tx_root   {}", hex::encode(h.tx_root));
    println!("time      {} ms", h.timestamp_ms);
    println!("txs       {}", block.transactions.len());
    for tx in &block.transactions {
        println!(
            "  {}  {} -> {}  {} lamports  (nonce {})",
            short(&hex::encode(tx.hash())),
            short(&hex::encode(tx.from)),
            short(&hex::encode(tx.to)),
            tx.amount,
            tx.nonce
        );
    }
}

fn short(s: &str) -> String {
    s.chars().take(16).collect()
}

fn poh_cmd(cfg: &CliConfig, cmd: PohCmd) -> anyhow::Result<()> {
    let status = net::get(cfg, "/status")?;
    match cmd {
        PohCmd::Head => println!("{}", status["poh_hash"].as_str().unwrap_or("")),
        PohCmd::Ticks => println!("{}", status["ticks"]),
    }
    Ok(())
}

// ---------------------------------------------------------------- pow

fn pow_cmd(cmd: PowCmd) -> anyhow::Result<()> {
    match cmd {
        PowCmd::Mine { difficulty, seed } => {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(seed.as_bytes());
            let seed_hash: [u8; 32] = h.finalize().into();

            let start = Instant::now();
            let (nonce, hash) = pow::mine(&seed_hash, difficulty);
            println!("seed       {seed}");
            println!("seed_hash  {}", hex::encode(seed_hash));
            println!("difficulty {difficulty} bits");
            println!("nonce      {nonce}");
            println!("hash       {}", hex::encode(hash));
            println!("elapsed    {} ms", start.elapsed().as_millis());
            println!("verify     {}", pow::verify(&seed_hash, nonce, difficulty));
            println!();
            println!("check it with: tachyon pow verify {seed} {nonce} {difficulty}");
        }
        PowCmd::Verify {
            seed,
            nonce,
            difficulty,
        } => {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(seed.as_bytes());
            let seed_hash: [u8; 32] = h.finalize().into();
            let ok = pow::verify(&seed_hash, nonce, difficulty);
            println!("{}", if ok { "true" } else { "false" });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- validator

fn validator_cmd(cfg: &CliConfig, cmd: ValidatorCmd) -> anyhow::Result<()> {
    match cmd {
        ValidatorCmd::Identity => {
            let (kp, _) = load_keypair(&cfg.keypair_path)?;
            println!("{}", kp.public);
        }
        ValidatorCmd::Info { pubkey } => {
            let validators = net::get(cfg, "/validators")?;
            let empty = Vec::new();
            let list = validators["validators"].as_array().unwrap_or(&empty);
            let found = list.iter().find(|v| v["identity"] == pubkey);
            match found {
                Some(v) => print_json(v),
                None => println!("{pubkey} is not in the validator set"),
            }
        }
        ValidatorCmd::List => print_json(&net::get(cfg, "/validators")?),
        ValidatorCmd::Join { keypair } => {
            let path = keypair.unwrap_or_else(|| cfg.keypair_path.clone());
            let (kp, _) = load_keypair(&path)?;
            println!("Add this identity to genesis, then restart the cluster:");
            println!();
            println!("  tachyon genesis add-validator genesis.json {}", kp.public);
            println!();
            println!("PoW admission (automatic joining) lands in v2.");
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- devnet

fn devnet_cmd(cfg: &CliConfig, cmd: DevnetCmd) -> anyhow::Result<()> {
    match cmd {
        DevnetCmd::Init { dir, validators } => {
            let dir = dir.unwrap_or_else(home);
            std::fs::create_dir_all(&dir)?;

            let mut genesis = Genesis::devnet(validators);
            let kp = KeypairFile::generate();

            // The local identity must be in the set or this node never leads.
            genesis.validators.push(GenesisValidator {
                identity: kp.public.clone(),
                name: "local".to_string(),
                commission_bps: 500,
            });
            genesis.allocations.push(GenesisAllocation {
                pubkey: kp.public.clone(),
                lamports: 1_000 * LAMPORTS_PER_TACHYON,
            });

            genesis.validate().map_err(|e| anyhow!(e))?;
            let genesis_path = dir.join("genesis.json");
            std::fs::write(&genesis_path, genesis.to_json())?;

            let identity_path = dir.join("validator.json");
            kp.save(&identity_path)?;

            write_node_toml(
                &dir,
                "127.0.0.1:8899",
                "127.0.0.1:9000",
                &genesis_path,
                &identity_path,
                Vec::new(),
            )?;

            let mut cli = cfg.clone();
            cli.data_dir = dir.clone();
            cli.genesis_path = genesis_path.clone();
            cli.keypair_path = identity_path.clone();
            cli.save()?;

            println!("devnet initialised in {}", dir.display());
            println!("chain_id   {}", genesis.chain_id);
            println!("genesis    {}", hex::encode(genesis.hash()));
            println!("identity   {}", kp.public);
            println!();
            println!("start the node with:");
            println!("  tachyond --config {}", dir.join("node.toml").display());
        }

        DevnetCmd::Join { seed, dir, peer } => {
            let dir = dir.unwrap_or_else(home);
            std::fs::create_dir_all(&dir)?;

            let remote = CliConfig {
                rpc_url: seed.trim_end_matches('/').to_string(),
                ..cfg.clone()
            };
            let response = net::get(&remote, "/genesis")?;
            let genesis_value = response
                .get("genesis")
                .cloned()
                .ok_or_else(|| anyhow!("seed node returned no genesis"))?;
            let genesis: Genesis = serde_json::from_value(genesis_value)?;
            genesis.validate().map_err(|e| anyhow!(e))?;

            let genesis_path = dir.join("genesis.json");
            std::fs::write(&genesis_path, genesis.to_json())?;

            let kp = KeypairFile::generate();
            let identity_path = dir.join("validator.json");
            kp.save(&identity_path)?;

            let peers = peer.into_iter().collect::<Vec<_>>();
            write_node_toml(
                &dir,
                "127.0.0.1:8899",
                "127.0.0.1:9000",
                &genesis_path,
                &identity_path,
                peers,
            )?;

            let mut cli = cfg.clone();
            cli.data_dir = dir.clone();
            cli.genesis_path = genesis_path.clone();
            cli.keypair_path = identity_path.clone();
            cli.save()?;

            println!("joined {}", genesis.chain_id);
            println!("genesis  {}", hex::encode(genesis.hash()));
            println!("identity {}", kp.public);
            println!();
            println!("start the node with:");
            println!("  tachyond --config {}", dir.join("node.toml").display());
            println!();
            println!("tip: running a second node on this same machine? give it unique ports:");
            println!(
                "  tachyond --config {} --rpc 127.0.0.1:8900 --p2p 127.0.0.1:9001",
                dir.join("node.toml").display()
            );
        }

        DevnetCmd::Publish => {
            let g = load_genesis(&cfg.genesis_path)?;
            let hash = hex::encode(g.hash());
            println!("Publish your devnet");
            println!();
            println!("cluster id  {}", g.chain_id);
            println!("genesis     {hash}");
            println!("validators  {}", g.validators.len());
            println!();
            println!("1. Run a seed node on a public host:");
            println!("     tachyond --config node.toml --rpc 0.0.0.0:8899 --p2p 0.0.0.0:9000");
            println!("2. Open ports 8899 (RPC) and 9000 (p2p).");
            println!("3. Tell people to join (they should check the genesis hash above):");
            println!("     tachyon devnet join http://YOUR_HOST:8899 --peer YOUR_HOST:9000");
            println!("4. Fund them:");
            println!("     tachyon airdrop <their-pubkey> 1000000000");
            println!();
            println!("Never expose the faucet on a network with real value.");
        }

        DevnetCmd::Info => {
            let g = load_genesis(&cfg.genesis_path)?;
            println!("chain_id   {}", g.chain_id);
            println!("hash       {}", hex::encode(g.hash()));
            println!("slot_ms    {}", g.slot_ms);
            println!("ticks/slot {}", g.ticks_per_slot);
            println!("difficulty {}", g.pow_difficulty);
            println!("validators {}", g.validators.len());
            println!("faucet     {}", g.faucet.as_deref().unwrap_or("disabled"));
            println!("allocations {}", g.allocations.len());
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- bench

fn bench_cmd(cmd: BenchCmd) -> anyhow::Result<()> {
    match cmd {
        BenchCmd::Pow {
            difficulty,
            iterations,
        } => {
            let seed = [7u8; 32];
            let start = Instant::now();
            for _ in 0..iterations {
                let _ = pow::mine(&seed, difficulty);
            }
            let elapsed = start.elapsed().as_secs_f64();
            let per = elapsed / iterations.max(1) as f64;
            let hashes = (iterations as f64) * 2f64.powi(difficulty as i32);
            println!("iterations  {iterations}");
            println!("difficulty  {difficulty} bits");
            println!("total       {:.3} s", elapsed);
            println!("per proof   {:.1} ms", per * 1000.0);
            println!("hash rate   {:.0} H/s", hashes / elapsed.max(0.000_001));
        }
        BenchCmd::Poh { ticks } => {
            let start = Instant::now();
            let mut poh = PohRecorder::new([0u8; 32]);
            for _ in 0..ticks {
                poh.tick();
            }
            let elapsed = start.elapsed().as_secs_f64();
            println!("ticks      {ticks}");
            println!("total      {:.3} s", elapsed);
            println!(
                "rate       {:.0} ticks/s",
                ticks as f64 / elapsed.max(0.000_001)
            );
            println!("head       {}", hex::encode(poh.hash()));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- doctor

fn doctor(cfg: &CliConfig) -> anyhow::Result<()> {
    println!("tachyon doctor");
    println!();
    println!("config     {}", cfg::config_path().display());
    println!("rpc_url    {}", cfg.rpc_url);
    println!("keypair    {}", cfg.keypair_path.display());
    println!("genesis    {}", cfg.genesis_path.display());
    println!();

    check("keypair file", cfg.keypair_path.exists());
    check("genesis file", cfg.genesis_path.exists());

    if cfg.genesis_path.exists() {
        match load_genesis(&cfg.genesis_path) {
            Ok(g) => check(
                &format!("genesis valid ({} validators)", g.validators.len()),
                g.validate().is_ok(),
            ),
            Err(_) => check("genesis parses", false),
        }
    }

    match net::get(cfg, "/health") {
        Ok(_) => check("node reachable", true),
        Err(_) => check("node reachable", false),
    }

    if let Ok(status) = net::get(cfg, "/status") {
        println!();
        println!("chain_id   {}", status["chain_id"]);
        println!("slot       {}", status["slot"]);
        println!("height     {}", status["height"]);
        println!("validators {}", status["validators"]);
        println!("peers      {}", status["peers"]);
    }

    Ok(())
}

fn check(label: &str, ok: bool) {
    println!("[{}] {label}", if ok { "ok" } else { "--" });
}

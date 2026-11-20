//! `chronyxd` — the node binary.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

use chronyx_node::config::{default_home, NodeConfig};
use chronyx_node::{node, p2p, rpc, Node};

#[derive(Parser, Debug)]
#[command(name = "chronyxd", version, about = "Chronyx devnet node")]
struct Args {
    /// Load a node.toml config file.
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    rpc: Option<SocketAddr>,
    #[arg(long)]
    p2p: Option<SocketAddr>,
    #[arg(long)]
    data_dir: Option<PathBuf>,
    #[arg(long)]
    genesis: Option<PathBuf>,
    #[arg(long)]
    identity: Option<PathBuf>,
    /// Peer p2p address (repeatable).
    #[arg(long = "peer")]
    peers: Vec<SocketAddr>,
    /// Disable the devnet faucet.
    #[arg(long)]
    no_faucet: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    let mut config = match &args.config {
        Some(path) => NodeConfig::load(path)?,
        None => NodeConfig::devnet(default_home()),
    };

    if let Some(v) = args.rpc {
        config.rpc = v;
    }
    if let Some(v) = args.p2p {
        config.p2p = v;
    }
    if let Some(v) = args.data_dir {
        config.data_dir = v;
    }
    if let Some(v) = args.genesis.clone() {
        config.genesis_path = v;
    }
    if let Some(v) = args.identity.clone() {
        config.identity_path = v;
    }

    // A bare `--data-dir` should be enough: keep genesis and identity inside it
    // unless the caller pointed at them explicitly.
    if args.config.is_none() {
        if args.genesis.is_none() {
            config.genesis_path = config.data_dir.join("genesis.json");
        }
        if args.identity.is_none() {
            config.identity_path = config.data_dir.join("validator.json");
        }
    }
    if !args.peers.is_empty() {
        config.peers = args.peers;
    }
    if args.no_faucet {
        config.faucet_enabled = false;
    }

    let node = Arc::new(Node::open(config)?);

    {
        let status = node.status().await;
        info!(
            chain_id = %status.chain_id,
            genesis = %status.genesis_hash,
            validators = status.validators,
            quorum = status.quorum,
            is_validator = status.is_validator,
            "chronyx node up"
        );
    }

    tokio::spawn(node::run_slot_clock(node.clone()));

    {
        let p2p_node = node.clone();
        let addr = node.config.p2p;
        tokio::spawn(async move {
            if let Err(e) = p2p::serve(p2p_node, addr).await {
                tracing::error!(error = %e, "p2p server failed");
            }
        });
    }

    for peer in node.config.peers.clone() {
        let dial_node = node.clone();
        tokio::spawn(p2p::dial(dial_node, peer));
    }

    let rpc_addr = node.config.rpc;
    let app = rpc::router(node);
    let listener = tokio::net::TcpListener::bind(rpc_addr).await?;
    info!(rpc = %rpc_addr, "http listening");

    axum::serve(listener, app).await?;
    Ok(())
}

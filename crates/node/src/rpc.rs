//! HTTP RPC. Small surface, real answers.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use tachyon_core::genesis::parse_key;
use tachyon_core::{pow, Transaction};

use crate::node::{Node, Status};

pub fn router(node: Arc<Node>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/genesis", get(genesis))
        .route("/validators", get(validators))
        .route("/supply", get(supply))
        .route("/peers", get(peers))
        .route("/account/{pubkey}", get(account))
        .route("/accounts", get(accounts))
        .route("/balance/{pubkey}", get(balance))
        .route("/block/{slot}", get(block))
        .route("/blocks", get(blocks))
        .route("/tx/{hash}", get(tx_status))
        .route("/tx", post(submit_tx))
        .route("/faucet", post(faucet))
        .route("/mine", get(mine))
        .with_state(node)
}

type ApiError = (StatusCode, Json<Value>);

fn err(code: StatusCode, msg: impl Into<String>) -> ApiError {
    (code, Json(json!({ "error": msg.into() })))
}

async fn health(State(node): State<Arc<Node>>) -> Json<Value> {
    Json(json!({
        "ok": true,
        "chain_id": node.genesis.chain_id,
        "uptime_secs": node.started.elapsed().as_secs(),
    }))
}

async fn status(State(node): State<Arc<Node>>) -> Json<Status> {
    Json(node.status().await)
}

async fn genesis(State(node): State<Arc<Node>>) -> Json<Value> {
    Json(json!({
        "hash": hex::encode(node.genesis_hash),
        "genesis": node.genesis,
    }))
}

async fn validators(State(node): State<Arc<Node>>) -> Json<Value> {
    let current = node.height().await;
    let leader = node
        .validators
        .leader_for_slot(current + 1)
        .map(|k| hex::encode(k.to_bytes()));
    Json(json!({
        "count": node.validators.len(),
        "quorum": node.validators.quorum(),
        "next_leader": leader,
        "validators": node.genesis.validators,
    }))
}

async fn supply(State(node): State<Arc<Node>>) -> Json<Value> {
    let state = node.state.lock().await;
    Json(json!({
        "lamports": state.total_supply(),
        "accounts": state.len(),
        "lamports_per_tachyon": tachyon_core::LAMPORTS_PER_TACHYON,
    }))
}

async fn peers(State(node): State<Arc<Node>>) -> Json<Value> {
    Json(json!({ "connected": node.hub.count() }))
}

async fn account(
    State(node): State<Arc<Node>>,
    Path(pubkey): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let key = parse_key(&pubkey).map_err(|e| err(StatusCode::BAD_REQUEST, e))?;
    let state = node.state.lock().await;
    let account = state.account(&key);
    Ok(Json(json!({
        "pubkey": pubkey,
        "lamports": account.lamports,
        "nonce": account.nonce,
    })))
}

async fn balance(
    State(node): State<Arc<Node>>,
    Path(pubkey): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let key = parse_key(&pubkey).map_err(|e| err(StatusCode::BAD_REQUEST, e))?;
    let lamports = node.state.lock().await.balance(&key);
    Ok(Json(json!({ "pubkey": pubkey, "lamports": lamports })))
}

async fn accounts(State(node): State<Arc<Node>>) -> Json<Value> {
    let state = node.state.lock().await;
    let list: Vec<Value> = state
        .snapshot()
        .into_iter()
        .map(|(pubkey, account)| {
            json!({
                "pubkey": pubkey,
                "lamports": account.lamports,
                "nonce": account.nonce,
            })
        })
        .collect();
    Json(json!({ "count": list.len(), "accounts": list }))
}

async fn block(
    State(node): State<Arc<Node>>,
    Path(slot): Path<u64>,
) -> Result<Json<Value>, ApiError> {
    match node.block_at(slot).await {
        Some(block) => Ok(Json(json!({ "slot": slot, "block": block }))),
        None => Err(err(
            StatusCode::NOT_FOUND,
            format!("no block at slot {slot}"),
        )),
    }
}

#[derive(Deserialize)]
struct BlocksQuery {
    /// Start slot. Omit for the most recent page.
    from: Option<u64>,
    /// Page size, capped at the ring size.
    limit: Option<usize>,
}

/// Recent blocks by default; `?from=<slot>&limit=<n>` pages through the ledger.
async fn blocks(State(node): State<Arc<Node>>, Query(q): Query<BlocksQuery>) -> Json<Value> {
    let height = node.height().await;
    let limit = q.limit.unwrap_or(1024).clamp(1, 1024);

    let blocks = match q.from {
        Some(from) => {
            // blocks_from already caps at one page; outer limit lets callers narrow it.
            let mut page = node.blocks_from(from).await;
            page.truncate(limit);
            page
        }
        None => {
            let ring = node.blocks.lock().await;
            let start = ring.len().saturating_sub(limit);
            ring[start..].to_vec()
        }
    };

    Json(json!({
        "height": height,
        "count": blocks.len(),
        "blocks": blocks,
    }))
}

/// Look a transaction up in the persisted ledger, newest first.
async fn tx_status(
    State(node): State<Arc<Node>>,
    Path(hash): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let hash = hash.to_lowercase();
    let blocks = node
        .ledger
        .load()
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    for block in blocks.iter().rev() {
        for tx in &block.transactions {
            if hex::encode(tx.hash()) == hash {
                return Ok(Json(json!({
                    "found": true,
                    "tx": hash,
                    "slot": block.header.slot,
                })));
            }
        }
    }

    Ok(Json(json!({ "found": false, "tx": hash })))
}

async fn submit_tx(
    State(node): State<Arc<Node>>,
    Json(tx): Json<Transaction>,
) -> Result<Json<Value>, ApiError> {
    // Cheap pre-check so garbage never reaches the mempool.
    tx.verify()
        .map_err(|e| err(StatusCode::BAD_REQUEST, e.to_string()))?;
    let hash = hex::encode(tx.hash());
    node.mempool.lock().await.push(tx);
    Ok(Json(json!({ "accepted": true, "tx": hash })))
}

#[derive(Deserialize)]
struct FaucetRequest {
    pubkey: String,
    lamports: u64,
}

/// Per-request faucet cap: 1,000 TACHYON.
const FAUCET_MAX: u64 = 1_000 * tachyon_core::LAMPORTS_PER_TACHYON;

async fn faucet(
    State(node): State<Arc<Node>>,
    Json(req): Json<FaucetRequest>,
) -> Result<Json<Value>, ApiError> {
    let faucet_key = node
        .faucet_key
        .as_ref()
        .ok_or_else(|| err(StatusCode::FORBIDDEN, "faucet is disabled on this node"))?;

    if req.lamports == 0 {
        return Err(err(StatusCode::BAD_REQUEST, "lamports must be > 0"));
    }
    if req.lamports > FAUCET_MAX {
        return Err(err(
            StatusCode::BAD_REQUEST,
            format!("faucet caps each request at {} lamports", FAUCET_MAX),
        ));
    }

    let to = parse_key(&req.pubkey).map_err(|e| err(StatusCode::BAD_REQUEST, e))?;
    let from = faucet_key.verifying_key().to_bytes();

    // Nonce = last applied nonce + faucet txs already sitting in the mempool.
    // Without the second term, two users hitting the faucet inside the same
    // slot would get the same nonce and the second transfer would be dropped.
    let applied = node.state.lock().await.nonce(&from);
    let pending = {
        let mempool = node.mempool.lock().await;
        mempool.iter().filter(|tx| tx.from == from).count() as u64
    };

    let tx = Transaction::sign(faucet_key, to, req.lamports, applied + pending);
    let hash = hex::encode(tx.hash());
    node.mempool.lock().await.push(tx);

    Ok(Json(json!({
        "accepted": true,
        "lamports": req.lamports,
        "tx": hash,
    })))
}

async fn mine(State(node): State<Arc<Node>>) -> Json<Value> {
    let seed = { node.poh.lock().await.hash() };
    let difficulty = node.difficulty;

    let (nonce, hash, elapsed_ms) = tokio::task::spawn_blocking(move || {
        let start = std::time::Instant::now();
        let (nonce, hash) = pow::mine(&seed, difficulty);
        (nonce, hash, start.elapsed().as_millis())
    })
    .await
    .expect("mining task panicked");

    Json(json!({
        "difficulty": difficulty,
        "nonce": nonce,
        "hash": hex::encode(hash),
        "elapsed_ms": elapsed_ms,
    }))
}

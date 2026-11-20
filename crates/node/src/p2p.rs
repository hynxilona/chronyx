//! Minimal TCP gossip.
//!
//! Frames are `u32 length` + JSON. That is not the fastest protocol on earth,
//! but it is debuggable with `nc`, needs no handshake library, and is enough for
//! a devnet. v2 replaces this with QUIC (`quinn`).

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use chronyx_core::Block;

use crate::node::{Node, BLOCKS_PAGE};

pub const MAX_FRAME: usize = 32 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Msg {
    Hello {
        chain_id: String,
        genesis: String,
        identity: String,
        height: u64,
    },
    GetBlocks {
        from: u64,
    },
    Blocks {
        blocks: Vec<Block>,
    },
    Block(Block),
    Ping,
    Pong,
}

/// Fan-out to every live peer connection.
///
/// Each connection registers a sender and deregisters it when the socket dies.
/// Without that, a devnet that reconnects would slowly accumulate dead peers
/// (and keep their write halves alive forever).
#[derive(Default)]
pub struct Hub {
    peers: Mutex<Vec<(u64, mpsc::UnboundedSender<Msg>)>>,
    next_id: AtomicU64,
}

impl Hub {
    pub fn register(&self, tx: mpsc::UnboundedSender<Msg>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut peers) = self.peers.lock() {
            peers.retain(|(_, p)| !p.is_closed());
            peers.push((id, tx));
        }
        id
    }

    pub fn deregister(&self, id: u64) {
        if let Ok(mut peers) = self.peers.lock() {
            peers.retain(|(pid, _)| *pid != id);
        }
    }

    pub fn broadcast(&self, msg: Msg) {
        if let Ok(mut peers) = self.peers.lock() {
            peers.retain(|(_, p)| !p.is_closed());
            for (_, peer) in peers.iter() {
                let _ = peer.send(msg.clone());
            }
        }
    }

    pub fn count(&self) -> usize {
        self.peers
            .lock()
            .map(|mut p| {
                p.retain(|(_, s)| !s.is_closed());
                p.len()
            })
            .unwrap_or(0)
    }
}

pub async fn serve(node: Arc<Node>, addr: SocketAddr) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!(%addr, "p2p listening");

    loop {
        let (stream, peer) = listener.accept().await?;
        let node = node.clone();
        tokio::spawn(async move {
            debug!(%peer, "inbound peer");
            if let Err(e) = handle_conn(node, stream, false).await {
                debug!(%peer, error = %e, "peer disconnected");
            }
        });
    }
}

/// Dial a peer forever, reconnecting with backoff.
pub async fn dial(node: Arc<Node>, addr: SocketAddr) {
    loop {
        match TcpStream::connect(addr).await {
            Ok(stream) => {
                info!(%addr, "connected to peer");
                if let Err(e) = handle_conn(node.clone(), stream, true).await {
                    warn!(%addr, error = %e, "peer connection ended");
                }
            }
            Err(e) => debug!(%addr, error = %e, "dial failed"),
        }
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}

async fn handle_conn(node: Arc<Node>, stream: TcpStream, outbound: bool) -> anyhow::Result<()> {
    let (mut reader, mut writer) = stream.into_split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Msg>();

    // Writer task owns the write half.
    let writer_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let bytes = match serde_json::to_vec(&msg) {
                Ok(b) => b,
                Err(_) => continue,
            };
            if bytes.len() > MAX_FRAME {
                continue;
            }
            let len = (bytes.len() as u32).to_le_bytes();
            if writer.write_all(&len).await.is_err() {
                break;
            }
            if writer.write_all(&bytes).await.is_err() {
                break;
            }
        }
    });

    let height = node.height().await;
    let _ = tx.send(Msg::Hello {
        chain_id: node.genesis.chain_id.clone(),
        genesis: hex::encode(node.genesis_hash),
        identity: hex::encode(node.local.verifying_key().to_bytes()),
        height,
    });

    let peer_id = node.hub.register(tx.clone());

    if outbound {
        let _ = tx.send(Msg::GetBlocks { from: height + 1 });
    }

    // Every exit path below must deregister, otherwise the hub keeps this
    // connection's sender alive and the writer task never terminates.
    let reason = read_loop(&node, &mut reader, &tx).await;

    node.hub.deregister(peer_id);
    drop(tx);
    writer_task.abort();

    match reason {
        Some(e) => Err(anyhow::anyhow!(e)),
        None => Ok(()),
    }
}

/// Pump frames until the socket closes. Returns a reason when the peer should
/// be reported (protocol violation), or `None` for a plain disconnect.
async fn read_loop(
    node: &Node,
    reader: &mut tokio::net::tcp::OwnedReadHalf,
    tx: &mpsc::UnboundedSender<Msg>,
) -> Option<String> {
    loop {
        let mut len_buf = [0u8; 4];
        reader.read_exact(&mut len_buf).await.ok()?;
        let len = u32::from_le_bytes(len_buf) as usize;
        if len == 0 || len > MAX_FRAME {
            return Some(format!("bad frame size {len}"));
        }

        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf).await.ok()?;
        let msg: Msg = match serde_json::from_slice(&buf) {
            Ok(m) => m,
            Err(e) => return Some(format!("bad frame: {e}")),
        };

        match msg {
            Msg::Hello {
                genesis, height, ..
            } => {
                if genesis != hex::encode(node.genesis_hash) {
                    return Some("genesis mismatch: refusing peer".into());
                }
                debug!(height, "peer hello ok");
            }
            Msg::GetBlocks { from } => {
                let blocks = node.blocks_from(from).await;
                let _ = tx.send(Msg::Blocks { blocks });
            }
            Msg::Blocks { blocks } => {
                let got = blocks.len();
                let before = node.height().await;
                for block in blocks {
                    node.accept_block(block).await;
                }
                let after = node.height().await;
                // A full page means there is probably more behind it. Keep
                // pulling until the peer returns a short page — or until we
                // stop making progress, which means the chain disagrees.
                if got >= BLOCKS_PAGE && after > before {
                    let _ = tx.send(Msg::GetBlocks { from: after + 1 });
                }
            }
            Msg::Block(block) => node.accept_block(block).await,
            Msg::Ping => {
                let _ = tx.send(Msg::Pong);
            }
            Msg::Pong => {}
        }
    }
}

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Everything a node needs to boot. TOML on disk so it is diffable.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeConfig {
    pub rpc: SocketAddr,
    pub p2p: SocketAddr,
    pub data_dir: PathBuf,
    pub genesis_path: PathBuf,
    pub identity_path: PathBuf,
    pub peers: Vec<SocketAddr>,
    pub faucet_enabled: bool,
}

impl NodeConfig {
    pub fn devnet(data_dir: impl Into<PathBuf>) -> Self {
        let data_dir = data_dir.into();
        Self {
            rpc: "127.0.0.1:8899".parse().expect("valid addr"),
            p2p: "127.0.0.1:9000".parse().expect("valid addr"),
            genesis_path: data_dir.join("genesis.json"),
            identity_path: data_dir.join("validator.json"),
            data_dir,
            peers: Vec::new(),
            faucet_enabled: true,
        }
    }

    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let s = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&s)?)
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }
}

/// `~/.tachyon` on every platform, via `dirs`.
pub fn default_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".tachyon")
}

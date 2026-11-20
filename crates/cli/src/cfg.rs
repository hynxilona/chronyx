use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// CLI-side settings. Separate from the node config on purpose: the CLI can
/// point at a remote devnet while the node runs somewhere else.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CliConfig {
    pub rpc_url: String,
    pub keypair_path: PathBuf,
    pub genesis_path: PathBuf,
    pub data_dir: PathBuf,
}

impl Default for CliConfig {
    fn default() -> Self {
        let home = home();
        Self {
            rpc_url: "http://127.0.0.1:8899".to_string(),
            keypair_path: home.join("id.json"),
            genesis_path: home.join("genesis.json"),
            data_dir: home,
        }
    }
}

pub fn home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".tachyon")
}

pub fn config_path() -> PathBuf {
    home().join("cli.toml")
}

impl CliConfig {
    pub fn load() -> Self {
        let path = config_path();
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(cfg) = toml::from_str::<CliConfig>(&text) {
                return cfg;
            }
        }
        Self::default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn set(&mut self, key: &str, value: &str) -> anyhow::Result<()> {
        match key {
            "rpc_url" | "rpc" => self.rpc_url = value.to_string(),
            "keypair_path" | "keypair" => self.keypair_path = PathBuf::from(value),
            "genesis_path" | "genesis" => self.genesis_path = PathBuf::from(value),
            "data_dir" => self.data_dir = PathBuf::from(value),
            other => anyhow::bail!("unknown config key `{other}`"),
        }
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<String> {
        match key {
            "rpc_url" | "rpc" => Some(self.rpc_url.clone()),
            "keypair_path" | "keypair" => Some(self.keypair_path.display().to_string()),
            "genesis_path" | "genesis" => Some(self.genesis_path.display().to_string()),
            "data_dir" => Some(self.data_dir.display().to_string()),
            _ => None,
        }
    }
}

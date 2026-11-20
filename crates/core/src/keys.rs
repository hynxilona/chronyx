//! On-disk keypair format, shared by the node and the CLI.
//!
//! Deliberately plain JSON so anyone can inspect it. The secret is hex, not
//! base58, because this is a devnet and debuggability beats aesthetics.

use std::path::Path;

use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeypairFile {
    pub secret: String,
    pub public: String,
}

impl KeypairFile {
    pub fn generate() -> Self {
        let sk = SigningKey::generate(&mut rand::rngs::OsRng);
        Self::from_signing_key(&sk)
    }

    pub fn from_signing_key(sk: &SigningKey) -> Self {
        Self {
            secret: hex::encode(sk.to_bytes()),
            public: hex::encode(sk.verifying_key().to_bytes()),
        }
    }

    pub fn signing_key(&self) -> Result<SigningKey, String> {
        let bytes = hex::decode(&self.secret).map_err(|e| format!("bad secret hex: {e}"))?;
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| "secret must be exactly 32 bytes".to_string())?;
        let sk = SigningKey::from_bytes(&arr);
        if hex::encode(sk.verifying_key().to_bytes()) != self.public {
            return Err("public key does not match secret".into());
        }
        Ok(sk)
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(
            path,
            serde_json::to_string_pretty(self).expect("keypair serializes"),
        )
    }

    pub fn load(path: &Path) -> std::io::Result<Self> {
        let s = std::fs::read_to_string(path)?;
        serde_json::from_str(&s)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

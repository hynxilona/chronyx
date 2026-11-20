//! Append-only block ledger.
//!
//! One JSON object per line. Not the fastest format on earth, but it survives
//! crashes, can be `grep`ed, and needs zero schema migrations. v2 moves to redb.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use tachyon_core::Block;

pub struct Ledger {
    path: PathBuf,
}

impl Ledger {
    pub fn open(dir: &Path) -> std::io::Result<Self> {
        std::fs::create_dir_all(dir)?;
        Ok(Self {
            path: dir.join("blocks.jsonl"),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn append(&self, block: &Block) -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let line = serde_json::to_string(block)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writeln!(file, "{line}")?;
        Ok(())
    }

    pub fn load(&self) -> std::io::Result<Vec<Block>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let file = File::open(&self.path)?;
        let mut out = Vec::new();
        for line in BufReader::new(file).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(block) = serde_json::from_str::<Block>(&line) {
                out.push(block);
            }
        }
        Ok(out)
    }

    pub fn count(&self) -> usize {
        self.load().map(|b| b.len()).unwrap_or(0)
    }

    /// Blocks at or after `from_slot`, capped at `limit`.
    ///
    /// Streams the file instead of loading it all, so a node syncing from
    /// genesis does not need the entire chain in memory.
    pub fn load_from(&self, from_slot: u64, limit: usize) -> std::io::Result<Vec<Block>> {
        if limit == 0 || !self.path.exists() {
            return Ok(Vec::new());
        }
        let file = File::open(&self.path)?;
        let mut out = Vec::with_capacity(limit.min(1024));
        for line in BufReader::new(file).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let Ok(block) = serde_json::from_str::<Block>(&line) else {
                continue;
            };
            if block.header.slot >= from_slot {
                out.push(block);
                if out.len() >= limit {
                    break;
                }
            }
        }
        Ok(out)
    }
}

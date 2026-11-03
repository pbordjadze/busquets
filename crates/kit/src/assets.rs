//! Where the data comes from: a directory (Linux, the device's bundle) or memory (the web, which
//! fetched it first). Nothing is fetched by the library itself.

use std::collections::HashMap;
use std::path::PathBuf;

pub trait Assets {
    fn read(&self, name: &str) -> anyhow::Result<Vec<u8>>;
}

pub struct DirAssets(pub PathBuf);

impl Assets for DirAssets {
    fn read(&self, name: &str) -> anyhow::Result<Vec<u8>> {
        let p = self.0.join(name);
        std::fs::read(&p).map_err(|e| anyhow::anyhow!("{}: {e}", p.display()))
    }
}

pub struct MemoryAssets(pub HashMap<String, Vec<u8>>);

impl Assets for MemoryAssets {
    fn read(&self, name: &str) -> anyhow::Result<Vec<u8>> {
        self.0.get(name).cloned().ok_or_else(|| anyhow::anyhow!("no asset {name}"))
    }
}


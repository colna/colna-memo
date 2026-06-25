//! 增量索引状态:记录每个源文件的内容指纹(sha256),
//! 用于在 `colna index` 时只对新增 / 变更 / 删除的文件做处理。
//!
//! 状态文件落在本地 `.colna/state.json`,与 zvec 索引一样不入 git、可重建。

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

/// 索引状态:source_path(相对 memory/)→ 文件内容 sha256 hex。
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct IndexState {
    pub files: BTreeMap<String, String>,
}

impl IndexState {
    /// 从 state.json 读取;文件不存在或解析失败时返回空状态。
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// 写回 state.json。
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let s = serde_json::to_string_pretty(self).context("序列化索引状态失败")?;
        std::fs::write(path, s).with_context(|| format!("写入状态文件失败: {}", path.display()))?;
        Ok(())
    }
}

/// 计算字符串内容的 sha256 hex 指纹。
pub fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}

/// 增量比对结果。
pub struct Diff {
    /// 内容有变化或新增的文件(需重新切块 / 嵌入 / upsert)。
    pub changed: Vec<String>,
    /// 已从磁盘删除的文件(需从索引删除其全部块)。
    pub removed: Vec<String>,
}

/// 比对当前文件指纹与上次状态,得出需要处理的增量。
///
/// `current` 为本次扫描得到的 source_path → hash。
pub fn diff(old: &IndexState, current: &BTreeMap<String, String>) -> Diff {
    let mut changed = Vec::new();
    for (path, hash) in current {
        match old.files.get(path) {
            Some(old_hash) if old_hash == hash => {}
            _ => changed.push(path.clone()),
        }
    }
    let removed: Vec<String> = old
        .files
        .keys()
        .filter(|p| !current.contains_key(*p))
        .cloned()
        .collect();
    Diff { changed, removed }
}

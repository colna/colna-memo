//! 增量索引状态:记录每个源文件的内容指纹(sha256),
//! 用于在 `colna index` 时只对新增 / 变更 / 删除的文件做处理。
//!
//! 状态文件落在本地 `.colna/state.json`,与 zvec 索引一样不入 git、可重建。

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

pub const FORMAT_VERSION: u32 = 5;
/// 切块逻辑版本;改动分块/标题/embed_text 规则时递增,强制重建。
pub const CHUNKER_VERSION: u32 = 3;

/// 索引状态:source_path(相对 memory/)→ 文件内容 sha256 hex。
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct IndexState {
    #[serde(default)]
    pub format_version: u32,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub chunker_version: u32,
    #[serde(default)]
    pub chunk_count: u64,
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    #[serde(default)]
    pub chunk_ids: BTreeMap<String, Vec<String>>,
}

impl IndexState {
    pub fn complete(
        files: BTreeMap<String, String>,
        chunk_ids: BTreeMap<String, Vec<String>>,
    ) -> Self {
        let chunk_count = chunk_ids.values().map(|ids| ids.len() as u64).sum();
        Self {
            format_version: FORMAT_VERSION,
            model: crate::embedder::MODEL_ID.to_string(),
            chunker_version: CHUNKER_VERSION,
            chunk_count,
            files,
            chunk_ids,
        }
    }

    pub fn is_compatible(&self) -> bool {
        let tracked_chunk_count: u64 = self.chunk_ids.values().map(|ids| ids.len() as u64).sum();
        self.format_version == FORMAT_VERSION
            && self.model == crate::embedder::MODEL_ID
            && self.chunker_version == CHUNKER_VERSION
            && self.chunk_count == tracked_chunk_count
            && self.files.keys().eq(self.chunk_ids.keys())
    }

    /// 从 state.json 读取;缺失或损坏由调用方视为需要全量恢复。
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("读取索引状态失败: {}", path.display()))?;
        serde_json::from_str(&content)
            .with_context(|| format!("解析索引状态失败: {}", path.display()))
    }

    /// 写回 state.json。
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("创建索引状态目录失败: {}", parent.display()))?;
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
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn legacy_state_without_integrity_metadata_is_not_compatible() {
        let legacy: IndexState = serde_json::from_str(r#"{"files": {}}"#).unwrap();

        assert!(!legacy.is_compatible());
    }

    #[test]
    fn state_from_previous_embedding_format_requires_full_rebuild() {
        let previous_format: IndexState = serde_json::from_str(
            r#"{
                "format_version": 2,
                "chunk_count": 1,
                "files": {"note.md": "hash"},
                "chunk_ids": {"note.md": ["chunk-1"]}
            }"#,
        )
        .unwrap();

        assert!(!previous_format.is_compatible());
    }

    #[test]
    fn complete_state_tracks_chunk_ids_and_validates_its_count() {
        let files = BTreeMap::from([("note.md".to_string(), "hash".to_string())]);
        let chunk_ids = BTreeMap::from([(
            "note.md".to_string(),
            vec!["chunk-1".to_string(), "chunk-2".to_string()],
        )]);
        let mut state = IndexState::complete(files, chunk_ids);

        assert_eq!(state.chunk_count, 2);
        assert!(state.is_compatible());
        state.chunk_count = 1;
        assert!(!state.is_compatible());
    }

    #[test]
    fn state_from_wrong_model_requires_full_rebuild() {
        let mut state = IndexState::complete(BTreeMap::new(), BTreeMap::new());
        state.model = "other-model".to_string();
        assert!(!state.is_compatible());
    }

    #[test]
    fn state_from_previous_chunker_requires_full_rebuild() {
        let mut state = IndexState::complete(BTreeMap::new(), BTreeMap::new());
        state.chunker_version = CHUNKER_VERSION.saturating_sub(1);
        assert!(!state.is_compatible());
    }

    #[test]
    fn corrupt_state_returns_an_error_instead_of_an_empty_state() {
        let id = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "colna-memo-corrupt-state-{}-{id}.json",
            std::process::id()
        ));
        std::fs::write(&path, "not json").unwrap();

        let result = IndexState::load(&path);
        let _ = std::fs::remove_file(&path);

        assert!(result.is_err());
    }
}

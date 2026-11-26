//! 本地嵌入缓存:`embed_text` → 向量。
//!
//! 全量重建时 99% 的文本与上次相同,重复跑 ONNX 推理是重建慢的主因。
//! 缓存按 `embed_text` 的 sha256 命中,与文件路径 / chunk id 无关;
//! 模型或维度变化时整体作废(header 校验),不会串用旧向量。
//!
//! 落盘格式(小端):
//! magic(8) | model_len(u32) | model | dim(u32) | count(u64) | [key(32) + dim×f32]...

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 8] = b"COLNAEC1";
/// 上限约 40k 条 × 3 KB ≈ 120 MB;超出后不再收录新条目,已有条目继续命中。
const MAX_ENTRIES: usize = 40_000;

pub struct EmbedCache {
    path: PathBuf,
    model: String,
    dim: usize,
    entries: HashMap<[u8; 32], Vec<f32>>,
    dirty: bool,
}

impl EmbedCache {
    /// 读取缓存;文件缺失、损坏或模型 / 维度不一致时从空缓存开始。
    pub fn load(path: &Path, model: &str, dim: usize) -> Self {
        let entries = std::fs::read(path)
            .ok()
            .and_then(|bytes| decode(&bytes, model, dim))
            .unwrap_or_default();
        Self {
            path: path.to_path_buf(),
            model: model.to_string(),
            dim,
            entries,
            dirty: false,
        }
    }

    /// 缓存键:与 `Embedder::embed_passages` 的输入契约保持一致(含 passage 前缀)。
    pub fn key(embed_text: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"passage: ");
        hasher.update(embed_text.as_bytes());
        let digest = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&digest);
        key
    }

    pub fn get(&self, key: &[u8; 32]) -> Option<&[f32]> {
        self.entries.get(key).map(Vec::as_slice)
    }

    /// 写入一条向量;维度不符或已达上限时忽略(缓存只是加速,不能影响正确性)。
    pub fn put(&mut self, key: [u8; 32], vector: Vec<f32>) {
        if vector.len() != self.dim || self.entries.contains_key(&key) {
            return;
        }
        if self.entries.len() >= MAX_ENTRIES {
            return;
        }
        self.entries.insert(key, vector);
        self.dirty = true;
    }

    /// 原子落盘(tmp + rename);失败由调用方决定是否只告警。
    pub fn save(&mut self) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("创建嵌入缓存目录失败: {}", parent.display()))?;
        }
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, self.encode())
            .with_context(|| format!("写入嵌入缓存失败: {}", tmp.display()))?;
        std::fs::rename(&tmp, &self.path)
            .with_context(|| format!("提交嵌入缓存失败: {}", self.path.display()))?;
        self.dirty = false;
        Ok(())
    }

    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(
            8 + 4 + self.model.len() + 4 + 8 + self.entries.len() * (32 + self.dim * 4),
        );
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&(self.model.len() as u32).to_le_bytes());
        out.extend_from_slice(self.model.as_bytes());
        out.extend_from_slice(&(self.dim as u32).to_le_bytes());
        out.extend_from_slice(&(self.entries.len() as u64).to_le_bytes());
        for (key, vector) in &self.entries {
            out.extend_from_slice(key);
            for value in vector {
                out.extend_from_slice(&value.to_le_bytes());
            }
        }
        out
    }
}

fn decode(bytes: &[u8], model: &str, dim: usize) -> Option<HashMap<[u8; 32], Vec<f32>>> {
    let mut cursor = 0usize;
    if take(bytes, &mut cursor, MAGIC.len())? != MAGIC {
        return None;
    }
    let model_len = u32::from_le_bytes(take(bytes, &mut cursor, 4)?.try_into().ok()?) as usize;
    let stored_model = std::str::from_utf8(take(bytes, &mut cursor, model_len)?).ok()?;
    if stored_model != model {
        return None;
    }
    let stored_dim = u32::from_le_bytes(take(bytes, &mut cursor, 4)?.try_into().ok()?) as usize;
    if stored_dim != dim {
        return None;
    }
    let count = u64::from_le_bytes(take(bytes, &mut cursor, 8)?.try_into().ok()?) as usize;
    // 防御损坏文件导致的超大分配。
    if count > MAX_ENTRIES {
        return None;
    }
    let mut entries = HashMap::with_capacity(count);
    for _ in 0..count {
        let key: [u8; 32] = take(bytes, &mut cursor, 32)?.try_into().ok()?;
        let mut vector = Vec::with_capacity(dim);
        for _ in 0..dim {
            let raw = take(bytes, &mut cursor, 4)?.try_into().ok()?;
            vector.push(f32::from_le_bytes(raw));
        }
        entries.insert(key, vector);
    }
    Some(entries)
}

fn take<'a>(bytes: &'a [u8], cursor: &mut usize, len: usize) -> Option<&'a [u8]> {
    let end = cursor.checked_add(len)?;
    let slice = bytes.get(*cursor..end)?;
    *cursor = end;
    Some(slice)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(1);

    fn temp_path() -> PathBuf {
        let id = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("colna-embed-cache-{}-{id}.bin", std::process::id()))
    }

    #[test]
    fn cache_round_trips_through_disk() {
        let path = temp_path();
        let mut cache = EmbedCache::load(&path, "m", 3);
        assert!(cache.entries.is_empty());
        let key = EmbedCache::key("hello");
        cache.put(key, vec![0.1, 0.2, 0.3]);
        cache.save().unwrap();

        let loaded = EmbedCache::load(&path, "m", 3);
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.get(&key), Some([0.1f32, 0.2, 0.3].as_slice()));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn model_or_dimension_change_invalidates_the_cache() {
        let path = temp_path();
        let mut cache = EmbedCache::load(&path, "m", 3);
        cache.put(EmbedCache::key("hello"), vec![1.0, 2.0, 3.0]);
        cache.save().unwrap();

        assert!(EmbedCache::load(&path, "other-model", 3).entries.is_empty());
        assert!(EmbedCache::load(&path, "m", 4).entries.is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn truncated_or_corrupt_files_are_ignored() {
        let path = temp_path();
        std::fs::write(&path, b"COLNAEC1 broken").unwrap();
        assert!(EmbedCache::load(&path, "m", 3).entries.is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn different_text_gets_different_keys_and_wrong_dimension_is_rejected() {
        assert_ne!(EmbedCache::key("a"), EmbedCache::key("b"));
        assert_eq!(EmbedCache::key("a"), EmbedCache::key("a"));

        let path = temp_path();
        let mut cache = EmbedCache::load(&path, "m", 3);
        cache.put(EmbedCache::key("a"), vec![1.0, 2.0]);
        assert!(cache.entries.is_empty());
    }
}

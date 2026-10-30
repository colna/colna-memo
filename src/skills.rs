//! 技能索引:把 `~/.agents/skills/*/SKILL.md` 作为独立 collection 建索引。
//!
//! 技能是过程性记忆;不混进 memory/ 主库,避免污染个人笔记检索。

use crate::chunker;
use crate::embedder::Embedder;
use crate::search::{self, Filters, SearchOptions};
use crate::store;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

pub const SKILLS_INDEX: &str = ".colna/skills.zvec";

pub fn default_skills_root() -> PathBuf {
    if let Ok(path) = std::env::var("COLNA_SKILLS_DIR") {
        return PathBuf::from(path);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".agents/skills")
}

pub fn collect_skill_files(root: &Path) -> Vec<(String, PathBuf)> {
    let mut files = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path().join("SKILL.md");
        if path.is_file() {
            let name = entry.file_name().to_string_lossy().to_string();
            files.push((format!("skills/{name}/SKILL.md"), path));
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

pub fn index(root: &Path, skills_root: &Path) -> Result<usize> {
    let files = collect_skill_files(skills_root);
    if files.is_empty() {
        bail!("没有找到技能: {}", skills_root.display());
    }

    let mut chunks = Vec::new();
    for (rel, path) in &files {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("读取技能失败: {}", path.display()))?;
        chunks.extend(chunker::chunk_markdown(rel, &content));
    }

    let index_path = root.join(SKILLS_INDEX);
    if index_path.exists() {
        std::fs::remove_dir_all(&index_path)
            .with_context(|| format!("清理旧技能索引失败: {}", index_path.display()))?;
    }
    let collection = store::create(&index_path.to_string_lossy())?;
    let mut embedder = Embedder::new_quiet()?;
    let embeddings = embedder.embed_passages(
        &chunks
            .iter()
            .map(|chunk| chunk.embed_text.clone())
            .collect::<Vec<_>>(),
    )?;
    store::insert_chunks(&collection, &chunks, &embeddings)?;
    store::finalize_collection(&collection, chunks.len() as u64)?;
    collection.close().ok();
    Ok(files.len())
}

pub fn search(
    root: &Path,
    query: &str,
    topk: usize,
    embedder: &mut Embedder,
) -> Result<Vec<store::Hit>> {
    let index_path = root.join(SKILLS_INDEX);
    if !index_path.exists() {
        bail!("技能索引不存在,先运行 `colna skills index`");
    }
    let query_vec = embedder.embed_query(query)?;
    let collection = store::open(&index_path.to_string_lossy())?;
    let hits = search::query(
        &collection,
        &query_vec,
        query,
        &SearchOptions {
            topk,
            hybrid: true,
            filters: Filters::default(),
            recency_weight: 0.0,
            mmr: true,
            include_summary: true,
            link_boost: 0.0,
        },
        None,
    )?;
    collection.close().ok();
    Ok(hits)
}

/// 供 MCP 使用:确保索引存在(不存在时直接报错,不悄悄重建)。
pub fn ensure_index_exists(root: &Path) -> Result<()> {
    if !root.join(SKILLS_INDEX).exists() {
        bail!("技能索引不存在,先运行 `colna skills index`");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_skill_manifests() {
        let dir = std::env::temp_dir().join(format!(
            "colna-skills-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(dir.join("alpha")).unwrap();
        std::fs::write(dir.join("alpha/SKILL.md"), "# Alpha\n").unwrap();
        let files = collect_skill_files(&dir);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].0, "skills/alpha/SKILL.md");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

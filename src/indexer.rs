//! 索引构建与增量更新。
//!
//! 这里从 main.rs 抽出来,供 CLI、MCP 写入、context 共用。
//! `quiet=true` 时所有进度写 stderr,保证 MCP 的 stdout 只有 JSON-RPC。

use crate::chunker;
use crate::embedder::Embedder;
use crate::ignore;
use crate::state;
use crate::store;
use anyhow::{anyhow, Context, Result};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use walkdir::WalkDir;

pub const MEMORY_DIR: &str = "memory";
pub const INDEX_PATH: &str = ".colna/index.zvec";
pub const STATE_PATH: &str = ".colna/state.json";
const INDEX_BATCH_SIZE: usize = 16;
const LOCK_STALE_AFTER: Duration = Duration::from_secs(10 * 60);

struct Log {
    quiet: bool,
}

impl Log {
    fn info(&self, message: &str) {
        if self.quiet {
            eprintln!("{message}");
        } else {
            println!("{message}");
        }
    }

    fn warn(&self, message: &str) {
        eprintln!("{message}");
    }
}

/// 收集 memory/ 下所有 .md 文件,返回 (相对路径, 绝对路径)。
/// 跳过任意一级 `_` 前缀目录(_templates / _attachments)。
pub(crate) fn collect_markdown(memory_root: &Path) -> Vec<(String, PathBuf)> {
    let root = memory_root.parent().unwrap_or(memory_root);
    let ignore_patterns = ignore::load(root);
    collect_markdown_with(memory_root, &ignore_patterns)
}

/// 包含 `.colnaignore` 排除项;供 secrets 扫描使用。
pub(crate) fn collect_markdown_all(memory_root: &Path) -> Vec<(String, PathBuf)> {
    collect_markdown_with(memory_root, &[])
}

fn collect_markdown_with(memory_root: &Path, ignore_patterns: &[String]) -> Vec<(String, PathBuf)> {
    let mut files = Vec::new();
    for entry in WalkDir::new(memory_root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && path.extension().map(|e| e == "md").unwrap_or(false) {
            let rel = path
                .strip_prefix(memory_root)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();
            if rel.split('/').any(|seg| seg.starts_with('_')) {
                continue;
            }
            if ignore::is_ignored(&rel, ignore_patterns) {
                continue;
            }
            files.push((rel, path.to_path_buf()));
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

/// 只算指纹,不保留正文(用于判断是否需要重建)。
fn scan_hashes(memory_root: &Path) -> Result<BTreeMap<String, String>> {
    let mut hashes = BTreeMap::new();
    for (rel, abs) in collect_markdown(memory_root) {
        let content = std::fs::read_to_string(&abs)
            .with_context(|| format!("读取失败: {}", abs.display()))?;
        hashes.insert(rel, state::content_hash(&content));
    }
    Ok(hashes)
}

/// 扫描全部 md,返回 (相对路径 → 文件内容, 相对路径 → 内容指纹)。
fn scan(memory_root: &Path) -> Result<(BTreeMap<String, String>, BTreeMap<String, String>)> {
    let mut contents = BTreeMap::new();
    let mut hashes = BTreeMap::new();
    for (rel, abs) in collect_markdown(memory_root) {
        let content = std::fs::read_to_string(&abs)
            .with_context(|| format!("读取失败: {}", abs.display()))?;
        hashes.insert(rel.clone(), state::content_hash(&content));
        contents.insert(rel, content);
    }
    Ok((contents, hashes))
}

fn chunks_for(rels: &[String], contents: &BTreeMap<String, String>) -> Vec<chunker::Chunk> {
    let mut chunks = Vec::new();
    for rel in rels {
        if let Some(content) = contents.get(rel) {
            chunks.extend(chunker::chunk_markdown(rel, content));
        }
    }
    chunks
}

fn chunk_ids_by_source(
    rels: &[String],
    chunks: &[chunker::Chunk],
) -> BTreeMap<String, Vec<String>> {
    let mut ids = rels
        .iter()
        .map(|rel| (rel.clone(), Vec::new()))
        .collect::<BTreeMap<_, _>>();
    for chunk in chunks {
        ids.entry(chunk.source_path.clone())
            .or_default()
            .push(chunk.id.clone());
    }
    ids
}

fn needs_full_rebuild(
    force: bool,
    index_exists: bool,
    saved_state: Option<&state::IndexState>,
    actual_doc_count: Option<u64>,
    indexes_complete: bool,
) -> bool {
    if force || !index_exists {
        return true;
    }
    match saved_state {
        Some(saved) => {
            !saved.is_compatible()
                || actual_doc_count != Some(saved.chunk_count)
                || !indexes_complete
        }
        None => true,
    }
}

fn process_in_batches<T>(
    items: &[T],
    batch_size: usize,
    mut process: impl FnMut(&[T]) -> Result<usize>,
) -> Result<usize> {
    if batch_size == 0 {
        anyhow::bail!("batch_size 必须大于 0");
    }
    let mut processed = 0;
    for batch in items.chunks(batch_size) {
        processed += process(batch)?;
    }
    Ok(processed)
}

#[derive(Clone, Copy)]
enum WriteMode {
    Insert,
    Upsert,
}

fn embed_and_write_chunks(
    collection: &zvec::Collection,
    chunks: &[chunker::Chunk],
    mode: WriteMode,
    embedder: &mut Embedder,
    log: &Log,
) -> Result<usize> {
    let batch_count = chunks.len().div_ceil(INDEX_BATCH_SIZE);
    let mut batch_number = 0;

    process_in_batches(chunks, INDEX_BATCH_SIZE, |batch| {
        let texts: Vec<String> = batch.iter().map(|chunk| chunk.embed_text.clone()).collect();
        let vectors = embedder.embed_passages(&texts)?;
        let written = match mode {
            WriteMode::Insert => store::insert_chunks(collection, batch, &vectors)?,
            WriteMode::Upsert => store::upsert_chunks(collection, batch, &vectors)?,
        };
        batch_number += 1;
        log.info(&format!(
            "  batch {batch_number}/{batch_count}: {written} 块"
        ));
        Ok(written)
    })
}

fn load_state(path: &Path, log: &Log) -> Option<state::IndexState> {
    if !path.exists() {
        return None;
    }
    match state::IndexState::load(path) {
        Ok(saved) => Some(saved),
        Err(error) => {
            log.warn(&format!("索引状态不可用,将自动全量重建: {error}"));
            None
        }
    }
}

fn current_status(
    index_path: &Path,
    state: Option<&state::IndexState>,
    force: bool,
    full: bool,
    log: &Log,
) -> (Option<u64>, bool) {
    if full || force || !index_path.exists() {
        return (None, false);
    }
    if !state.map(state::IndexState::is_compatible).unwrap_or(false) {
        return (None, false);
    }
    match store::collection_status(&index_path.to_string_lossy()) {
        Ok(status) => (Some(status.doc_count), status.indexes_complete),
        Err(error) => {
            log.warn(&format!("索引无法校验,将自动全量重建: {error}"));
            (None, false)
        }
    }
}

/// 执行索引(CLI `colna index` / add / sync 使用)。
pub fn run(root: &Path, full: bool, quiet: bool) -> Result<()> {
    ensure(root, full, quiet, None).map(|_| ())
}

/// 确保索引与当前 memory/ 一致;返回是否发生了写入。
///
/// 两阶段:先无锁扫描判断是否需要更新(大多数搜索走这里,零写入);
/// 需要更新时再拿锁、重新扫描、执行全量或增量。
pub fn ensure(
    root: &Path,
    full: bool,
    quiet: bool,
    embedder: Option<&mut Embedder>,
) -> Result<bool> {
    let memory_root = root.join(MEMORY_DIR);
    if !memory_root.is_dir() {
        anyhow::bail!("找不到 memory/ 目录: {}", memory_root.display());
    }
    let index_path = root.join(INDEX_PATH);
    let state_path = root.join(STATE_PATH);
    let log = Log { quiet };

    let hashes = scan_hashes(&memory_root)?;
    let saved = load_state(&state_path, &log);
    let (doc_count, indexes_complete) =
        current_status(&index_path, saved.as_ref(), full, full, &log);
    let do_full = needs_full_rebuild(
        full,
        index_path.exists(),
        saved.as_ref(),
        doc_count,
        indexes_complete,
    );

    if !do_full {
        let saved = saved.as_ref().expect("完整性检查已确认 state 存在");
        let diff = state::diff(saved, &hashes);
        if diff.changed.is_empty() && diff.removed.is_empty() {
            log.info(&format!("✅ 无变化,索引已是最新({} 个文件)", hashes.len()));
            return Ok(false);
        }
    }

    let _lock = IndexLock::acquire(root)?;

    // 拿锁后重新读取,避免扫描期间文件变化导致状态与索引不一致。
    let (contents, hashes) = scan(&memory_root)?;
    let saved = load_state(&state_path, &log);
    let (doc_count, indexes_complete) =
        current_status(&index_path, saved.as_ref(), full, full, &log);
    let do_full = needs_full_rebuild(
        full,
        index_path.exists(),
        saved.as_ref(),
        doc_count,
        indexes_complete,
    );

    let mut owned;
    let embedder: &mut Embedder = match embedder {
        Some(existing) => existing,
        None => {
            owned = Embedder::new_quiet()?;
            &mut owned
        }
    };

    if do_full {
        if !full && (index_path.exists() || state_path.exists()) {
            log.info("索引完整性校验未通过,自动执行全量重建。");
        }
        full_rebuild(&index_path, &state_path, &contents, &hashes, embedder, &log)?;
    } else {
        incremental(
            &index_path,
            &state_path,
            &contents,
            &hashes,
            saved.as_ref().expect("完整性检查已确认 state 存在"),
            embedder,
            &log,
        )?;
    }
    Ok(true)
}

fn full_rebuild(
    index_path: &Path,
    state_path: &Path,
    contents: &BTreeMap<String, String>,
    hashes: &BTreeMap<String, String>,
    embedder: &mut Embedder,
    log: &Log,
) -> Result<()> {
    if state_path.exists() {
        std::fs::remove_file(state_path)
            .with_context(|| format!("清理旧索引状态失败: {}", state_path.display()))?;
    }
    if index_path.exists() {
        std::fs::remove_dir_all(index_path)
            .with_context(|| format!("清理旧索引失败: {}", index_path.display()))?;
    }
    if let Some(parent) = index_path.parent() {
        std::fs::create_dir_all(parent).context("创建 .colna 目录失败")?;
    }

    let all_rels: Vec<String> = contents.keys().cloned().collect();
    log.info(&format!(
        "全量重建:扫描到 {} 个 Markdown 文件",
        all_rels.len()
    ));
    let chunks = chunks_for(&all_rels, contents);
    let chunk_ids = chunk_ids_by_source(&all_rels, &chunks);
    log.info(&format!("共切出 {} 个内容块", chunks.len()));

    let index_path_str = index_path.to_string_lossy().to_string();
    let collection = store::create(&index_path_str)?;

    if !chunks.is_empty() {
        log.info("分批生成向量并写入...");
        let n = embed_and_write_chunks(&collection, &chunks, WriteMode::Insert, embedder, log)?;
        log.info(&format!("✅ 写入 {n} 块"));
    } else {
        log.info("没有可索引的内容。");
    }
    store::finalize_collection(&collection, chunks.len() as u64)?;
    collection
        .close()
        .map_err(|e| anyhow!("关闭索引失败: {e}"))?;

    state::IndexState::complete(hashes.clone(), chunk_ids).save(state_path)?;
    log.info(&format!("索引完成 → {index_path_str}"));
    Ok(())
}

fn incremental(
    index_path: &Path,
    state_path: &Path,
    contents: &BTreeMap<String, String>,
    hashes: &BTreeMap<String, String>,
    old: &state::IndexState,
    embedder: &mut Embedder,
    log: &Log,
) -> Result<()> {
    let diff = state::diff(old, hashes);

    if diff.changed.is_empty() && diff.removed.is_empty() {
        log.info(&format!("✅ 无变化,索引已是最新({} 个文件)", hashes.len()));
        return Ok(());
    }
    log.info(&format!(
        "增量更新:变更/新增 {} 个,删除 {} 个",
        diff.changed.len(),
        diff.removed.len()
    ));

    let index_path_str = index_path.to_string_lossy().to_string();
    let collection = store::open(&index_path_str)?;

    for rel in diff.removed.iter().chain(diff.changed.iter()) {
        if let Some(old_ids) = old.chunk_ids.get(rel) {
            for batch in old_ids.chunks(INDEX_BATCH_SIZE) {
                store::delete_chunks(&collection, batch)?;
            }
        }
    }

    let chunks = chunks_for(&diff.changed, contents);
    if !chunks.is_empty() {
        log.info(&format!("重新嵌入 {} 个内容块...", chunks.len()));
        let n = embed_and_write_chunks(&collection, &chunks, WriteMode::Upsert, embedder, log)?;
        log.info(&format!("✅ upsert {n} 块"));
    }
    let mut chunk_ids = old.chunk_ids.clone();
    for rel in &diff.removed {
        chunk_ids.remove(rel);
    }
    for rel in &diff.changed {
        chunk_ids.insert(rel.clone(), Vec::new());
    }
    for chunk in &chunks {
        chunk_ids
            .entry(chunk.source_path.clone())
            .or_default()
            .push(chunk.id.clone());
    }
    let expected_doc_count = chunk_ids.values().map(|ids| ids.len() as u64).sum();
    store::finalize_collection(&collection, expected_doc_count)?;
    collection
        .close()
        .map_err(|e| anyhow!("关闭索引失败: {e}"))?;

    state::IndexState::complete(hashes.clone(), chunk_ids).save(state_path)?;
    log.info(&format!("增量索引完成 → {index_path_str}"));
    Ok(())
}

/// 简单的进程级锁:用目录而不是单文件,避免被 `rm -f index.lock` 误删。
struct IndexLock {
    dir: PathBuf,
}

impl IndexLock {
    fn acquire(root: &Path) -> Result<Self> {
        let dir = root.join(".colna/index.lock.d");
        if let Some(parent) = dir.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("创建锁目录失败: {}", parent.display()))?;
        }
        if dir.exists() {
            if is_stale(&dir) {
                let _ = std::fs::remove_dir_all(&dir);
            } else {
                anyhow::bail!(
                    "另一个 colna 进程正在更新索引(lock: {});确认没有进程在跑后删除该目录再重试",
                    dir.display()
                );
            }
        }
        match std::fs::create_dir(&dir) {
            Ok(()) => {
                let _ = std::fs::write(dir.join("pid"), format!("pid={}\n", std::process::id()));
                Ok(Self { dir })
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                anyhow::bail!(
                    "另一个 colna 进程正在更新索引(lock: {});确认没有进程在跑后删除该目录再重试",
                    dir.display()
                )
            }
            Err(error) => Err(error).with_context(|| format!("创建索引锁失败: {}", dir.display())),
        }
    }
}

impl Drop for IndexLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn is_stale(dir: &Path) -> bool {
    if let Ok(content) = std::fs::read_to_string(dir.join("pid")) {
        if let Some(pid) = content
            .lines()
            .find_map(|line| line.strip_prefix("pid="))
            .and_then(|pid| pid.trim().parse::<u32>().ok())
        {
            return !process_alive(pid);
        }
    }
    std::fs::metadata(dir)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .map(|age| age > LOCK_STALE_AFTER)
        .unwrap_or(false)
}

fn process_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .map(|status| status.success())
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete_state() -> state::IndexState {
        let files = BTreeMap::from([("note.md".to_string(), "hash".to_string())]);
        let chunk_ids = BTreeMap::from([("note.md".to_string(), vec!["chunk-1".to_string()])]);
        state::IndexState::complete(files, chunk_ids)
    }

    #[test]
    fn incomplete_or_unverifiable_index_requires_full_rebuild() {
        let state = complete_state();

        assert!(needs_full_rebuild(false, true, None, None, false));
        assert!(needs_full_rebuild(false, true, Some(&state), Some(0), true));
        assert!(needs_full_rebuild(
            false,
            false,
            Some(&state),
            Some(1),
            true
        ));
        assert!(needs_full_rebuild(true, true, Some(&state), Some(1), true));
        assert!(!needs_full_rebuild(
            false,
            true,
            Some(&state),
            Some(1),
            true
        ));
        assert!(needs_full_rebuild(
            false,
            true,
            Some(&state),
            Some(1),
            false
        ));
    }

    #[test]
    fn processes_items_in_bounded_batches() {
        let items: Vec<usize> = (0..35).collect();
        let mut batch_sizes = Vec::new();

        let processed = process_in_batches(&items, 16, |batch| {
            batch_sizes.push(batch.len());
            Ok(batch.len())
        })
        .unwrap();

        assert_eq!(processed, 35);
        assert_eq!(batch_sizes, vec![16, 16, 3]);
    }

    #[test]
    fn rejects_zero_batch_size() {
        let error = process_in_batches(&[1], 0, |_| Ok(1)).unwrap_err();
        assert!(error.to_string().contains("必须大于 0"));
    }

    #[test]
    fn stops_after_a_batch_fails() {
        let items: Vec<usize> = (0..7).collect();
        let mut calls = 0;

        let error = process_in_batches(&items, 3, |batch| {
            calls += 1;
            if calls == 2 {
                anyhow::bail!("batch failed");
            }
            Ok(batch.len())
        })
        .unwrap_err();

        assert_eq!(error.to_string(), "batch failed");
        assert_eq!(calls, 2);
    }
}

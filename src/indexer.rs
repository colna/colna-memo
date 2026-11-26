//! 索引构建与增量更新。
//!
//! 这里从 main.rs 抽出来,供 CLI、MCP 写入、context 共用。
//! `quiet=true` 时所有进度写 stderr,保证 MCP 的 stdout 只有 JSON-RPC。

use crate::chunker;
use crate::embedcache::EmbedCache;
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
pub const EMBED_CACHE_PATH: &str = ".colna/embed.cache";
const INDEX_BATCH_SIZE: usize = 16;
const LOCK_STALE_AFTER: Duration = Duration::from_secs(10 * 60);
/// 全量重建先写到 `.new`,成功后再原子换名;中断时旧索引仍然可用。
const BUILD_SUFFIX: &str = ".new";
const PREVIOUS_SUFFIX: &str = ".old";

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
    indexes_complete: bool,
) -> bool {
    if force || !index_exists {
        return true;
    }
    match saved_state {
        Some(saved) => !saved.is_compatible() || !indexes_complete,
        None => true,
    }
}

struct IndexPlan {
    do_full: bool,
    /// state 记录的总块数与索引实际文档数不一致(通常是增量写入中途被打断)。
    count_mismatch: bool,
    saved: Option<state::IndexState>,
    doc_count: Option<u64>,
}

/// 判断本次是否需要全量重建,并记录可用于增量修复的上下文。
///
/// 文档数不一致不再直接升级为全量:先让增量流程重放一次 diff,
/// 由 `finalize_collection` 的严格校验兜底,失败再降级全量。
fn plan_index(
    index_path: &Path,
    state_path: &Path,
    hashes: &BTreeMap<String, String>,
    full: bool,
    log: &Log,
) -> IndexPlan {
    let saved = load_state(state_path, log);
    let (doc_count, indexes_complete) = current_status(index_path, saved.as_ref(), full, full, log);
    let mut do_full =
        needs_full_rebuild(full, index_path.exists(), saved.as_ref(), indexes_complete);
    let count_mismatch = match (saved.as_ref(), doc_count) {
        (Some(saved), Some(actual)) => actual != saved.chunk_count,
        _ => false,
    };
    if !do_full {
        if let Some(saved) = saved.as_ref() {
            let diff = state::diff(saved, hashes);
            if count_mismatch && diff.changed.is_empty() && diff.removed.is_empty() {
                log.warn(&format!(
                    "索引文档数与状态不一致(actual={}, expected={}),且没有可重放的文件变更;需要全量重建",
                    doc_count.unwrap_or(0),
                    saved.chunk_count
                ));
                do_full = true;
            }
        }
    }
    IndexPlan {
        do_full,
        count_mismatch,
        saved,
        doc_count,
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
    cache: &mut EmbedCache,
    log: &Log,
) -> Result<usize> {
    let batch_count = chunks.len().div_ceil(INDEX_BATCH_SIZE);
    let mut batch_number = 0;
    let mut cache_hits = 0usize;

    let written = process_in_batches(chunks, INDEX_BATCH_SIZE, |batch| {
        let vectors = embed_batch_with_cache(batch, embedder, cache, &mut cache_hits)?;
        let written = match mode {
            WriteMode::Insert => store::insert_chunks(collection, batch, &vectors)?,
            WriteMode::Upsert => store::upsert_chunks(collection, batch, &vectors)?,
        };
        batch_number += 1;
        log.info(&format!(
            "  batch {batch_number}/{batch_count}: {written} 块"
        ));
        // 长重建每隔一段落一次缓存,避免整体被 kill 时已算好的向量全丢。
        if batch_number % 64 == 0 {
            if let Err(error) = cache.save() {
                log.warn(&format!("写入嵌入缓存失败(稍后重试): {error}"));
            }
        }
        Ok(written)
    })?;
    if cache_hits > 0 {
        log.info(&format!("  嵌入缓存命中 {cache_hits}/{}", chunks.len()));
    }
    Ok(written)
}

/// 命中缓存的块直接取向量,只对 miss 的部分跑 ONNX 推理。
fn embed_batch_with_cache(
    batch: &[chunker::Chunk],
    embedder: &mut Embedder,
    cache: &mut EmbedCache,
    cache_hits: &mut usize,
) -> Result<Vec<Vec<f32>>> {
    let keys: Vec<[u8; 32]> = batch
        .iter()
        .map(|chunk| EmbedCache::key(&chunk.embed_text))
        .collect();
    let mut vectors: Vec<Option<Vec<f32>>> = Vec::with_capacity(batch.len());
    let mut misses: Vec<usize> = Vec::new();
    for (index, key) in keys.iter().enumerate() {
        match cache.get(key) {
            Some(vector) => {
                vectors.push(Some(vector.to_vec()));
                *cache_hits += 1;
            }
            None => {
                vectors.push(None);
                misses.push(index);
            }
        }
    }
    if misses.is_empty() {
        return Ok(vectors.into_iter().flatten().collect());
    }

    let texts: Vec<String> = misses
        .iter()
        .map(|&index| batch[index].embed_text.clone())
        .collect();
    let fresh = embedder.embed_passages(&texts)?;
    if fresh.len() != misses.len() {
        anyhow::bail!(
            "embedding 返回数量不一致: expected={}, actual={}",
            misses.len(),
            fresh.len()
        );
    }
    for (position, &index) in misses.iter().enumerate() {
        vectors[index] = Some(fresh[position].clone());
        cache.put(keys[index], fresh[position].clone());
    }
    Ok(vectors
        .into_iter()
        .map(|vector| vector.expect("每个块都应有向量"))
        .collect())
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
/// 允许按需全量重建,供 `colna index` / `colna sync` / 显式写入使用。
pub fn ensure(
    root: &Path,
    full: bool,
    quiet: bool,
    embedder: Option<&mut Embedder>,
) -> Result<bool> {
    ensure_inner(root, full, quiet, embedder, true)
}

/// 只读路径使用:允许增量更新,但索引需要全量重建时立刻报错。
///
/// 避免一条 `search` / MCP 查询在后台跑 30 分钟整库重建并长时间持锁。
pub fn ensure_readable(root: &Path, quiet: bool, embedder: Option<&mut Embedder>) -> Result<bool> {
    ensure_inner(root, false, quiet, embedder, false)
}

/// 两阶段:先无锁扫描判断是否需要更新(大多数搜索走这里,零写入);
/// 需要更新时再拿锁、重新扫描、执行全量或增量。
fn ensure_inner(
    root: &Path,
    full: bool,
    quiet: bool,
    embedder: Option<&mut Embedder>,
    allow_full_rebuild: bool,
) -> Result<bool> {
    let memory_root = root.join(MEMORY_DIR);
    if !memory_root.is_dir() {
        anyhow::bail!("找不到 memory/ 目录: {}", memory_root.display());
    }
    let index_path = root.join(INDEX_PATH);
    let state_path = root.join(STATE_PATH);
    let cache_path = root.join(EMBED_CACHE_PATH);
    let log = Log { quiet };

    let hashes = scan_hashes(&memory_root)?;
    let plan = plan_index(&index_path, &state_path, &hashes, full, &log);
    if !plan.do_full {
        let saved = plan.saved.as_ref().expect("完整性检查已确认 state 存在");
        let diff = state::diff(saved, &hashes);
        if diff.changed.is_empty() && diff.removed.is_empty() {
            log.info(&format!("✅ 无变化,索引已是最新({} 个文件)", hashes.len()));
            return Ok(false);
        }
    }
    let _lock = IndexLock::acquire(root)?;
    recover_interrupted_swap(&index_path, &log);

    // 拿锁后重新读取,避免扫描期间文件变化导致状态与索引不一致。
    let (contents, hashes) = scan(&memory_root)?;
    let plan = plan_index(&index_path, &state_path, &hashes, full, &log);
    ensure_full_rebuild_allowed(plan.do_full, allow_full_rebuild)?;

    let mut owned;
    let embedder: &mut Embedder = match embedder {
        Some(existing) => existing,
        None => {
            owned = Embedder::new_quiet()?;
            &mut owned
        }
    };
    let mut cache = EmbedCache::load(&cache_path, crate::embedder::MODEL_ID, crate::embedder::DIM);

    if plan.do_full {
        if !full && (index_path.exists() || state_path.exists()) {
            log.info("索引完整性校验未通过,自动执行全量重建。");
        }
        full_rebuild(
            IndexPaths {
                index: &index_path,
                state: &state_path,
            },
            &contents,
            &hashes,
            embedder,
            &mut cache,
            &log,
        )?;
    } else {
        let saved = plan.saved.as_ref().expect("完整性检查已确认 state 存在");
        if plan.count_mismatch {
            log.warn(&format!(
                "索引文档数与状态不一致(actual={}, expected={});先尝试增量修复",
                plan.doc_count.unwrap_or(0),
                saved.chunk_count
            ));
        }
        match incremental(
            IndexPaths {
                index: &index_path,
                state: &state_path,
            },
            &contents,
            &hashes,
            saved,
            embedder,
            &mut cache,
            &log,
        ) {
            Ok(()) => {}
            Err(error) if plan.count_mismatch => {
                log.warn(&format!("增量修复失败,降级为全量重建: {error}"));
                full_rebuild(
                    IndexPaths {
                        index: &index_path,
                        state: &state_path,
                    },
                    &contents,
                    &hashes,
                    embedder,
                    &mut cache,
                    &log,
                )?;
            }
            Err(error) => return Err(error),
        }
    }
    if let Err(error) = cache.save() {
        log.warn(&format!("写入嵌入缓存失败(不影响索引): {error}"));
    }
    Ok(true)
}

fn ensure_full_rebuild_allowed(do_full: bool, allowed: bool) -> Result<()> {
    if do_full && !allowed {
        anyhow::bail!(
            "索引不可用,需要全量重建: 请在知识库根目录运行 `colna index`(或 `colna sync`)后再试"
        );
    }
    Ok(())
}

fn sibling_path(path: &Path, suffix: &str) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    path.with_file_name(format!("{name}{suffix}"))
}

/// 清理上次中断留下的中间产物:
/// - 旧索引已改名、新索引还没换上的窗口 → 恢复旧索引;
/// - 未完成的新索引 → 删除;
/// - 已成功换名但没来得及清理的旧索引 → 删除。
fn recover_interrupted_swap(index_path: &Path, log: &Log) {
    let build_path = sibling_path(index_path, BUILD_SUFFIX);
    let previous_path = sibling_path(index_path, PREVIOUS_SUFFIX);

    if !index_path.exists() && previous_path.exists() {
        match std::fs::rename(&previous_path, index_path) {
            Ok(()) => log.warn("检测到中断的索引替换,已恢复上一版索引"),
            Err(error) => log.warn(&format!("恢复上一版索引失败: {error}")),
        }
    }
    if build_path.exists() {
        let _ = std::fs::remove_dir_all(&build_path);
    }
    if index_path.exists() && previous_path.exists() {
        let _ = std::fs::remove_dir_all(&previous_path);
    }
}

struct IndexPaths<'a> {
    index: &'a Path,
    state: &'a Path,
}

/// 全量重建:写到 `.new`,成功后再原子替换正式索引。
///
/// 任何一步失败(超时被杀、flush/optimize 报错)都只留下 `.new`,
/// 旧索引与旧 state 原样保留,不会再出现「索引归零、下次又得全量」。
fn full_rebuild(
    paths: IndexPaths<'_>,
    contents: &BTreeMap<String, String>,
    hashes: &BTreeMap<String, String>,
    embedder: &mut Embedder,
    cache: &mut EmbedCache,
    log: &Log,
) -> Result<()> {
    let IndexPaths {
        index: index_path,
        state: state_path,
    } = paths;
    let build_path = sibling_path(index_path, BUILD_SUFFIX);
    let previous_path = sibling_path(index_path, PREVIOUS_SUFFIX);
    if build_path.exists() {
        std::fs::remove_dir_all(&build_path)
            .with_context(|| format!("清理上次未完成的重建失败: {}", build_path.display()))?;
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

    let build_path_str = build_path.to_string_lossy().to_string();
    let collection = store::create(&build_path_str)?;

    if !chunks.is_empty() {
        log.info("分批生成向量并写入...");
        let n = embed_and_write_chunks(
            &collection,
            &chunks,
            WriteMode::Insert,
            embedder,
            cache,
            log,
        )?;
        log.info(&format!("✅ 写入 {n} 块"));
    } else {
        log.info("没有可索引的内容。");
    }
    store::finalize_collection(&collection, chunks.len() as u64)?;
    collection
        .close()
        .map_err(|e| anyhow!("关闭索引失败: {e}"))?;

    // 状态先落到临时文件,确保换名前状态已经完整写好。
    let new_state = state::IndexState::complete(hashes.clone(), chunk_ids);
    let pending_state = state_path.with_extension("json.new");
    if pending_state.exists() {
        let _ = std::fs::remove_file(&pending_state);
    }
    new_state.save(&pending_state)?;

    if index_path.exists() {
        if previous_path.exists() {
            std::fs::remove_dir_all(&previous_path)
                .with_context(|| format!("清理上一版索引失败: {}", previous_path.display()))?;
        }
        std::fs::rename(index_path, &previous_path)
            .with_context(|| format!("暂存旧索引失败: {}", index_path.display()))?;
    }
    std::fs::rename(&build_path, index_path)
        .with_context(|| format!("启用新索引失败: {}", build_path.display()))?;
    std::fs::rename(&pending_state, state_path)
        .with_context(|| format!("启用索引状态失败: {}", pending_state.display()))?;
    if previous_path.exists() {
        let _ = std::fs::remove_dir_all(&previous_path);
    }

    log.info(&format!("索引完成 → {}", index_path.display()));
    Ok(())
}

fn incremental(
    paths: IndexPaths<'_>,
    contents: &BTreeMap<String, String>,
    hashes: &BTreeMap<String, String>,
    old: &state::IndexState,
    embedder: &mut Embedder,
    cache: &mut EmbedCache,
    log: &Log,
) -> Result<()> {
    let IndexPaths {
        index: index_path,
        state: state_path,
    } = paths;
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
        let n = embed_and_write_chunks(
            &collection,
            &chunks,
            WriteMode::Upsert,
            embedder,
            cache,
            log,
        )?;
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
        // 只删自己的锁:旧进程 / 被手工清过锁的进程退出时不能误删新持有者的锁。
        if lock_pid(&self.dir) == Some(std::process::id()) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }
}

fn lock_pid(dir: &Path) -> Option<u32> {
    let content = std::fs::read_to_string(dir.join("pid")).ok()?;
    content
        .lines()
        .find_map(|line| line.strip_prefix("pid="))
        .and_then(|pid| pid.trim().parse::<u32>().ok())
}

fn is_stale(dir: &Path) -> bool {
    if let Some(pid) = lock_pid(dir) {
        return !process_alive(pid);
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

        assert!(needs_full_rebuild(false, true, None, true));
        assert!(needs_full_rebuild(false, false, Some(&state), true));
        assert!(needs_full_rebuild(true, true, Some(&state), true));
        assert!(!needs_full_rebuild(false, true, Some(&state), true));
        assert!(needs_full_rebuild(false, true, Some(&state), false));
    }

    fn temp_root() -> PathBuf {
        let id = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("colna-indexer-{id}-{nanos}"));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn write_dir(path: &Path) {
        std::fs::create_dir_all(path).unwrap();
        std::fs::write(path.join("manifest.0"), b"x").unwrap();
    }

    #[test]
    fn interrupted_swap_restores_the_previous_index() {
        let root = temp_root();
        let index = root.join("index.zvec");
        let previous = sibling_path(&index, PREVIOUS_SUFFIX);
        write_dir(&previous);

        recover_interrupted_swap(&index, &Log { quiet: true });

        assert!(index.exists(), "旧索引应被恢复");
        assert!(!previous.exists(), "恢复后不应留下 .old");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn uncommitted_build_is_discarded() {
        let root = temp_root();
        let index = root.join("index.zvec");
        let build = sibling_path(&index, BUILD_SUFFIX);
        write_dir(&index);
        write_dir(&build);

        recover_interrupted_swap(&index, &Log { quiet: true });

        assert!(index.exists());
        assert!(!build.exists(), "未完成的 .new 应被清理");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn lock_is_removed_only_by_its_owner() {
        let root = temp_root();
        let lock = IndexLock::acquire(&root).unwrap();
        let dir = root.join(".colna/index.lock.d");
        assert!(dir.exists());

        // 模拟锁被另一个进程接手:当前进程退出时不能删掉别人的锁。
        std::fs::write(dir.join("pid"), "pid=1\n").unwrap();
        drop(lock);
        assert!(dir.exists(), "非持有者不应删除锁目录");
        let _ = std::fs::remove_dir_all(&root);

        let root = temp_root();
        let lock = IndexLock::acquire(&root).unwrap();
        let dir = root.join(".colna/index.lock.d");
        drop(lock);
        assert!(!dir.exists(), "持有者退出后应释放锁");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn committed_index_drops_the_previous_generation() {
        let root = temp_root();
        let index = root.join("index.zvec");
        let previous = sibling_path(&index, PREVIOUS_SUFFIX);
        write_dir(&index);
        write_dir(&previous);

        recover_interrupted_swap(&index, &Log { quiet: true });

        assert!(index.exists());
        assert!(!previous.exists(), "换名完成后应清理 .old");
        let _ = std::fs::remove_dir_all(&root);
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

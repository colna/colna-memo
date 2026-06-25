//! colna — 跨设备个人知识库 CLI
//!
//! 子命令:
//!   colna index             扫描 memory/,增量重建本地 zvec 语义索引(--full 强制全量)
//!   colna search <query>    语义 / 混合检索
//!
//! 真源是 memory/ 下的 Markdown(走 git 跨设备同步);
//! 索引在本地 .colna/index.zvec(不入 git,可随时重建)。

mod chunker;
mod embedder;
mod mcp;
mod state;
mod store;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const MEMORY_DIR: &str = "memory";
const INDEX_PATH: &str = ".colna/index.zvec";
const STATE_PATH: &str = ".colna/state.json";

#[derive(Parser)]
#[command(name = "colna", about = "跨设备个人知识库:Git 存 Markdown,zvec 本地语义索引")]
struct Cli {
    /// 知识库根目录(默认当前目录)
    #[arg(long, global = true, default_value = ".")]
    root: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 扫描 memory/ 并(增量)重建本地语义索引
    Index {
        /// 强制全量重建(忽略增量状态)
        #[arg(long)]
        full: bool,
    },
    /// 语义 / 混合检索
    Search {
        /// 查询语句
        query: String,
        /// 返回条数
        #[arg(short, long, default_value_t = 5)]
        topk: usize,
        /// 只用向量语义检索,关闭 FTS 关键词混合
        #[arg(long)]
        semantic_only: bool,
    },
    /// 以 MCP server(stdio)运行,供 Claude 调用 kb_search / kb_get
    Mcp,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    store::init()?;
    let result = match &cli.command {
        Command::Index { full } => cmd_index(&cli.root, *full),
        Command::Search {
            query,
            topk,
            semantic_only,
        } => cmd_search(&cli.root, query, *topk, !*semantic_only),
        Command::Mcp => mcp::serve(&cli.root),
    };
    // 无论成功失败都尝试关闭运行时
    let _ = store::shutdown();
    result
}

/// 收集 memory/ 下所有 .md 文件,返回 (相对路径, 绝对路径)
fn collect_markdown(memory_root: &Path) -> Vec<(String, PathBuf)> {
    let mut files = Vec::new();
    for entry in WalkDir::new(memory_root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && path.extension().map(|e| e == "md").unwrap_or(false) {
            let rel = path
                .strip_prefix(memory_root)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();
            files.push((rel, path.to_path_buf()));
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

/// 扫描全部 md,返回 (相对路径 → 文件内容, 相对路径 → 内容指纹)。
fn scan(memory_root: &Path) -> Result<(BTreeMap<String, String>, BTreeMap<String, String>)> {
    let files = collect_markdown(memory_root);
    let mut contents = BTreeMap::new();
    let mut hashes = BTreeMap::new();
    for (rel, abs) in &files {
        let content =
            std::fs::read_to_string(abs).with_context(|| format!("读取失败: {}", abs.display()))?;
        hashes.insert(rel.clone(), state::content_hash(&content));
        contents.insert(rel.clone(), content);
    }
    Ok((contents, hashes))
}

/// 对给定的若干源文件切块。
fn chunks_for(rels: &[String], contents: &BTreeMap<String, String>) -> Vec<chunker::Chunk> {
    let mut chunks = Vec::new();
    for rel in rels {
        if let Some(content) = contents.get(rel) {
            chunks.extend(chunker::chunk_markdown(rel, content));
        }
    }
    chunks
}

fn cmd_index(root: &Path, full: bool) -> Result<()> {
    let memory_root = root.join(MEMORY_DIR);
    if !memory_root.is_dir() {
        anyhow::bail!("找不到 memory/ 目录: {}", memory_root.display());
    }
    let index_path = root.join(INDEX_PATH);
    let state_path = root.join(STATE_PATH);

    let (contents, hashes) = scan(&memory_root)?;
    let do_full = full || !index_path.exists();

    if do_full {
        full_rebuild(&index_path, &state_path, &contents, &hashes)
    } else {
        incremental(&index_path, &state_path, &contents, &hashes)
    }
}

/// 全量重建:删旧索引,重新切块 / 嵌入 / 写入。
fn full_rebuild(
    index_path: &Path,
    state_path: &Path,
    contents: &BTreeMap<String, String>,
    hashes: &BTreeMap<String, String>,
) -> Result<()> {
    if index_path.exists() {
        std::fs::remove_dir_all(index_path).ok();
    }
    if let Some(parent) = index_path.parent() {
        std::fs::create_dir_all(parent).context("创建 .colna 目录失败")?;
    }

    let all_rels: Vec<String> = contents.keys().cloned().collect();
    println!("全量重建:扫描到 {} 个 Markdown 文件", all_rels.len());
    let chunks = chunks_for(&all_rels, contents);
    println!("共切出 {} 个内容块", chunks.len());

    let index_path_str = index_path.to_string_lossy().to_string();
    let collection = store::create(&index_path_str)?;

    if !chunks.is_empty() {
        println!("加载 embedding 模型(首次会下载)...");
        let mut emb = embedder::Embedder::new()?;
        let texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
        println!("生成向量中...");
        let vectors = emb.embed_passages(&texts)?;
        let n = store::insert_chunks(&collection, &chunks, &vectors)?;
        println!("✅ 写入 {} 块", n);
    } else {
        println!("没有可索引的内容。");
    }
    collection.flush().ok();
    collection.close().ok();

    state::IndexState {
        files: hashes.clone(),
    }
    .save(state_path)?;
    println!("索引完成 → {}", index_path_str);
    Ok(())
}

/// 增量更新:只处理新增 / 变更 / 删除的文件。
fn incremental(
    index_path: &Path,
    state_path: &Path,
    contents: &BTreeMap<String, String>,
    hashes: &BTreeMap<String, String>,
) -> Result<()> {
    let old = state::IndexState::load(state_path);
    let d = state::diff(&old, hashes);

    if d.changed.is_empty() && d.removed.is_empty() {
        println!("✅ 无变化,索引已是最新({} 个文件)", hashes.len());
        return Ok(());
    }
    println!(
        "增量更新:变更/新增 {} 个,删除 {} 个",
        d.changed.len(),
        d.removed.len()
    );

    let index_path_str = index_path.to_string_lossy().to_string();
    let collection = store::open(&index_path_str)?;

    // 1. 先删:被删文件 + 变更文件的旧块(变更文件的块 id 可能变,统一按 source_path 清掉)
    for rel in d.removed.iter().chain(d.changed.iter()) {
        store::delete_by_source(&collection, rel)?;
    }

    // 2. 再写:变更/新增文件重新切块 + 嵌入 + upsert
    let chunks = chunks_for(&d.changed, contents);
    if !chunks.is_empty() {
        println!("重新嵌入 {} 个内容块...", chunks.len());
        let mut emb = embedder::Embedder::new()?;
        let texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
        let vectors = emb.embed_passages(&texts)?;
        let n = store::upsert_chunks(&collection, &chunks, &vectors)?;
        println!("✅ upsert {} 块", n);
    }
    collection.flush().ok();
    collection.close().ok();

    // 3. 状态对齐磁盘
    state::IndexState {
        files: hashes.clone(),
    }
    .save(state_path)?;
    println!("增量索引完成 → {}", index_path_str);
    Ok(())
}

fn cmd_search(root: &Path, query: &str, topk: usize, hybrid: bool) -> Result<()> {
    let index_path = root.join(INDEX_PATH);
    let index_path_str = index_path.to_string_lossy().to_string();

    let mut emb = embedder::Embedder::new()?;
    let qv = emb.embed_query(query)?;

    let collection = store::open(&index_path_str)?;
    let hits = if hybrid {
        store::hybrid_search(&collection, &qv, query, topk)?
    } else {
        store::vector_search(&collection, &qv, topk)?
    };
    collection.close().ok();

    if hits.is_empty() {
        println!("无匹配结果。");
        return Ok(());
    }
    let mode = if hybrid { "混合(向量+FTS)" } else { "纯向量" };
    println!("查询: {}  [{}]\n", query, mode);
    for (i, h) in hits.iter().enumerate() {
        let loc = if h.heading.is_empty() {
            h.title.clone()
        } else {
            format!("{} › {}", h.title, h.heading)
        };
        println!("#{}  [score {:.4}]  {}  ({})", i + 1, h.score, loc, h.source_path);
        if !h.tags.is_empty() || !h.date.is_empty() {
            println!("    ({} {})", h.date, h.tags);
        }
        let snippet: String = h.text.chars().take(160).collect();
        println!("    {}\n", snippet.replace('\n', " "));
    }
    Ok(())
}

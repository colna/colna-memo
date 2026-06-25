//! colna — 跨设备个人知识库 CLI
//!
//! 子命令:
//!   colna index             扫描 memory/ 全量重建本地 zvec 语义索引
//!   colna search <query>    语义检索
//!
//! 真源是 memory/ 下的 Markdown(走 git 跨设备同步);
//! 索引在本地 .colna/index.zvec(不入 git,可随时重建)。

mod chunker;
mod embedder;
mod store;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const MEMORY_DIR: &str = "memory";
const INDEX_PATH: &str = ".colna/index.zvec";

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
    /// 扫描 memory/ 并全量重建本地语义索引
    Index,
    /// 语义检索
    Search {
        /// 查询语句
        query: String,
        /// 返回条数
        #[arg(short, long, default_value_t = 5)]
        topk: usize,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    store::init()?;
    let result = match &cli.command {
        Command::Index => cmd_index(&cli.root),
        Command::Search { query, topk } => cmd_search(&cli.root, query, *topk),
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

fn cmd_index(root: &Path) -> Result<()> {
    let memory_root = root.join(MEMORY_DIR);
    if !memory_root.is_dir() {
        anyhow::bail!("找不到 memory/ 目录: {}", memory_root.display());
    }
    let index_path = root.join(INDEX_PATH);

    // 全量重建:删旧索引目录
    if index_path.exists() {
        std::fs::remove_dir_all(&index_path).ok();
    }
    if let Some(parent) = index_path.parent() {
        std::fs::create_dir_all(parent).context("创建 .colna 目录失败")?;
    }

    // 1. 扫描 + 切块
    let files = collect_markdown(&memory_root);
    println!("扫描到 {} 个 Markdown 文件", files.len());
    let mut chunks = Vec::new();
    for (rel, abs) in &files {
        let content = std::fs::read_to_string(abs)
            .with_context(|| format!("读取失败: {}", abs.display()))?;
        chunks.extend(chunker::chunk_markdown(rel, &content));
    }
    println!("共切出 {} 个内容块", chunks.len());
    if chunks.is_empty() {
        println!("没有可索引的内容,结束。");
        return Ok(());
    }

    // 2. 本地 embedding
    println!("加载 embedding 模型(首次会下载)...");
    let mut emb = embedder::Embedder::new()?;
    let texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
    println!("生成向量中...");
    let vectors = emb.embed_passages(&texts)?;

    // 3. 写入 zvec
    let index_path_str = index_path.to_string_lossy().to_string();
    let collection = store::create(&index_path_str)?;
    let n = store::insert_chunks(&collection, &chunks, &vectors)?;
    collection.flush().ok();
    collection.close().ok();
    println!("✅ 索引完成:写入 {} 块 → {}", n, index_path_str);
    Ok(())
}

fn cmd_search(root: &Path, query: &str, topk: usize) -> Result<()> {
    let index_path = root.join(INDEX_PATH);
    let index_path_str = index_path.to_string_lossy().to_string();

    let mut emb = embedder::Embedder::new()?;
    let qv = emb.embed_query(query)?;

    let collection = store::open(&index_path_str)?;
    let hits = store::search(&collection, &qv, topk)?;
    collection.close().ok();

    if hits.is_empty() {
        println!("无匹配结果。");
        return Ok(());
    }
    println!("查询: {}\n", query);
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

//! colna — 跨设备个人知识库 CLI
//!
//! 子命令:
//!   colna index             扫描 memory/,增量重建本地 zvec 语义索引
//!   colna search <query>    语义 / 混合检索
//!   colna context <task>    生成可注入 agent 的上下文包
//!   colna clip <url>        网页正文 → Markdown → 00-Inbox/
//!
//! 真源是 memory/ 下的 Markdown(走 git 跨设备同步);
//! 索引在本地 .colna/index.zvec(不入 git,可随时重建)。

mod chunker;
mod clip;
mod context;
mod digest;
mod embedder;
mod encrypt;
mod eval;
mod gitsync;
mod graph;
mod ignore;
mod indexer;
mod ingest;
mod mcp;
mod rerank;
mod scope;
mod search;
mod secrets;
mod skills;
mod state;
mod store;
mod task;
mod util;
mod watch;
mod write;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "colna",
    about = "跨设备个人知识库:Git 存 Markdown,zvec 本地语义索引"
)]
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
        query: String,
        #[arg(short, long, default_value_t = 5)]
        topk: usize,
        #[arg(long)]
        semantic_only: bool,
        #[arg(long = "tag")]
        tags: Vec<String>,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        since: Option<String>,
        #[arg(long)]
        until: Option<String>,
        #[arg(long = "type")]
        kind: Option<String>,
        #[arg(long)]
        no_ensure_index: bool,
        #[arg(long)]
        no_mmr: bool,
        #[arg(long)]
        no_summary: bool,
        #[arg(long, default_value_t = 0.15)]
        recency_weight: f32,
        #[arg(long, default_value_t = 0.05)]
        link_boost: f32,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long)]
        expand: bool,
        #[arg(long)]
        rerank: bool,
    },
    /// 生成可注入 agent 的上下文包
    Context {
        task: String,
        #[arg(short, long, default_value_t = 8)]
        topk: usize,
        #[arg(long, default_value_t = 12_000)]
        max_chars: usize,
        #[arg(long)]
        scope: Option<String>,
    },
    /// 网页正文 → Markdown → 00-Inbox/
    Clip {
        url: String,
        #[arg(short, long)]
        title: Option<String>,
        #[arg(long, default_value = "")]
        tags: String,
    },
    /// 附件 / 文档摄取:PDF、HTML、图片、音视频
    Ingest {
        path: PathBuf,
        #[arg(short, long)]
        title: Option<String>,
        #[arg(long, default_value = "")]
        tags: String,
    },
    /// 生成 Daily digest(默认只打印)
    Digest {
        #[arg(long)]
        date: Option<String>,
        #[arg(long)]
        write: bool,
        #[arg(long)]
        json: bool,
        /// 为可沉淀经验找语义相近的已有笔记
        #[arg(long)]
        semantic: bool,
    },
    /// 把前一天未完成项转入今日 Daily,并写 digest
    Consolidate {
        #[arg(long)]
        date: Option<String>,
        #[arg(long)]
        apply: bool,
    },
    /// 列出 / 同步 open work 清单
    Work {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        sync: bool,
    },
    /// 检索评测(recall@k / MRR)
    Eval {
        file: PathBuf,
        #[arg(short, long, default_value_t = 5)]
        topk: usize,
        #[arg(long)]
        json: bool,
    },
    /// 查看某篇笔记的 backlinks / 语义邻居
    Related {
        path: String,
        #[arg(short, long, default_value_t = 5)]
        topk: usize,
        #[arg(long)]
        json: bool,
    },
    /// 输出笔记图谱
    Graph {
        #[arg(long)]
        json: bool,
    },
    /// 任务 receipt
    Task {
        #[command(subcommand)]
        command: TaskCommand,
    },
    /// 轮询索引变化
    Watch {
        #[arg(long, default_value_t = 60)]
        interval: u64,
        #[arg(long)]
        once: bool,
        /// 索引变化后生成当天的 Digest
        #[arg(long)]
        digest: bool,
    },
    /// 监控 URL 变化 / 价格,写回 Daily
    WatchUrl {
        url: String,
        #[arg(long)]
        contains: Option<String>,
        #[arg(long)]
        price: bool,
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// 扫描 / 脱敏敏感信息
    Secrets {
        #[arg(long)]
        fix: bool,
        #[arg(long)]
        json: bool,
    },
    /// age 加密
    Encrypt {
        path: PathBuf,
        #[arg(long)]
        recipient: Option<String>,
        #[arg(long)]
        passphrase: bool,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// age 解密到 stdout
    Decrypt { path: PathBuf },
    /// 技能索引
    Skills {
        #[command(subcommand)]
        command: SkillsCommand,
    },
    /// 以 MCP server(stdio)运行,供 Claude 调用知识库工具
    Mcp,
    /// 新建一篇笔记到 memory/00-Inbox/
    Add {
        title: String,
        #[arg(short, long)]
        body: Option<String>,
        #[arg(short, long, default_value = "")]
        tags: String,
    },
    /// 跨设备同步:commit memory → pull --rebase --autostash → reindex → push
    Sync {
        #[arg(short, long, default_value = "colna sync")]
        message: String,
    },
}

#[derive(Subcommand)]
enum TaskCommand {
    Start {
        goal: String,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        plan: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    Checkpoint {
        path: String,
        note: String,
        #[arg(long)]
        dry_run: bool,
    },
    Finish {
        path: String,
        #[arg(long, default_value = "done")]
        status: String,
        #[arg(long)]
        outcome: String,
        #[arg(long)]
        evidence: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum SkillsCommand {
    Index {
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    Search {
        query: String,
        #[arg(short, long, default_value_t = 5)]
        topk: usize,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    store::init()?;
    let result = match &cli.command {
        Command::Index { full } => indexer::run(&cli.root, *full, false),
        Command::Search {
            query,
            topk,
            semantic_only,
            tags,
            path,
            since,
            until,
            kind,
            no_ensure_index,
            no_mmr,
            no_summary,
            recency_weight,
            link_boost,
            scope,
            expand,
            rerank,
        } => cmd_search(
            &cli.root,
            &SearchRequest {
                query,
                topk: *topk,
                hybrid: !*semantic_only,
                tags,
                path: path.as_deref(),
                since: since.as_deref(),
                until: until.as_deref(),
                kind: kind.as_deref(),
                no_ensure_index: *no_ensure_index,
                no_mmr: *no_mmr,
                no_summary: *no_summary,
                recency_weight: *recency_weight,
                link_boost: *link_boost,
                scope: scope.as_deref(),
                expand: *expand,
                rerank: *rerank,
            },
        ),
        Command::Context {
            task,
            topk,
            max_chars,
            scope,
        } => cmd_context(&cli.root, task, *topk, *max_chars, scope.as_deref()),
        Command::Clip { url, title, tags } => cmd_clip(&cli.root, url, title.as_deref(), tags),
        Command::Ingest { path, title, tags } => {
            let path = ingest::ingest(&cli.root, path, title.as_deref(), tags)?;
            println!("📥 已摄取 → {}", path.display());
            Ok(())
        }
        Command::Digest {
            date,
            write,
            json,
            semantic,
        } => cmd_digest(&cli.root, date.as_deref(), *write, *json, *semantic),
        Command::Consolidate { date, apply } => cmd_consolidate(&cli.root, date.as_deref(), *apply),
        Command::Work { json, sync } => cmd_work(&cli.root, *json, *sync),
        Command::Eval { file, topk, json } => cmd_eval(&cli.root, file, *topk, *json),
        Command::Related { path, topk, json } => cmd_related(&cli.root, path, *topk, *json),
        Command::Graph { json } => cmd_graph(&cli.root, *json),
        Command::Task { command } => cmd_task(&cli.root, command),
        Command::Watch {
            interval,
            once,
            digest,
        } => watch::watch_index(&cli.root, *interval, *once, *digest),
        Command::WatchUrl {
            url,
            contains,
            price,
            note,
            dry_run,
        } => {
            watch::watch_url(
                &cli.root,
                url,
                contains.as_deref(),
                *price,
                note.as_deref(),
                *dry_run,
            )?;
            Ok(())
        }
        Command::Secrets { fix, json } => cmd_secrets(&cli.root, *fix, *json),
        Command::Encrypt {
            path,
            recipient,
            passphrase,
            output,
        } => {
            let output =
                encrypt::encrypt(path, recipient.as_deref(), *passphrase, output.as_deref())?;
            println!("🔐 已加密 → {}", output.display());
            Ok(())
        }
        Command::Decrypt { path } => {
            print!("{}", encrypt::decrypt(path)?);
            Ok(())
        }
        Command::Skills { command } => cmd_skills(&cli.root, command),
        Command::Mcp => mcp::serve(&cli.root),
        Command::Add { title, body, tags } => cmd_add(&cli.root, title, body.as_deref(), tags),
        Command::Sync { message } => cmd_sync(&cli.root, message),
    };
    let _ = store::shutdown();
    result
}

struct SearchRequest<'a> {
    query: &'a str,
    topk: usize,
    hybrid: bool,
    tags: &'a [String],
    path: Option<&'a str>,
    since: Option<&'a str>,
    until: Option<&'a str>,
    kind: Option<&'a str>,
    no_ensure_index: bool,
    no_mmr: bool,
    no_summary: bool,
    recency_weight: f32,
    link_boost: f32,
    scope: Option<&'a str>,
    expand: bool,
    rerank: bool,
}

fn cmd_search(root: &Path, request: &SearchRequest<'_>) -> Result<()> {
    if request.path.is_some() && request.kind.is_some() {
        anyhow::bail!("--path 与 --type 只能选一个");
    }
    for (label, date) in [("since", request.since), ("until", request.until)] {
        if let Some(date) = date {
            if util::parse_date(date).is_none() {
                anyhow::bail!("--{label} 必须是 YYYY-MM-DD: {date}");
            }
        }
    }
    let path = match (request.path, request.kind) {
        (Some(path), _) => Some(path.to_string()),
        (None, Some(kind)) => Some(
            util::type_prefix(kind)
                .ok_or_else(|| anyhow::anyhow!("未知类型: {kind}"))?
                .to_string(),
        ),
        (None, None) => None,
    };
    let scope = scope::resolve(root, request.scope)?;

    let mut embedder = embedder::Embedder::new()?;
    if !request.no_ensure_index {
        indexer::ensure(root, false, false, Some(&mut embedder))?;
    }
    let link_counts = if request.link_boost > 0.0 {
        Some(graph::in_degree(root)?)
    } else {
        None
    };

    let options = search::SearchOptions {
        topk: if request.rerank {
            request.topk * 2
        } else {
            request.topk
        },
        hybrid: request.hybrid,
        filters: search::Filters {
            tags: request.tags.to_vec(),
            path,
            since: request.since.map(str::to_string),
            until: request.until.map(str::to_string),
            scope,
        },
        recency_weight: request.recency_weight,
        mmr: !request.no_mmr,
        include_summary: !request.no_summary,
        link_boost: request.link_boost,
    };
    let queries = if request.expand {
        search::expand_queries(request.query)
    } else {
        vec![request.query.to_string()]
    };
    if request.expand && queries.len() == 1 {
        eprintln!("未配置 COLNA_QUERY_EXPAND_CMD,使用原 query");
    }

    let collection = store::open(&root.join(indexer::INDEX_PATH).to_string_lossy())?;
    let mut hits = search::query_expanded(
        &collection,
        &mut embedder,
        &queries,
        &options,
        link_counts.as_ref(),
    )?;
    if request.rerank {
        let reranker = rerank::Reranker::new()?;
        let documents: Vec<String> = hits.iter().map(|hit| hit.text.clone()).collect();
        let ordered = reranker.rerank(request.query, &documents, request.topk)?;
        let mut reranked = Vec::with_capacity(ordered.len());
        for (index, score) in ordered {
            if let Some(mut hit) = hits.get(index).cloned() {
                hit.score = score;
                reranked.push(hit);
            }
        }
        hits = reranked;
    }
    collection.close().ok();

    print_hits(&hits, request.query, request.hybrid);
    Ok(())
}

fn print_hits(hits: &[store::Hit], query: &str, hybrid: bool) {
    if hits.is_empty() {
        println!("无匹配结果。");
        return;
    }
    let mode = if hybrid {
        "混合(向量+FTS)"
    } else {
        "纯向量"
    };
    println!("查询: {query}  [{mode}]\n");
    for (index, hit) in hits.iter().enumerate() {
        let location = if hit.breadcrumb.is_empty() {
            hit.title.clone()
        } else {
            format!("{} › {}", hit.title, hit.breadcrumb)
        };
        let tags = if hit.kind == "summary" {
            " [summary]"
        } else {
            ""
        };
        println!(
            "#{}  [score {:.4}]  {}{}  ({})",
            index + 1,
            hit.score,
            location,
            tags,
            hit.source_path
        );
        let mut detail = Vec::new();
        if let Some(vector) = hit.vector_score {
            detail.push(format!("vec={vector:.4}"));
        }
        if let Some(fts) = hit.fts_score {
            detail.push(format!("fts={fts:.4}"));
        }
        if hit.recency_boost > 0.0 {
            detail.push(format!("recency=+{:.4}", hit.recency_boost));
        }
        if hit.link_boost > 0.0 {
            detail.push(format!("links=+{:.4}", hit.link_boost));
        }
        if hit.mmr_penalty > 0.0 {
            detail.push(format!("mmr=-{:.4}", hit.mmr_penalty));
        }
        if !detail.is_empty() {
            println!("    ({})", detail.join(" "));
        }
        if !hit.tags.is_empty() || !hit.date.is_empty() {
            println!("    ({} {})", hit.date, hit.tags);
        }
        let snippet: String = hit.text.chars().take(160).collect();
        println!("    {}\n", snippet.replace('\n', " "));
    }
}

fn cmd_context(
    root: &Path,
    task: &str,
    topk: usize,
    max_chars: usize,
    scope: Option<&str>,
) -> Result<()> {
    let mut embedder = embedder::Embedder::new()?;
    let scope = scope::resolve(root, scope)?;
    let pack = context::build_with_embedder(root, task, topk, max_chars, &mut embedder, scope)?;
    print!("{pack}");
    Ok(())
}

fn cmd_clip(root: &Path, url: &str, title: Option<&str>, tags: &str) -> Result<()> {
    let fetched = clip::fetch(url)?;
    let title = title
        .map(str::to_string)
        .or(fetched.title)
        .unwrap_or_else(|| url.to_string());
    let extra = [
        ("source_url", url),
        ("fetched_at", fetched.fetched_at.as_str()),
        ("content_hash", fetched.content_hash.as_str()),
    ];
    let path = write::add_note(root, &title, tags, &fetched.markdown, &extra, reindex)?;
    println!("📥 已剪藏 → {}", path.display());
    Ok(())
}

fn cmd_digest(
    root: &Path,
    date: Option<&str>,
    write: bool,
    json: bool,
    semantic: bool,
) -> Result<()> {
    let date = date.map(str::to_string).unwrap_or_else(util::today_beijing);
    digest::validate_date(&date)?;
    let report = digest::analyze(root, &date)?;
    let similar = if semantic {
        let mut embedder = embedder::Embedder::new()?;
        digest::annotate_similar(root, &report, &mut embedder)?
    } else {
        Vec::new()
    };

    if json {
        let mut value = serde_json::to_value(&report)?;
        value["similar"] = serde_json::to_value(&similar)?;
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        print!("{}", digest::render(&report));
        for (lesson, path, score) in &similar {
            println!("\n可能已记录: {path} (score {score:.2}) ← {lesson}");
        }
    }

    if write {
        let path = digest::write_digest(root, &report)?;
        eprintln!("📝 digest → {}", path.display());
    }
    Ok(())
}

fn cmd_consolidate(root: &Path, date: Option<&str>, apply: bool) -> Result<()> {
    let date = date
        .map(str::to_string)
        .unwrap_or_else(util::yesterday_beijing);
    digest::validate_date(&date)?;
    let report = digest::analyze(root, &date)?;
    print!("{}", digest::render(&report));
    if !apply {
        println!("\n[dry-run] 用 `--apply` 转抄未完成项并写 digest。");
        return Ok(());
    }
    let today = util::today_beijing();
    digest::ensure_daily_exists(root, &today)?;
    let added = digest::carry_over_todos(root, &date)?;
    let path = digest::write_digest(root, &report)?;
    println!(
        "\n✅ 转入今日待办 {} 条;digest → {}",
        added.len(),
        path.display()
    );
    Ok(())
}

fn cmd_work(root: &Path, json: bool, sync: bool) -> Result<()> {
    if sync {
        let path = digest::sync_open_work(root)?;
        println!("✅ open work → {}", path.display());
        return Ok(());
    }
    let items = digest::list_open_work(root)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&items)?);
        return Ok(());
    }
    let mut by_path: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for item in items {
        by_path.entry(item.path).or_default().push(item.text);
    }
    if by_path.is_empty() {
        println!("无未完成项。");
        return Ok(());
    }
    for (path, items) in by_path {
        println!("{path}");
        for item in items {
            println!("  {item}");
        }
    }
    Ok(())
}

fn cmd_eval(root: &Path, file: &Path, topk: usize, json: bool) -> Result<()> {
    let cases = eval::load(file)?;
    let report = eval::run(root, &cases, topk)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&eval::to_json(&report))?);
    } else {
        print!("{}", eval::render_text(&report));
    }
    Ok(())
}

fn cmd_related(root: &Path, path: &str, topk: usize, json: bool) -> Result<()> {
    let graph = graph::build(root)?;
    let inbound = graph.inbound(path);
    let outbound = graph.outbound(path);
    let mut embedder = embedder::Embedder::new()?;
    indexer::ensure(root, false, false, Some(&mut embedder))?;
    let query = write::read_note(root, path)
        .map(|content| content.chars().take(1_000).collect::<String>())
        .unwrap_or_else(|_| path.to_string());
    let collection = store::open(&root.join(indexer::INDEX_PATH).to_string_lossy())?;
    let query_vec = embedder.embed_query(&query)?;
    let hits = search::query(
        &collection,
        &query_vec,
        &query,
        &search::SearchOptions {
            topk,
            hybrid: true,
            filters: search::Filters::default(),
            recency_weight: 0.0,
            mmr: true,
            include_summary: true,
            link_boost: 0.0,
        },
        None,
    )?;
    collection.close().ok();
    let mut seen_sources = std::collections::HashSet::new();
    let semantic: Vec<&store::Hit> = hits
        .iter()
        .filter(|hit| hit.source_path != path && seen_sources.insert(hit.source_path.clone()))
        .collect();

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "path": path,
                "inbound": inbound,
                "outbound": outbound,
                "semantic": semantic.iter().map(|hit| serde_json::json!({
                    "source_path": hit.source_path,
                    "title": hit.title,
                    "breadcrumb": hit.breadcrumb,
                    "score": hit.score,
                })).collect::<Vec<_>>(),
            }))?
        );
        return Ok(());
    }

    println!("{path}");
    println!(
        "  inbound: {}",
        if inbound.is_empty() {
            "—".to_string()
        } else {
            inbound.join(", ")
        }
    );
    println!(
        "  outbound: {}",
        if outbound.is_empty() {
            "—".to_string()
        } else {
            outbound.join(", ")
        }
    );
    if !semantic.is_empty() {
        println!("  semantic:");
        for hit in semantic {
            println!("    {:.4}  {}  ({})", hit.score, hit.title, hit.source_path);
        }
    }
    Ok(())
}

fn cmd_graph(root: &Path, json: bool) -> Result<()> {
    let graph = graph::build(root)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&graph::to_json(&graph))?);
        return Ok(());
    }
    println!("nodes: {}  edges: {}", graph.nodes.len(), graph.edges.len());
    for edge in graph.edges {
        println!("{} → {}", edge.from, edge.to);
    }
    Ok(())
}

fn cmd_task(root: &Path, command: &TaskCommand) -> Result<()> {
    match command {
        TaskCommand::Start {
            goal,
            project,
            plan,
            dry_run,
        } => {
            let preview = task::start(root, goal, project.as_deref(), plan.as_deref(), !dry_run)?;
            print_note_preview(&preview);
        }
        TaskCommand::Checkpoint {
            path,
            note,
            dry_run,
        } => {
            let preview = task::checkpoint(root, path, note, !dry_run)?;
            print_note_preview(&preview);
        }
        TaskCommand::Finish {
            path,
            status,
            outcome,
            evidence,
            dry_run,
        } => {
            let preview = task::finish(root, path, status, outcome, evidence.as_deref(), !dry_run)?;
            print_note_preview(&preview);
        }
        TaskCommand::List { status, json } => {
            let tasks = task::list(root, status.as_deref())?;
            if *json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&tasks.iter().map(task_json).collect::<Vec<_>>())?
                );
            } else if tasks.is_empty() {
                println!("无任务。");
            } else {
                for item in tasks {
                    println!(
                        "[{}] {}  {}  ({})",
                        item.status, item.date, item.title, item.path
                    );
                }
            }
        }
    }
    Ok(())
}

fn task_json(task: &task::Task) -> serde_json::Value {
    serde_json::json!({
        "path": task.path,
        "title": task.title,
        "status": task.status,
        "project": task.project,
        "goal": task.goal,
        "date": task.date,
    })
}

fn print_note_preview(preview: &write::NotePreview) {
    if preview.applied {
        println!("✅ 已写入 → {} (hash {})", preview.path, preview.hash);
    } else {
        println!("预览(未写入)→ {} (hash {})", preview.path, preview.hash);
        println!("{}", preview.diff);
    }
}

fn cmd_secrets(root: &Path, fix: bool, json: bool) -> Result<()> {
    let findings = secrets::scan(root)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &findings
                    .iter()
                    .map(|finding| serde_json::json!({
                        "path": finding.path,
                        "line": finding.line,
                        "kind": finding.kind,
                        "snippet": finding.snippet,
                    }))
                    .collect::<Vec<_>>()
            )?
        );
    } else if findings.is_empty() {
        println!("未发现敏感信息。");
    } else {
        for finding in &findings {
            println!(
                "{}:{} [{}] {}",
                finding.path, finding.line, finding.kind, finding.snippet
            );
        }
    }
    if fix && !findings.is_empty() {
        let changed = secrets::fix(root, &findings)?;
        println!("✅ 已脱敏 {changed} 个文件");
    }
    Ok(())
}

fn cmd_skills(root: &Path, command: &SkillsCommand) -> Result<()> {
    match command {
        SkillsCommand::Index { dir } => {
            let dir = dir.clone().unwrap_or_else(skills::default_skills_root);
            let count = skills::index(root, &dir)?;
            println!("✅ 技能索引完成:{count} 个技能");
        }
        SkillsCommand::Search { query, topk } => {
            let mut embedder = embedder::Embedder::new()?;
            let hits = skills::search(root, query, *topk, &mut embedder)?;
            print_hits(&hits, query, true);
        }
    }
    Ok(())
}

/// 增量重建索引(供 add / sync 复用)。
fn reindex(root: &Path) -> Result<()> {
    indexer::run(root, false, false)
}

fn cmd_add(root: &Path, title: &str, body: Option<&str>, tags: &str) -> Result<()> {
    let body = match body {
        Some(body) => body.to_string(),
        None => {
            use std::io::Read;
            let mut buffer = String::new();
            std::io::stdin()
                .read_to_string(&mut buffer)
                .map_err(|error| anyhow::anyhow!("从 stdin 读取正文失败: {error}"))?;
            buffer
        }
    };
    let path = write::add_note(root, title, tags, &body, &[], reindex)?;
    println!("📝 已新建笔记 → {}", path.display());
    Ok(())
}

fn cmd_sync(root: &Path, message: &str) -> Result<()> {
    gitsync::sync(root, message, reindex)
}

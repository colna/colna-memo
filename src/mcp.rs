//! MCP server(stdio / JSON-RPC 2.0):让 Claude 直接调用知识库。
//!
//! 读工具:`kb_search` / `kb_get` / `kb_context`
//! 写工具:`kb_put` / `kb_append` / `kb_update` / `kb_archive`
//!
//! 写工具默认只返回预览(`confirm=false`),真正落盘需要再调一次并传
//! `confirm=true`;更新/追加必须带 `expected_hash` 做乐观锁。
//! stdout 只输出协议消息,所有日志走 stderr。

use crate::context;
use crate::digest;
use crate::embedder::Embedder;
use crate::graph;
use crate::indexer;
use crate::rerank;
use crate::scope;
use crate::search::{self, Filters, SearchOptions};
use crate::skills;
use crate::store;
use crate::task;
use crate::util;
use crate::write;
use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

const PROTOCOL_VERSION: &str = "2024-11-05";
const MEMORY_DIR: &str = "memory";

/// 运行 MCP server,直到 stdin 关闭。
pub fn serve(root: &Path) -> Result<()> {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    let mut emb: Option<Embedder> = None;

    eprintln!("colna mcp: 已启动,root = {}", root.display());

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => break,
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let req: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("colna mcp: 跳过无法解析的行: {error}");
                continue;
            }
        };

        let id = req.get("id").cloned();
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let is_notification = id.is_none();

        let response = match method {
            "initialize" => Some(make_result(id.clone(), initialize_result(&req))),
            "tools/list" => Some(make_result(id.clone(), tools_list())),
            "tools/call" => {
                let result = handle_tool_call(root, &mut emb, &req);
                Some(match result {
                    Ok(value) => make_result(id.clone(), value),
                    Err(error) => make_result(id.clone(), tool_error(&error.to_string())),
                })
            }
            "ping" => Some(make_result(id.clone(), json!({}))),
            "notifications/initialized" | "notifications/cancelled" => None,
            _ if is_notification => None,
            _ => Some(make_error(
                id.clone(),
                -32601,
                &format!("未知方法: {method}"),
            )),
        };

        if let Some(response) = response {
            let text = serde_json::to_string(&response)?;
            writeln!(out, "{text}")?;
            out.flush()?;
        }
    }
    Ok(())
}

fn initialize_result(req: &Value) -> Value {
    let version = req
        .get("params")
        .and_then(|params| params.get("protocolVersion"))
        .and_then(|value| value.as_str())
        .unwrap_or(PROTOCOL_VERSION)
        .to_string();
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "colna-memo", "version": env!("CARGO_PKG_VERSION") }
    })
}

fn tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "kb_search",
                "description": "在个人知识库(memory/ 下的 Markdown)中做语义/混合检索,返回最相关的内容块。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "自然语言或关键词查询" },
                        "topk": { "type": "integer", "description": "返回条数,默认 5" },
                        "semantic_only": { "type": "boolean", "description": "true=只用向量语义检索;默认 false=向量+FTS 混合" },
                        "tags": { "type": "array", "items": { "type": "string" }, "description": "标签过滤;数组内所有标签都必须命中" },
                        "path": { "type": "string", "description": "路径前缀,如 10-Projects/colna-memo" },
                        "since": { "type": "string", "description": "起始日期 YYYY-MM-DD(含)" },
                        "until": { "type": "string", "description": "结束日期 YYYY-MM-DD(含)" },
                        "type": { "type": "string", "description": "类型过滤:inbox/project/area/resource/archive/daily" },
                        "ensure_index": { "type": "boolean", "description": "检索前先做增量索引校验,默认 true" },
                        "mmr": { "type": "boolean", "description": "是否启用 MMR 去重,默认 true" },
                        "recency_weight": { "type": "number", "description": "时效加分权重,默认 0.15;0 关闭" },
                        "link_boost": { "type": "number", "description": "入链加权权重,默认 0.05;0 关闭" },
                        "include_summary": { "type": "boolean", "description": "是否返回整篇摘要块,默认 true" },
                        "scope": { "type": "string", "description": "scope 名(见 .colna/scopes.json)" },
                        "expand": { "type": "boolean", "description": "用 COLNA_QUERY_EXPAND_CMD 做查询扩展" },
                        "rerank": { "type": "boolean", "description": "使用 BGE cross-encoder 重排(首次下载模型)" }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "kb_get",
                "description": "按相对路径(相对 memory/,如 00-Inbox/welcome.md)取回某个 Markdown 文件的全文。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "source_path": { "type": "string", "description": "相对 memory/ 的文件路径" }
                    },
                    "required": ["source_path"]
                }
            },
            {
                "name": "kb_context",
                "description": "为一个任务生成紧凑上下文包:书写约定 + 今日重点 + 最近工作日志 + 相关笔记 + 排错经验。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "task": { "type": "string", "description": "任务描述" },
                        "topk": { "type": "integer", "description": "检索候选条数,默认 8" },
                        "max_chars": { "type": "integer", "description": "输出字符上限,默认 12000" },
                        "scope": { "type": "string", "description": "scope 名(见 .colna/scopes.json)" }
                    },
                    "required": ["task"]
                }
            },
            {
                "name": "kb_put",
                "description": "新建笔记。默认只返回预览;确认落盘需再调一次并传 confirm=true。默认写入 00-Inbox/。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string", "description": "标题" },
                        "body": { "type": "string", "description": "正文" },
                        "tags": { "type": "string", "description": "标签,逗号分隔" },
                        "path": { "type": "string", "description": "相对 memory/ 的路径;省略则用 00-Inbox/<slug>.md" },
                        "confirm": { "type": "boolean", "description": "false=只预览(默认),true=落盘" }
                    },
                    "required": ["title"]
                }
            },
            {
                "name": "kb_append",
                "description": "向已有笔记追加内容。默认只预览;expected_hash 用于防止覆盖并发修改。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "相对 memory/ 的路径" },
                        "body": { "type": "string", "description": "追加正文" },
                        "heading": { "type": "string", "description": "可选:追加到新二级标题下" },
                        "expected_hash": { "type": "string", "description": "当前文件 sha256;不匹配则拒绝" },
                        "confirm": { "type": "boolean", "description": "false=只预览(默认),true=落盘" }
                    },
                    "required": ["path", "body"]
                }
            },
            {
                "name": "kb_update",
                "description": "整篇替换笔记内容。必须带 expected_hash;默认只预览。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "相对 memory/ 的路径" },
                        "content": { "type": "string", "description": "新的完整文件内容(含 front-matter)" },
                        "expected_hash": { "type": "string", "description": "当前文件 sha256" },
                        "confirm": { "type": "boolean", "description": "false=只预览(默认),true=落盘" }
                    },
                    "required": ["path", "content", "expected_hash"]
                }
            },
            {
                "name": "kb_archive",
                "description": "归档笔记:移动到 40-Archive/,永不硬删。默认只预览。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "相对 memory/ 的路径" },
                        "expected_hash": { "type": "string", "description": "当前文件 sha256;可选" },
                        "confirm": { "type": "boolean", "description": "false=只预览(默认),true=移动" }
                    },
                    "required": ["path"]
                }
            },
            {
                "name": "kb_skill_search",
                "description": "在 ~/.agents/skills 的技能索引中检索(先运行 colna skills index)。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "技能检索语句" },
                        "topk": { "type": "integer", "description": "返回条数,默认 5" }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "kb_work_list",
                "description": "列出 memory/ 中所有未完成项(- [ ]),可按来源路径分组。",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "kb_task_start",
                "description": "创建任务 receipt。默认只预览;confirm=true 才落盘。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "goal": { "type": "string", "description": "任务目标" },
                        "project": { "type": "string", "description": "可选项目名,写入 10-Projects/<project>/tasks/" },
                        "plan": { "type": "string", "description": "可选初始计划" },
                        "confirm": { "type": "boolean", "description": "false=只预览(默认),true=落盘" }
                    },
                    "required": ["goal"]
                }
            },
            {
                "name": "kb_task_checkpoint",
                "description": "给任务追加一个 checkpoint。默认只预览。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "任务笔记路径" },
                        "note": { "type": "string", "description": "checkpoint 内容" },
                        "confirm": { "type": "boolean", "description": "false=只预览(默认),true=落盘" }
                    },
                    "required": ["path", "note"]
                }
            },
            {
                "name": "kb_task_finish",
                "description": "完成任务:更新 status,追加 Result / Evidence。默认只预览。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "任务笔记路径" },
                        "status": { "type": "string", "description": "done / blocked / open,默认 done" },
                        "outcome": { "type": "string", "description": "结果说明" },
                        "evidence": { "type": "string", "description": "可选证据/链接" },
                        "confirm": { "type": "boolean", "description": "false=只预览(默认),true=落盘" }
                    },
                    "required": ["path", "outcome"]
                }
            },
            {
                "name": "kb_task_list",
                "description": "列出任务 receipt,可按 status 过滤。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "status": { "type": "string", "description": "done / blocked / open" }
                    }
                }
            }
        ]
    })
}

fn handle_tool_call(root: &Path, emb: &mut Option<Embedder>, req: &Value) -> Result<Value> {
    let params = req.get("params").cloned().unwrap_or(json!({}));
    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    match name {
        "kb_search" => tool_kb_search(root, emb, &args),
        "kb_get" => tool_kb_get(root, &args),
        "kb_context" => tool_kb_context(root, emb, &args),
        "kb_put" => tool_kb_put(root, emb, &args),
        "kb_append" => tool_kb_append(root, emb, &args),
        "kb_update" => tool_kb_update(root, emb, &args),
        "kb_archive" => tool_kb_archive(root, emb, &args),
        "kb_skill_search" => tool_kb_skill_search(root, emb, &args),
        "kb_work_list" => tool_kb_work_list(root),
        "kb_task_start" => tool_kb_task_start(root, &args),
        "kb_task_checkpoint" => tool_kb_task_checkpoint(root, &args),
        "kb_task_finish" => tool_kb_task_finish(root, &args),
        "kb_task_list" => tool_kb_task_list(root, &args),
        other => Ok(tool_error(&format!("未知工具: {other}"))),
    }
}

fn ensure_embedder(emb: &mut Option<Embedder>) -> Result<()> {
    if emb.is_none() {
        *emb = Some(Embedder::new_quiet()?);
    }
    Ok(())
}

fn reindex_after_write(root: &Path, emb: &mut Option<Embedder>) -> Result<bool> {
    ensure_embedder(emb)?;
    indexer::ensure(root, false, true, emb.as_mut())
}

fn tool_kb_search(root: &Path, emb: &mut Option<Embedder>, args: &Value) -> Result<Value> {
    let query = match args.get("query").and_then(|q| q.as_str()) {
        Some(query) if !query.trim().is_empty() => query,
        _ => return Ok(tool_error("缺少参数 query")),
    };
    let topk = args.get("topk").and_then(|t| t.as_u64()).unwrap_or(5) as usize;
    let semantic_only = args
        .get("semantic_only")
        .and_then(|b| b.as_bool())
        .unwrap_or(false);
    let ensure_index = args
        .get("ensure_index")
        .and_then(|b| b.as_bool())
        .unwrap_or(true);
    let mmr = args.get("mmr").and_then(|b| b.as_bool()).unwrap_or(true);
    let include_summary = args
        .get("include_summary")
        .and_then(|b| b.as_bool())
        .unwrap_or(true);
    let expand = args
        .get("expand")
        .and_then(|b| b.as_bool())
        .unwrap_or(false);
    let rerank = args
        .get("rerank")
        .and_then(|b| b.as_bool())
        .unwrap_or(false);
    let recency_weight = args
        .get("recency_weight")
        .and_then(|n| n.as_f64())
        .map(|n| n as f32)
        .unwrap_or(0.15);
    let link_boost = args
        .get("link_boost")
        .and_then(|n| n.as_f64())
        .map(|n| n as f32)
        .unwrap_or(0.05);
    let scope = scope::resolve(root, args.get("scope").and_then(|value| value.as_str()))?;

    let mut path = args
        .get("path")
        .and_then(|p| p.as_str())
        .map(str::to_string);
    if path.is_none() {
        if let Some(kind) = args.get("type").and_then(|t| t.as_str()) {
            path = match util::type_prefix(kind) {
                Some(prefix) => Some(prefix.to_string()),
                None => return Ok(tool_error(&format!("未知类型: {kind}"))),
            };
        }
    }
    let since = args
        .get("since")
        .and_then(|s| s.as_str())
        .map(str::to_string);
    let until = args
        .get("until")
        .and_then(|s| s.as_str())
        .map(str::to_string);
    for (label, date) in [("since", &since), ("until", &until)] {
        if let Some(date) = date {
            if util::parse_date(date).is_none() {
                return Ok(tool_error(&format!("{label} 必须是 YYYY-MM-DD: {date}")));
            }
        }
    }

    ensure_embedder(emb)?;
    if ensure_index {
        indexer::ensure(root, false, true, emb.as_mut())?;
    }
    let link_counts = if link_boost > 0.0 {
        Some(graph::in_degree(root)?)
    } else {
        None
    };
    let queries = if expand {
        search::expand_queries(query)
    } else {
        vec![query.to_string()]
    };
    let collection = store::open(&root.join(indexer::INDEX_PATH).to_string_lossy())?;
    let mut hits = search::query_expanded(
        &collection,
        emb.as_mut().expect("embedder 已初始化"),
        &queries,
        &SearchOptions {
            topk: if rerank { topk * 2 } else { topk },
            hybrid: !semantic_only,
            filters: Filters {
                tags: parse_string_list(args.get("tags")),
                path,
                since,
                until,
                scope,
            },
            recency_weight,
            mmr,
            include_summary,
            link_boost,
        },
        link_counts.as_ref(),
    )?;
    if rerank {
        let model = rerank::Reranker::new()?;
        let documents: Vec<String> = hits.iter().map(|hit| hit.text.clone()).collect();
        let ordered = model.rerank(query, &documents, topk)?;
        let mut reranked = Vec::new();
        for (index, score) in ordered {
            if let Some(mut hit) = hits.get(index).cloned() {
                hit.score = score;
                reranked.push(hit);
            }
        }
        hits = reranked;
    }
    collection.close().ok();

    let mut text = format!("查询: {query}(共 {} 条)\n\n", hits.len());
    for (index, hit) in hits.iter().enumerate() {
        let location = if hit.breadcrumb.is_empty() {
            hit.title.clone()
        } else {
            format!("{} › {}", hit.title, hit.breadcrumb)
        };
        text.push_str(&format!(
            "#{} [score {:.4}] {} ({})\n{}\n\n",
            index + 1,
            hit.score,
            location,
            hit.source_path,
            hit.text.trim()
        ));
    }

    let results: Vec<Value> = hits
        .iter()
        .map(|hit| {
            json!({
                "source_path": hit.source_path,
                "title": hit.title,
                "heading": hit.heading,
                "breadcrumb": hit.breadcrumb,
                "kind": hit.kind,
                "tags": hit.tags,
                "date": hit.date,
                "score": hit.score,
                "score_detail": {
                    "vector_score": hit.vector_score,
                    "fts_score": hit.fts_score,
                    "recency_boost": hit.recency_boost,
                    "mmr_penalty": hit.mmr_penalty,
                    "link_boost": hit.link_boost,
                },
                "snippet": snippet(&hit.text, 240),
                "text": hit.text,
            })
        })
        .collect();

    Ok(json!({
        "content": [ { "type": "text", "text": text } ],
        "structuredContent": { "results": results },
        "isError": false
    }))
}

fn tool_kb_get(root: &Path, args: &Value) -> Result<Value> {
    let source_path = match args.get("source_path").and_then(|p| p.as_str()) {
        Some(path) if !path.trim().is_empty() => path,
        _ => return Ok(tool_error("缺少参数 source_path")),
    };

    if source_path.contains("..") || Path::new(source_path).is_absolute() {
        return Ok(tool_error("非法路径"));
    }
    let full: PathBuf = root.join(MEMORY_DIR).join(source_path);
    match std::fs::read_to_string(&full) {
        Ok(content) => Ok(json!({
            "content": [ { "type": "text", "text": content } ],
            "isError": false
        })),
        Err(error) => Ok(tool_error(&format!("读取失败 {source_path}: {error}"))),
    }
}

fn tool_kb_context(root: &Path, emb: &mut Option<Embedder>, args: &Value) -> Result<Value> {
    let task = match args.get("task").and_then(|t| t.as_str()) {
        Some(task) if !task.trim().is_empty() => task,
        _ => return Ok(tool_error("缺少参数 task")),
    };
    let topk = args.get("topk").and_then(|t| t.as_u64()).unwrap_or(8) as usize;
    let max_chars = args
        .get("max_chars")
        .and_then(|n| n.as_u64())
        .unwrap_or(12_000) as usize;
    let scope = scope::resolve(root, args.get("scope").and_then(|value| value.as_str()))?;

    ensure_embedder(emb)?;
    let text = context::build_with_embedder(
        root,
        task,
        topk,
        max_chars,
        emb.as_mut().expect("embedder 已初始化"),
        scope,
    )?;

    Ok(json!({
        "content": [ { "type": "text", "text": text } ],
        "structuredContent": { "task": task, "chars": text.chars().count() },
        "isError": false
    }))
}

fn tool_kb_put(root: &Path, emb: &mut Option<Embedder>, args: &Value) -> Result<Value> {
    let title = match args.get("title").and_then(|t| t.as_str()) {
        Some(title) if !title.trim().is_empty() => title,
        _ => return Ok(tool_error("缺少参数 title")),
    };
    let body = args.get("body").and_then(|b| b.as_str()).unwrap_or("");
    let tags = args.get("tags").and_then(|t| t.as_str()).unwrap_or("");
    let path = args.get("path").and_then(|p| p.as_str());
    let confirm = args
        .get("confirm")
        .and_then(|c| c.as_bool())
        .unwrap_or(false);

    let result = write::put(root, title, body, tags, path, confirm)?;
    let reindexed = if result.applied {
        reindex_after_write(root, emb)?
    } else {
        false
    };
    Ok(note_result(&result, reindexed))
}

fn tool_kb_append(root: &Path, emb: &mut Option<Embedder>, args: &Value) -> Result<Value> {
    let path = match args.get("path").and_then(|p| p.as_str()) {
        Some(path) if !path.trim().is_empty() => path,
        _ => return Ok(tool_error("缺少参数 path")),
    };
    let body = match args.get("body").and_then(|b| b.as_str()) {
        Some(body) if !body.trim().is_empty() => body,
        _ => return Ok(tool_error("缺少参数 body")),
    };
    let heading = args.get("heading").and_then(|h| h.as_str());
    let expected_hash = args.get("expected_hash").and_then(|h| h.as_str());
    let confirm = args
        .get("confirm")
        .and_then(|c| c.as_bool())
        .unwrap_or(false);

    let result = write::append(root, path, body, heading, expected_hash, confirm)?;
    let reindexed = if result.applied {
        reindex_after_write(root, emb)?
    } else {
        false
    };
    Ok(note_result(&result, reindexed))
}

fn tool_kb_update(root: &Path, emb: &mut Option<Embedder>, args: &Value) -> Result<Value> {
    let path = match args.get("path").and_then(|p| p.as_str()) {
        Some(path) if !path.trim().is_empty() => path,
        _ => return Ok(tool_error("缺少参数 path")),
    };
    let content = match args.get("content").and_then(|c| c.as_str()) {
        Some(content) => content,
        _ => return Ok(tool_error("缺少参数 content")),
    };
    let expected_hash = match args.get("expected_hash").and_then(|h| h.as_str()) {
        Some(hash) if !hash.trim().is_empty() => hash,
        _ => return Ok(tool_error("缺少参数 expected_hash")),
    };
    let confirm = args
        .get("confirm")
        .and_then(|c| c.as_bool())
        .unwrap_or(false);

    let result = write::update(root, path, content, expected_hash, confirm)?;
    let reindexed = if result.applied {
        reindex_after_write(root, emb)?
    } else {
        false
    };
    Ok(note_result(&result, reindexed))
}

fn tool_kb_archive(root: &Path, emb: &mut Option<Embedder>, args: &Value) -> Result<Value> {
    let path = match args.get("path").and_then(|p| p.as_str()) {
        Some(path) if !path.trim().is_empty() => path,
        _ => return Ok(tool_error("缺少参数 path")),
    };
    let expected_hash = args.get("expected_hash").and_then(|h| h.as_str());
    let confirm = args
        .get("confirm")
        .and_then(|c| c.as_bool())
        .unwrap_or(false);

    let result = write::archive(root, path, expected_hash, confirm)?;
    let reindexed = if result.applied {
        reindex_after_write(root, emb)?
    } else {
        false
    };
    let text = if result.applied {
        format!("已归档: {} → {}", result.from, result.to)
    } else {
        format!(
            "预览归档: {} → {} (confirm=true 才会移动)",
            result.from, result.to
        )
    };
    Ok(json!({
        "content": [ { "type": "text", "text": text } ],
        "structuredContent": {
            "applied": result.applied,
            "from": result.from,
            "to": result.to,
            "reindexed": reindexed,
        },
        "isError": false
    }))
}

fn tool_kb_skill_search(root: &Path, emb: &mut Option<Embedder>, args: &Value) -> Result<Value> {
    let query = match args.get("query").and_then(|q| q.as_str()) {
        Some(query) if !query.trim().is_empty() => query,
        _ => return Ok(tool_error("缺少参数 query")),
    };
    let topk = args.get("topk").and_then(|t| t.as_u64()).unwrap_or(5) as usize;
    skills::ensure_index_exists(root)?;
    ensure_embedder(emb)?;
    let hits = skills::search(root, query, topk, emb.as_mut().expect("embedder 已初始化"))?;

    let mut text = format!("技能查询: {query}(共 {} 条)\n\n", hits.len());
    for (index, hit) in hits.iter().enumerate() {
        let location = if hit.breadcrumb.is_empty() {
            hit.title.clone()
        } else {
            format!("{} › {}", hit.title, hit.breadcrumb)
        };
        text.push_str(&format!(
            "#{} [score {:.4}] {} ({})\n{}\n\n",
            index + 1,
            hit.score,
            location,
            hit.source_path,
            hit.text.trim()
        ));
    }
    let results: Vec<Value> = hits
        .iter()
        .map(|hit| {
            json!({
                "source_path": hit.source_path,
                "title": hit.title,
                "breadcrumb": hit.breadcrumb,
                "score": hit.score,
                "snippet": snippet(&hit.text, 240),
                "text": hit.text,
            })
        })
        .collect();
    Ok(json!({
        "content": [ { "type": "text", "text": text } ],
        "structuredContent": { "results": results },
        "isError": false
    }))
}

fn tool_kb_work_list(root: &Path) -> Result<Value> {
    let items = digest::list_open_work(root)?;
    let mut text = format!("未完成项:{} 条\n\n", items.len());
    for item in &items {
        text.push_str(&format!("- {} — {}\n", item.path, item.text));
    }
    Ok(json!({
        "content": [ { "type": "text", "text": text } ],
        "structuredContent": { "items": items },
        "isError": false
    }))
}

fn tool_kb_task_start(root: &Path, args: &Value) -> Result<Value> {
    let goal = match args.get("goal").and_then(|value| value.as_str()) {
        Some(goal) if !goal.trim().is_empty() => goal,
        _ => return Ok(tool_error("缺少参数 goal")),
    };
    let project = args.get("project").and_then(|value| value.as_str());
    let plan = args.get("plan").and_then(|value| value.as_str());
    let confirm = args
        .get("confirm")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let preview = task::start(root, goal, project, plan, confirm)?;
    Ok(note_result(&preview, preview.applied))
}

fn tool_kb_task_checkpoint(root: &Path, args: &Value) -> Result<Value> {
    let path = match args.get("path").and_then(|value| value.as_str()) {
        Some(path) if !path.trim().is_empty() => path,
        _ => return Ok(tool_error("缺少参数 path")),
    };
    let note = match args.get("note").and_then(|value| value.as_str()) {
        Some(note) if !note.trim().is_empty() => note,
        _ => return Ok(tool_error("缺少参数 note")),
    };
    let confirm = args
        .get("confirm")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let preview = task::checkpoint(root, path, note, confirm)?;
    Ok(note_result(&preview, preview.applied))
}

fn tool_kb_task_finish(root: &Path, args: &Value) -> Result<Value> {
    let path = match args.get("path").and_then(|value| value.as_str()) {
        Some(path) if !path.trim().is_empty() => path,
        _ => return Ok(tool_error("缺少参数 path")),
    };
    let status = args
        .get("status")
        .and_then(|value| value.as_str())
        .unwrap_or("done");
    let outcome = match args.get("outcome").and_then(|value| value.as_str()) {
        Some(outcome) if !outcome.trim().is_empty() => outcome,
        _ => return Ok(tool_error("缺少参数 outcome")),
    };
    let evidence = args.get("evidence").and_then(|value| value.as_str());
    let confirm = args
        .get("confirm")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let preview = task::finish(root, path, status, outcome, evidence, confirm)?;
    Ok(note_result(&preview, preview.applied))
}

fn tool_kb_task_list(root: &Path, args: &Value) -> Result<Value> {
    let status = args.get("status").and_then(|value| value.as_str());
    let tasks = task::list(root, status)?;
    let mut text = format!("任务:{} 条\n\n", tasks.len());
    for item in &tasks {
        text.push_str(&format!(
            "[{}] {}  {}  ({})\n",
            item.status, item.date, item.title, item.path
        ));
    }
    let items: Vec<Value> = tasks
        .iter()
        .map(|item| {
            json!({
                "path": item.path,
                "title": item.title,
                "status": item.status,
                "project": item.project,
                "goal": item.goal,
                "date": item.date,
            })
        })
        .collect();
    Ok(json!({
        "content": [ { "type": "text", "text": text } ],
        "structuredContent": { "tasks": items },
        "isError": false
    }))
}

fn note_result(result: &write::NotePreview, reindexed: bool) -> Value {
    let text = if result.applied {
        format!("已写入: {} (hash {})", result.path, result.hash)
    } else {
        format!(
            "预览(未写入): {} (hash {})\n\n{}",
            result.path, result.hash, result.diff
        )
    };
    json!({
        "content": [ { "type": "text", "text": text } ],
        "structuredContent": {
            "applied": result.applied,
            "path": result.path,
            "hash": result.hash,
            "diff": result.diff,
            "content": result.content,
            "reindexed": reindexed,
        },
        "isError": false
    })
}

fn parse_string_list(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.as_str())
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(str::to_string)
            .collect(),
        Some(Value::String(text)) => text
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

fn snippet(text: &str, max_chars: usize) -> String {
    let text = text.replace('\n', " ");
    if text.chars().count() <= max_chars {
        text
    } else {
        let mut out: String = text.chars().take(max_chars).collect();
        out.push('…');
        out
    }
}

// ---- JSON-RPC 信封辅助 ----

fn make_result(id: Option<Value>, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id.unwrap_or(Value::Null), "result": result })
}

fn make_error(id: Option<Value>, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id.unwrap_or(Value::Null), "error": { "code": code, "message": message } })
}

/// tools/call 里的“工具级错误”:仍是成功的 JSON-RPC result,但 isError=true。
fn tool_error(message: &str) -> Value {
    json!({
        "content": [ { "type": "text", "text": message } ],
        "isError": true
    })
}

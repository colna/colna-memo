//! MCP server(stdio / JSON-RPC 2.0):让 Claude 直接调用知识库。
//!
//! 暴露两个工具:
//!   - `kb_search`:语义 / 混合检索 memory/ 知识库
//!   - `kb_get`:按相对路径取回某个 Markdown 文件的全文
//!
//! 走 MCP 的 stdio transport:每行一条 JSON-RPC 消息(换行分隔),
//! stdout 只输出协议消息,日志一律走 stderr。
//! 实现刻意只依赖 serde_json,不引入 tokio / rmcp,避免协议库 API 漂移。

use crate::embedder::Embedder;
use crate::store;
use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

const PROTOCOL_VERSION: &str = "2024-11-05";
const MEMORY_DIR: &str = "memory";
const INDEX_PATH: &str = ".colna/index.zvec";

/// 运行 MCP server,直到 stdin 关闭。
pub fn serve(root: &Path) -> Result<()> {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    let mut emb: Option<Embedder> = None;

    eprintln!("colna mcp: 已启动,root = {}", root.display());

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let req: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("colna mcp: 跳过无法解析的行: {e}");
                continue;
            }
        };

        let id = req.get("id").cloned();
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");

        // 通知(没有 id)不需要回复
        let is_notification = id.is_none();

        let response = match method {
            "initialize" => Some(make_result(id.clone(), initialize_result(&req))),
            "tools/list" => Some(make_result(id.clone(), tools_list())),
            "tools/call" => {
                let res = handle_tool_call(root, &mut emb, &req);
                Some(match res {
                    Ok(v) => make_result(id.clone(), v),
                    Err(e) => make_result(id.clone(), tool_error(&e.to_string())),
                })
            }
            "ping" => Some(make_result(id.clone(), json!({}))),
            "notifications/initialized" | "notifications/cancelled" => None,
            _ if is_notification => None,
            _ => Some(make_error(id.clone(), -32601, &format!("未知方法: {method}"))),
        };

        if let Some(resp) = response {
            let s = serde_json::to_string(&resp)?;
            writeln!(out, "{s}")?;
            out.flush()?;
        }
    }
    Ok(())
}

fn initialize_result(req: &Value) -> Value {
    // 回显客户端请求的协议版本,缺省用我们支持的版本
    let version = req
        .get("params")
        .and_then(|p| p.get("protocolVersion"))
        .and_then(|v| v.as_str())
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
                        "semantic_only": { "type": "boolean", "description": "true=只用向量语义检索;默认 false=向量+FTS 混合" }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "kb_get",
                "description": "按相对路径(相对 memory/,如 notes/welcome.md)取回某个 Markdown 文件的全文。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "source_path": { "type": "string", "description": "相对 memory/ 的文件路径" }
                    },
                    "required": ["source_path"]
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
        other => Ok(tool_error(&format!("未知工具: {other}"))),
    }
}

fn tool_kb_search(root: &Path, emb: &mut Option<Embedder>, args: &Value) -> Result<Value> {
    let query = match args.get("query").and_then(|q| q.as_str()) {
        Some(q) if !q.trim().is_empty() => q,
        _ => return Ok(tool_error("缺少参数 query")),
    };
    let topk = args.get("topk").and_then(|t| t.as_u64()).unwrap_or(5) as usize;
    let semantic_only = args
        .get("semantic_only")
        .and_then(|b| b.as_bool())
        .unwrap_or(false);

    if emb.is_none() {
        *emb = Some(Embedder::new_quiet()?);
    }
    let embedder = emb.as_mut().unwrap();
    let qv = embedder.embed_query(query)?;

    let index_path = root.join(INDEX_PATH);
    let collection = store::open(&index_path.to_string_lossy())?;
    let hits = if semantic_only {
        store::vector_search(&collection, &qv, topk)?
    } else {
        store::hybrid_search(&collection, &qv, query, topk)?
    };
    collection.close().ok();

    // 文本视图(给人/模型读)
    let mut text = format!("查询: {query}(共 {} 条)\n\n", hits.len());
    for (i, h) in hits.iter().enumerate() {
        let loc = if h.heading.is_empty() {
            h.title.clone()
        } else {
            format!("{} › {}", h.title, h.heading)
        };
        text.push_str(&format!(
            "#{} [{}] {} (score {:.4})\n{}\n\n",
            i + 1,
            h.source_path,
            loc,
            h.score,
            h.text.trim()
        ));
    }

    // 结构化视图(structuredContent)
    let results: Vec<Value> = hits
        .iter()
        .map(|h| {
            json!({
                "source_path": h.source_path,
                "title": h.title,
                "heading": h.heading,
                "tags": h.tags,
                "date": h.date,
                "score": h.score,
                "text": h.text,
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
        Some(p) if !p.trim().is_empty() => p,
        _ => return Ok(tool_error("缺少参数 source_path")),
    };

    // 防目录穿越:拒绝 .. 与绝对路径
    if source_path.contains("..") || Path::new(source_path).is_absolute() {
        return Ok(tool_error("非法路径"));
    }
    let full: PathBuf = root.join(MEMORY_DIR).join(source_path);
    match std::fs::read_to_string(&full) {
        Ok(content) => Ok(json!({
            "content": [ { "type": "text", "text": content } ],
            "isError": false
        })),
        Err(e) => Ok(tool_error(&format!("读取失败 {source_path}: {e}"))),
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

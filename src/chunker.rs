//! Markdown 切块:按标题分段,提取 front-matter 元数据。

use sha2::{Digest, Sha256};
use std::path::Path;

/// 一个可索引的内容块
#[derive(Debug, Clone)]
pub struct Chunk {
    pub id: String,
    pub source_path: String,
    pub title: String,
    pub heading: String,
    pub tags: String,
    pub date: String,
    /// 展示 / FTS / 存储用的原文(标题 + 正文)
    pub text: String,
    /// 仅用于生成向量的「上下文增强」文本:
    /// 文档标题 · 日期 · 标题面包屑 + 正文。让每个块自带文档/章节上下文,
    /// 大幅改善「碎小段落丢失归属」「大段稀释语义」的召回精准度。不落库、不展示。
    pub embed_text: String,
}

/// front-matter 解析结果
#[derive(Default)]
struct FrontMatter {
    title: Option<String>,
    tags: Option<String>,
    date: Option<String>,
}

/// 解析 YAML 风格 front-matter(--- ... ---),只取 title/tags/date 三个键。
/// 返回 (front_matter, 去掉 front-matter 后的正文)
fn parse_front_matter(content: &str) -> (FrontMatter, &str) {
    let mut fm = FrontMatter::default();
    let trimmed = content.trim_start_matches('\u{feff}'); // 去 BOM
    if !trimmed.starts_with("---") {
        return (fm, content);
    }
    // 找第二个 "---"
    let after_first = &trimmed[3..];
    if let Some(end) = after_first.find("\n---") {
        let block = &after_first[..end];
        for line in block.lines() {
            let line = line.trim();
            if let Some((k, v)) = line.split_once(':') {
                let key = k.trim().to_lowercase();
                let val = v.trim().trim_matches('"').trim().to_string();
                match key.as_str() {
                    "title" => fm.title = Some(val),
                    "tags" => fm.tags = Some(val),
                    "date" => fm.date = Some(val),
                    _ => {}
                }
            }
        }
        // 正文 = 第二个 --- 之后
        let rest_start = end + 4; // 跳过 "\n---"
        let body = &after_first[rest_start..];
        let body = body.strip_prefix('\n').unwrap_or(body);
        return (fm, body);
    }
    (fm, content)
}

/// 稳定 id:source_path + heading + 序号 的 sha256(取前 16 字节 hex)
fn make_id(source_path: &str, heading: &str, ordinal: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source_path.as_bytes());
    hasher.update(b"\x00");
    hasher.update(heading.as_bytes());
    hasher.update(b"\x00");
    hasher.update(ordinal.to_le_bytes());
    let digest = hasher.finalize();
    digest[..16].iter().map(|b| format!("{:02x}", b)).collect()
}

/// 把一个 Markdown 文件切成若干 Chunk。
/// source_path 为相对 memory/ 根的路径(用于展示与稳定 id)。
pub fn chunk_markdown(source_path: &str, content: &str) -> Vec<Chunk> {
    let (fm, body) = parse_front_matter(content);

    // 标题兜底:front-matter title → 第一个 H1 → 文件名
    let fallback_title = fm.title.clone().unwrap_or_else(|| {
        body.lines()
            .find(|l| l.trim_start().starts_with("# "))
            .map(|l| l.trim_start().trim_start_matches('#').trim().to_string())
            .unwrap_or_else(|| {
                Path::new(source_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(source_path)
                    .to_string()
            })
    });
    let tags = fm.tags.unwrap_or_default();
    let date = fm.date.unwrap_or_default();

    // 按标题行(#, ##, ###...)切段;同时维护标题层级栈,给每段算出「面包屑」路径。
    // section = (heading, breadcrumb, body)。breadcrumb 形如 "工作日志 › 沉淀"。
    let mut sections: Vec<(String, String, String)> = Vec::new();
    let mut cur_heading = String::new();
    let mut cur_crumb = String::new();
    let mut cur_body = String::new();
    let mut stack: Vec<(usize, String)> = Vec::new();

    for line in body.lines() {
        let ls = line.trim_start();
        if ls.starts_with('#') {
            // flush 上一段
            let b = cur_body.trim();
            if !b.is_empty() || !cur_heading.is_empty() {
                sections.push((cur_heading.clone(), cur_crumb.clone(), b.to_string()));
            }
            cur_body.clear();
            // 解析层级 + 标题文本
            let level = ls.chars().take_while(|c| *c == '#').count();
            let heading = ls.trim_start_matches('#').trim().to_string();
            // 弹出同级或更深的祖先,压入当前标题
            while matches!(stack.last(), Some((lv, _)) if *lv >= level) {
                stack.pop();
            }
            stack.push((level, heading.clone()));
            cur_heading = heading;
            cur_crumb = stack
                .iter()
                .map(|(_, h)| h.as_str())
                .collect::<Vec<_>>()
                .join(" › ");
        } else {
            cur_body.push_str(line);
            cur_body.push('\n');
        }
    }
    // flush 末段
    let b = cur_body.trim();
    if !b.is_empty() || !cur_heading.is_empty() {
        sections.push((cur_heading.clone(), cur_crumb.clone(), b.to_string()));
    }

    // 组装 Chunk;text = 标题 + 正文(展示/FTS);embed_text = 上下文 + 正文(仅嵌入),空段跳过
    let mut chunks = Vec::new();
    for (i, (heading, crumb, sec_body)) in sections.into_iter().enumerate() {
        let text = if heading.is_empty() {
            sec_body.clone()
        } else if sec_body.is_empty() {
            heading.clone()
        } else {
            format!("{}\n{}", heading, sec_body)
        };
        if text.trim().is_empty() {
            continue;
        }

        // 上下文前缀:文档标题 · 日期 · 面包屑(面包屑已含本级标题)
        let mut ctx: Vec<String> = Vec::new();
        if !fallback_title.is_empty() {
            ctx.push(fallback_title.clone());
        }
        if !date.is_empty() {
            ctx.push(date.clone());
        }
        let crumb_or_heading = if crumb.is_empty() { heading.clone() } else { crumb };
        if !crumb_or_heading.is_empty() {
            ctx.push(crumb_or_heading);
        }
        let prefix = ctx.join(" · ");
        let embed_text = match (prefix.is_empty(), sec_body.is_empty()) {
            (true, _) => text.clone(),
            (false, true) => prefix,
            (false, false) => format!("{}\n{}", prefix, sec_body),
        };

        chunks.push(Chunk {
            id: make_id(source_path, &heading, i),
            source_path: source_path.to_string(),
            title: fallback_title.clone(),
            heading,
            tags: tags.clone(),
            date: date.clone(),
            text,
            embed_text,
        });
    }
    chunks
}

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
    /// 标题面包屑(如 `工作日志 › 沉淀`),用于检索结果展示与过滤。
    pub breadcrumb: String,
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
#[derive(Default, Debug, Clone)]
pub struct FrontMatter {
    pub title: Option<String>,
    pub tags: Option<String>,
    pub date: Option<String>,
    pub extra: std::collections::BTreeMap<String, String>,
}

/// 解析 YAML 风格 front-matter(--- ... ---),只取 title/tags/date 三个键。
/// 返回 (front_matter, 去掉 front-matter 后的正文)
pub fn parse_front_matter(content: &str) -> (FrontMatter, &str) {
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
                    _ => {
                        fm.extra.insert(key, val);
                    }
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

    // 组装 Chunk:整篇摘要 + 按标题分段,超长段按 ~1200 字符切分(不切代码块)
    let mut chunks = Vec::new();

    if let Some(summary) = build_summary(body) {
        let text = format!("{fallback_title}\n\n{summary}");
        chunks.push(Chunk {
            id: make_id(source_path, "__summary__", 0),
            source_path: source_path.to_string(),
            title: fallback_title.clone(),
            heading: "__summary__".to_string(),
            breadcrumb: String::new(),
            tags: tags.clone(),
            date: date.clone(),
            text: text.clone(),
            embed_text: format!("{fallback_title} · 摘要\n{summary}"),
        });
    }

    let mut ordinal = 1usize;
    for (heading, crumb, sec_body) in sections.into_iter() {
        // 上下文前缀:文档标题 · 日期 · 面包屑(面包屑已含本级标题)
        let mut ctx: Vec<String> = Vec::new();
        if !fallback_title.is_empty() {
            ctx.push(fallback_title.clone());
        }
        if !date.is_empty() {
            ctx.push(date.clone());
        }
        let crumb_or_heading = if crumb.is_empty() {
            heading.clone()
        } else {
            crumb
        };
        if !crumb_or_heading.is_empty() {
            ctx.push(crumb_or_heading.clone());
        }
        let prefix = ctx.join(" · ");
        let breadcrumb = {
            let title_prefix = format!("{fallback_title} › ");
            if crumb_or_heading == fallback_title {
                String::new()
            } else {
                crumb_or_heading
                    .strip_prefix(&title_prefix)
                    .map(str::to_string)
                    .unwrap_or(crumb_or_heading)
            }
        };

        for part in split_section(&sec_body) {
            let text = if heading.is_empty() {
                part.clone()
            } else if part.is_empty() {
                heading.clone()
            } else {
                format!("{}\n{}", heading, part)
            };
            if text.trim().is_empty() {
                continue;
            }
            let embed_text = match (prefix.is_empty(), part.is_empty()) {
                (true, _) => text.clone(),
                (false, true) => prefix.clone(),
                (false, false) => format!("{}\n{}", prefix, part),
            };

            chunks.push(Chunk {
                id: make_id(source_path, &heading, ordinal),
                source_path: source_path.to_string(),
                title: fallback_title.clone(),
                heading: heading.clone(),
                breadcrumb: breadcrumb.clone(),
                tags: tags.clone(),
                date: date.clone(),
                text,
                embed_text,
            });
            ordinal += 1;
        }
    }

    chunks
}

const MAX_SECTION_CHARS: usize = 1200;
const SUMMARY_CHARS: usize = 600;

/// 按 ~1200 字符切分,尽量不切断代码块。
fn split_section(body: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_code = false;

    for line in body.lines() {
        let is_fence = line.trim_start().starts_with("```");
        let would_exceed = !in_code
            && !current.is_empty()
            && current.chars().count() + line.chars().count() + 1 > MAX_SECTION_CHARS;
        if would_exceed {
            parts.push(current.trim().to_string());
            current.clear();
        }
        current.push_str(line);
        current.push('\n');
        if is_fence {
            in_code = !in_code;
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    if parts.is_empty() {
        parts.push(String::new());
    }
    parts
}

/// 整篇摘要:标题列表 + 前 600 字符正文(跳过代码块)。
fn build_summary(body: &str) -> Option<String> {
    let mut headings = Vec::new();
    let mut prose = String::new();
    let mut in_code = false;

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        if trimmed.starts_with('#') {
            if headings.len() < 8 {
                headings.push(trimmed.trim_start_matches('#').trim().to_string());
            }
            continue;
        }
        if prose.chars().count() < SUMMARY_CHARS {
            prose.push_str(trimmed);
            prose.push(' ');
        }
    }

    let mut parts = Vec::new();
    if !headings.is_empty() {
        parts.push(headings.join(" / "));
    }
    let prose = prose.trim();
    if !prose.is_empty() {
        let mut clipped: String = prose.chars().take(SUMMARY_CHARS).collect();
        if prose.chars().count() > SUMMARY_CHARS {
            clipped.push('…');
        }
        parts.push(clipped);
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_heading_breadcrumb_for_nested_sections() {
        let content =
            "---\ntitle: Demo\ndate: 2026-10-30\n---\n# Top\n\nintro\n\n## Child\n\nbody\n";
        let chunks = chunk_markdown("demo.md", content);

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].heading, "__summary__");
        assert_ne!(chunks[1].heading, "__summary__");
        assert_eq!(chunks[1].breadcrumb, "Top");
        assert_eq!(chunks[2].breadcrumb, "Top › Child");
        assert_eq!(chunks[1].title, "Demo");
    }

    #[test]
    fn splits_long_sections_without_cutting_code_fences() {
        let body = format!("```\n{}\n```\n\n{}", "x".repeat(1500), "tail");
        let parts = split_section(&body);
        assert_eq!(parts.len(), 2);
        assert!(parts[0].contains("```"));
        assert!(parts[0].contains(&"x".repeat(1500)));
        assert_eq!(parts[1], "tail");
    }

    #[test]
    fn builds_document_summary() {
        let content = "---\ntitle: Demo\n---\n# Top\n\nfirst paragraph\n\n## Child\n\nsecond\n";
        let chunks = chunk_markdown("demo.md", content);
        let summary = chunks
            .iter()
            .find(|chunk| chunk.heading == "__summary__")
            .unwrap();
        assert!(summary.text.contains("Top"));
        assert!(summary.text.contains("first paragraph"));
        assert!(summary.text.contains("Child"));
    }

    #[test]
    fn parses_front_matter_keys() {
        let (fm, body) =
            parse_front_matter("---\ntitle: T\ntags: a, b\ndate: 2026-01-02\n---\n\nBody\n");

        assert_eq!(fm.title.as_deref(), Some("T"));
        assert_eq!(fm.tags.as_deref(), Some("a, b"));
        assert_eq!(fm.date.as_deref(), Some("2026-01-02"));
        assert_eq!(body.trim(), "Body");
    }
}

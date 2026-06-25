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
    pub text: String,
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

    // 按标题行(#, ##, ###...)切段
    let mut sections: Vec<(String, String)> = Vec::new();
    let mut cur_heading = String::new();
    let mut cur_body = String::new();
    let flush = |heading: &str, body: &mut String, out: &mut Vec<(String, String)>| {
        let b = body.trim();
        if !b.is_empty() || !heading.is_empty() {
            out.push((heading.to_string(), b.to_string()));
        }
        body.clear();
    };
    for line in body.lines() {
        if line.trim_start().starts_with('#') {
            flush(&cur_heading, &mut cur_body, &mut sections);
            cur_heading = line.trim_start().trim_start_matches('#').trim().to_string();
        } else {
            cur_body.push_str(line);
            cur_body.push('\n');
        }
    }
    flush(&cur_heading, &mut cur_body, &mut sections);

    // 组装 Chunk;text = 标题 + 正文,空段跳过
    let mut chunks = Vec::new();
    for (i, (heading, sec_body)) in sections.into_iter().enumerate() {
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
        chunks.push(Chunk {
            id: make_id(source_path, &heading, i),
            source_path: source_path.to_string(),
            title: fallback_title.clone(),
            heading,
            tags: tags.clone(),
            date: date.clone(),
            text,
        });
    }
    chunks
}

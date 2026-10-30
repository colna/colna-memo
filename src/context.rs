//! `colna context <task>`:把完成任务所需的上下文压成一个 Markdown 包。
//!
//! 内容:书写约定 + 今日重点 + 最近工作日志 + 相关笔记 + 排错经验。
//! 不做执行,只负责“给 agent 一份可注入的背景”。

use crate::chunker;
use crate::embedder::Embedder;
use crate::indexer;
use crate::search::{self, Filters, SearchOptions};
use crate::store;
use crate::util;
use anyhow::Result;
use std::path::Path;

pub fn build_with_embedder(
    root: &Path,
    task: &str,
    topk: usize,
    max_chars: usize,
    embedder: &mut Embedder,
    scope: Option<String>,
) -> Result<String> {
    indexer::ensure(root, false, true, Some(embedder))?;

    let query_vec = embedder.embed_query(task)?;
    let collection = store::open(&root.join(indexer::INDEX_PATH).to_string_lossy())?;
    let hits = search::query(
        &collection,
        &query_vec,
        task,
        &SearchOptions {
            topk: topk.max(1) * 2,
            hybrid: true,
            filters: Filters {
                scope,
                ..Filters::default()
            },
            recency_weight: 0.15,
            mmr: true,
            include_summary: true,
            link_boost: 0.05,
        },
        None,
    )?;
    collection.close().ok();

    let max_chars = max_chars.max(2_000);
    let mut out = String::new();
    out.push_str(&format!("# 上下文包:{task}\n\n"));
    out.push_str("> 由 colna-memo 生成。按需引用,不要整段复述;涉及事实时优先核对来源路径。\n\n");

    if let Some(conventions) = read_optional(root, "CONVENTIONS.md") {
        let (_, body) = chunker::parse_front_matter(&conventions);
        append(
            &mut out,
            "## 书写约定\n\n",
            &truncate_chars(body, 3_000),
            max_chars,
        );
    }

    let today = util::today_beijing();
    if let Some(daily) = read_optional(root, &format!("50-Daily/{today}.md")) {
        let todos = extract_todos(&daily);
        if !todos.is_empty() {
            append(&mut out, "## 今日重点\n\n", &todos, max_chars);
        }
        let recent = extract_recent_worklogs(&daily, 2);
        if !recent.is_empty() {
            append(&mut out, "## 最近工作日志\n\n", &recent, max_chars);
        }
    }

    let today_path = format!("50-Daily/{today}.md");
    let max_score = hits
        .iter()
        .map(|hit| hit.score)
        .fold(f32::MIN, f32::max)
        .max(0.0);
    let min_score = max_score * 0.2;
    let relevant: Vec<_> = hits.iter().filter(|hit| hit.score >= min_score).collect();
    let troubleshooting: Vec<_> = relevant
        .iter()
        .copied()
        .filter(|hit| hit.source_path.starts_with("30-Resources/troubleshooting/"))
        .collect();
    let others: Vec<_> = relevant
        .into_iter()
        .filter(|hit| {
            !hit.source_path.starts_with("30-Resources/troubleshooting/")
                && hit.source_path != today_path
                && hit.source_path != "CONVENTIONS.md"
        })
        .collect();

    if !others.is_empty() {
        let mut section = String::new();
        for hit in others {
            let location = if hit.breadcrumb.is_empty() {
                hit.title.clone()
            } else {
                format!("{} › {}", hit.title, hit.breadcrumb)
            };
            section.push_str(&format!(
                "### {}\n\n- 路径: `{}`\n- 日期/标签: {} {}\n\n{}\n\n",
                location,
                hit.source_path,
                if hit.date.is_empty() {
                    "—"
                } else {
                    &hit.date
                },
                if hit.tags.is_empty() { "" } else { &hit.tags },
                truncate_chars(&hit.text, 900)
            ));
        }
        append(&mut out, "## 相关笔记\n\n", &section, max_chars);
    }

    if !troubleshooting.is_empty() {
        let mut section = String::new();
        for hit in troubleshooting {
            let location = if hit.breadcrumb.is_empty() {
                hit.title.clone()
            } else {
                format!("{} › {}", hit.title, hit.breadcrumb)
            };
            section.push_str(&format!(
                "### {}\n\n- 路径: `{}`\n\n{}\n\n",
                location,
                hit.source_path,
                truncate_chars(&hit.text, 900)
            ));
        }
        append(&mut out, "## 排错经验\n\n", &section, max_chars);
    }

    if out.len() >= max_chars {
        out.push_str("\n> [上下文包已截断]\n");
    }
    Ok(out)
}

fn read_optional(root: &Path, rel: &str) -> Option<String> {
    let path = root.join(indexer::MEMORY_DIR).join(rel);
    std::fs::read_to_string(path).ok()
}

fn append(out: &mut String, heading: &str, body: &str, max_chars: usize) {
    if out.len() >= max_chars || body.trim().is_empty() {
        return;
    }
    let remaining = max_chars.saturating_sub(out.len());
    let block = format!("{heading}{body}\n\n");
    out.push_str(&truncate_chars(&block, remaining));
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut out: String = value.chars().take(max_chars).collect();
    out.push('…');
    out
}

fn extract_todos(daily: &str) -> String {
    let mut in_focus = false;
    let mut todos = Vec::new();
    for line in daily.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            in_focus = trimmed == "## 今日重点";
            continue;
        }
        if in_focus && (trimmed.starts_with("- [ ]") || trimmed.starts_with("- [x]")) {
            todos.push(trimmed.to_string());
        }
    }
    if todos.is_empty() {
        // 没有「今日重点」section 时,退回全篇未完成项
        for line in daily.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("- [ ]") {
                todos.push(trimmed.to_string());
            }
        }
    }
    todos.join("\n")
}

fn extract_recent_worklogs(daily: &str, count: usize) -> String {
    let mut sections: Vec<String> = Vec::new();
    let mut current: Option<String> = None;
    for line in daily.lines() {
        if line.trim_start().starts_with("## 工作日志") {
            if let Some(section) = current.take() {
                sections.push(section);
            }
            current = Some(format!("{line}\n"));
        } else if let Some(section) = current.as_mut() {
            section.push_str(line);
            section.push('\n');
        }
    }
    if let Some(section) = current {
        sections.push(section);
    }
    sections
        .into_iter()
        .rev()
        .take(count)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|section| truncate_chars(section.trim(), 1_500))
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_todos_from_focus_section() {
        let daily = "# D\n\n## 今日重点\n\n- [ ] a\n- [x] b\n\n## 工作日志\n\n- [ ] c\n";
        assert_eq!(extract_todos(daily), "- [ ] a\n- [x] b");
    }

    #[test]
    fn extracts_last_worklogs_in_order() {
        let daily = "## 工作日志 — 1\n\na\n\n## 工作日志 — 2\n\nb\n";
        let recent = extract_recent_worklogs(daily, 1);
        assert!(recent.contains("工作日志 — 2"));
        assert!(!recent.contains("工作日志 — 1"));
    }

    #[test]
    fn truncates_on_char_boundary() {
        let text = "你好世界";
        assert_eq!(truncate_chars(text, 2), "你好…");
        assert_eq!(truncate_chars(text, 10), text);
    }
}

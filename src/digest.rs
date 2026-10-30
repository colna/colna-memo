//! Daily 整理:digest / consolidate / open work。
//!
//! 只做确定性的提取与安全的写入(待办转抄、digest 笔记、open-work 清单);
//! 是否把内容合并进项目/排错笔记仍由人或上层 agent 判断。

use crate::chunker;
use crate::embedder::Embedder;
use crate::graph;
use crate::indexer;
use crate::search::{self, Filters, SearchOptions};
use crate::store;
use crate::util;
use crate::write;
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Default, Clone, Serialize)]
pub struct Report {
    pub date: String,
    pub todos: Vec<String>,
    pub done: Vec<String>,
    pub worklogs: Vec<(String, String)>,
    pub lessons: Vec<String>,
    pub links: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenItem {
    pub path: String,
    pub text: String,
}

pub fn analyze(root: &Path, date: &str) -> Result<Report> {
    let rel = format!("50-Daily/{date}.md");
    let content =
        write::read_note(root, &rel).with_context(|| format!("读取 Daily 失败: {rel}"))?;
    let (_, body) = chunker::parse_front_matter(&content);

    let mut report = Report {
        date: date.to_string(),
        ..Report::default()
    };
    let mut current_worklog: Option<(String, String)> = None;

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## 工作日志") {
            if let Some(worklog) = current_worklog.take() {
                if !worklog.1.trim().is_empty() {
                    report.worklogs.push(worklog);
                }
            }
            current_worklog = Some((
                trimmed.trim_start_matches('#').trim().to_string(),
                String::new(),
            ));
            continue;
        }
        if trimmed.starts_with("## ") {
            if let Some(worklog) = current_worklog.take() {
                if !worklog.1.trim().is_empty() {
                    report.worklogs.push(worklog);
                }
            }
            continue;
        }
        if let Some((_, body)) = current_worklog.as_mut() {
            if !trimmed.is_empty() && body.lines().count() < 6 {
                body.push_str(trimmed);
                body.push('\n');
            }
        }
        if trimmed.starts_with("- [ ]") {
            report.todos.push(trimmed.to_string());
        } else if trimmed.starts_with("- [x]") || trimmed.starts_with("- [X]") {
            report.done.push(trimmed.to_string());
        }
        if is_lesson(trimmed) {
            report.lessons.push(trimmed.to_string());
        }
    }
    if let Some(worklog) = current_worklog {
        if !worklog.1.trim().is_empty() {
            report.worklogs.push(worklog);
        }
    }

    report.todos = dedupe(report.todos);
    report.done = dedupe(report.done);
    report.lessons = dedupe(report.lessons);
    report.links = dedupe(graph::extract_targets(body));
    Ok(report)
}

fn is_lesson(line: &str) -> bool {
    if line.is_empty() || line.starts_with('#') {
        return false;
    }
    [
        "根因", "修法", "结论", "沉淀", "教训", "决定", "方案", "坑", "风险",
    ]
    .iter()
    .any(|keyword| line.contains(keyword))
}

fn dedupe(items: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    items
        .into_iter()
        .filter(|item| seen.insert(normalize(item)))
        .collect()
}

fn normalize(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn render(report: &Report) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Digest {}\n\n", report.date));
    out.push_str("> 自动生成,供整理用;合并进项目/排错笔记前请人工判断。\n\n");

    out.push_str("## 今日重点\n\n");
    if report.todos.is_empty() {
        out.push_str("- （无未完成项）\n");
    } else {
        for todo in &report.todos {
            out.push_str(&format!("{todo}\n"));
        }
    }
    out.push('\n');

    if !report.done.is_empty() {
        out.push_str("## 已完成\n\n");
        for item in &report.done {
            out.push_str(&format!("{item}\n"));
        }
        out.push('\n');
    }

    if !report.worklogs.is_empty() {
        out.push_str("## 工作日志摘要\n\n");
        for (heading, body) in &report.worklogs {
            out.push_str(&format!("### {heading}\n\n"));
            if body.trim().is_empty() {
                out.push_str("（无正文）\n\n");
            } else {
                out.push_str(body.trim());
                out.push_str("\n\n");
            }
        }
    }

    if !report.lessons.is_empty() {
        out.push_str("## 可沉淀经验\n\n");
        for lesson in &report.lessons {
            out.push_str(&format!("{lesson}\n"));
        }
        out.push('\n');
    }

    if !report.links.is_empty() {
        out.push_str("## 相关链接\n\n");
        for link in &report.links {
            out.push_str(&format!("- {link}\n"));
        }
        out.push('\n');
    }

    out.push_str("## 建议动作\n\n");
    out.push_str("- 未完成项用 `colna consolidate --apply` 转抄到今日 Daily;\n");
    out.push_str("- 可沉淀经验检查是否已在 `30-Resources/troubleshooting/`;\n");
    out.push_str("- 相关链接可用 `colna related <path>` 查 backlinks。\n");
    out
}

pub fn write_digest(root: &Path, report: &Report) -> Result<std::path::PathBuf> {
    let title = format!("Digest {}", report.date);
    let rel = format!("00-Inbox/Digest-{}.md", report.date);
    let content = format!(
        "---\ntitle: {}\ndate: {}\ntags: digest\nsource_date: {}\n---\n\n{}",
        util::yaml_quote(&title),
        util::today_beijing(),
        report.date,
        render(report)
    );
    write::replace_file(root, &rel, &content)?;
    indexer::ensure(root, false, false, None)?;
    Ok(root.join(indexer::MEMORY_DIR).join(rel))
}

pub fn carry_over_todos(root: &Path, date: &str) -> Result<Vec<String>> {
    let today = util::today_beijing();
    if today == date {
        return Ok(Vec::new());
    }
    let report = analyze(root, date)?;
    if report.todos.is_empty() {
        return Ok(Vec::new());
    }

    let rel = format!("50-Daily/{today}.md");
    let mut content = match write::read_note(root, &rel) {
        Ok(content) => content,
        Err(_) => format!(
            "---\ntitle: \"{today}\"\ndate: {today}\ntags: daily\n---\n\n# {today}\n\n## 今日重点\n\n"
        ),
    };
    let added = insert_todos(&mut content, &report.todos);
    if added.is_empty() {
        return Ok(added);
    }
    write::replace_file(root, &rel, &content)?;
    indexer::ensure(root, false, false, None)?;
    Ok(added)
}

fn insert_todos(content: &mut String, todos: &[String]) -> Vec<String> {
    let existing: std::collections::HashSet<String> = content
        .lines()
        .filter(|line| line.trim_start().starts_with("- ["))
        .map(normalize)
        .collect();
    let new_items: Vec<String> = todos
        .iter()
        .filter(|todo| !existing.contains(&normalize(todo)))
        .cloned()
        .collect();
    if new_items.is_empty() {
        return new_items;
    }

    if let Some(start) = content.find("## 今日重点") {
        let after = start + "## 今日重点".len();
        let insert_at = content[after..]
            .find("\n## ")
            .map(|offset| after + offset)
            .unwrap_or(content.len());
        let before = &content[..insert_at];
        let prefix = if before.ends_with("\n\n") {
            ""
        } else if before.ends_with('\n') {
            "\n"
        } else {
            "\n\n"
        };
        content.insert_str(insert_at, &format!("{prefix}{}\n", new_items.join("\n")));
    } else {
        content.push_str("\n## 今日重点\n\n");
        content.push_str(&new_items.join("\n"));
        content.push('\n');
    }
    new_items
}

pub fn list_open_work(root: &Path) -> Result<Vec<OpenItem>> {
    let mut items = Vec::new();
    for (rel, path) in indexer::collect_markdown(&root.join(indexer::MEMORY_DIR)) {
        if rel.starts_with("40-Archive/")
            || rel == "00-Inbox/open-work.md"
            || rel.starts_with("00-Inbox/Digest-")
        {
            continue;
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("读取失败: {}", path.display()))?;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("- [ ]") {
                items.push(OpenItem {
                    path: rel.clone(),
                    text: trimmed.to_string(),
                });
            }
        }
    }
    Ok(items)
}

pub fn sync_open_work(root: &Path) -> Result<std::path::PathBuf> {
    let items = list_open_work(root)?;
    let mut by_path: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for item in items {
        by_path.entry(item.path).or_default().push(item.text);
    }

    let today = util::today_beijing();
    let mut content = format!(
        "---\ntitle: \"Open Work\"\ndate: {today}\ntags: open-work\n---\n\n# Open Work\n\n> 由 `colna work sync` 生成;修改请直接改源文件,不要只改这里。\n\n"
    );
    if by_path.is_empty() {
        content.push_str("（无未完成项）\n");
    } else {
        for (path, items) in by_path {
            content.push_str(&format!("## `{path}`\n\n"));
            for item in items {
                content.push_str(&format!("{item}\n"));
            }
            content.push('\n');
        }
    }
    let rel = "00-Inbox/open-work.md";
    write::replace_file(root, rel, &content)?;
    indexer::ensure(root, false, false, None)?;
    Ok(root.join(indexer::MEMORY_DIR).join(rel))
}

/// 为可沉淀经验找语义相近的已有笔记(向量相似度 > 0.82)。
pub fn annotate_similar(
    root: &Path,
    report: &Report,
    embedder: &mut Embedder,
) -> Result<Vec<(String, String, f32)>> {
    if report.lessons.is_empty() {
        return Ok(Vec::new());
    }
    indexer::ensure(root, false, true, Some(embedder))?;
    let collection = store::open(&root.join(indexer::INDEX_PATH).to_string_lossy())?;
    let mut result = Vec::new();
    for lesson in report.lessons.iter().take(8) {
        let query = embedder.embed_query(lesson)?;
        let hits = search::query(
            &collection,
            &query,
            lesson,
            &SearchOptions {
                topk: 1,
                hybrid: false,
                filters: Filters::default(),
                recency_weight: 0.0,
                mmr: false,
                include_summary: false,
                link_boost: 0.0,
            },
            None,
        )?;
        if let Some(hit) = hits.first() {
            if hit.score > 0.82 {
                result.push((lesson.clone(), hit.source_path.clone(), hit.score));
            }
        }
    }
    collection.close().ok();
    Ok(result)
}

pub fn ensure_daily_exists(root: &Path, date: &str) -> Result<()> {
    let rel = format!("50-Daily/{date}.md");
    if write::read_note(root, &rel).is_ok() {
        return Ok(());
    }
    let content = format!(
        "---\ntitle: \"{date}\"\ndate: {date}\ntags: daily\n---\n\n# {date}\n\n## 今日重点\n\n"
    );
    write::replace_file(root, &rel, &content)
}

pub fn validate_date(date: &str) -> Result<()> {
    if util::parse_date(date).is_none() {
        bail!("日期必须是 YYYY-MM-DD: {date}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_todos_dedupes_and_keeps_section() {
        let mut content = "# D\n\n## 今日重点\n\n- [ ] a\n\n## 工作日志\n".to_string();
        let todos = vec!["- [ ] a".to_string(), "- [ ] b".to_string()];
        let added = insert_todos(&mut content, &todos);
        assert_eq!(added, vec!["- [ ] b".to_string()]);
        assert_eq!(content.matches("- [ ] a").count(), 1);
        assert!(content.contains("- [ ] b"));
    }

    #[test]
    fn analyzes_todos_lessons_and_worklogs() {
        let root = std::env::temp_dir().join(format!(
            "colna-digest-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("memory/50-Daily")).unwrap();
        std::fs::write(
            root.join("memory/50-Daily/2026-01-01.md"),
            "---\ntitle: D\ndate: 2026-01-01\n---\n# D\n\n## 今日重点\n\n- [ ] todo\n\n## 工作日志 — 10:00\n\n- 根因是 X\n- 普通进展\n",
        )
        .unwrap();
        let report = analyze(&root, "2026-01-01").unwrap();
        assert_eq!(report.todos, vec!["- [ ] todo".to_string()]);
        assert!(report.lessons.iter().any(|line| line.contains("根因")));
        assert_eq!(report.worklogs.len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }
}

//! 轻量任务 receipt:结构化 task note + checkpoint / finish / list。
//!
//! 只记录,不跑 worker;任务执行仍由 agent/人完成。

use crate::chunker;
use crate::indexer;
use crate::util;
use crate::write;
use anyhow::{bail, Context, Result};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Task {
    pub path: String,
    pub title: String,
    pub status: String,
    pub project: Option<String>,
    pub goal: Option<String>,
    pub date: String,
}

pub fn start(
    root: &Path,
    goal: &str,
    project: Option<&str>,
    plan: Option<&str>,
    confirm: bool,
) -> Result<write::NotePreview> {
    if goal.trim().is_empty() {
        bail!("goal 不能为空");
    }
    let date = util::today_beijing();
    let slug = util::slugify(goal);
    let rel = match project.map(str::trim).filter(|value| !value.is_empty()) {
        Some(project) => format!(
            "10-Projects/{}/tasks/{date}-{slug}.md",
            util::slugify(project)
        ),
        None => format!("00-Inbox/tasks/{date}-{slug}.md"),
    };
    let project_line = project
        .map(|project| format!("project: {}\n", util::yaml_quote(project)))
        .unwrap_or_default();
    let content = format!(
        "---\ntitle: {}\ndate: {date}\ntags: task\nstatus: open\n{project_line}goal: {}\n---\n\n# {}\n\n## Goal\n\n{}\n\n## Plan\n\n{}\n\n## Checkpoints\n\n",
        util::yaml_quote(goal),
        util::yaml_quote(goal),
        goal.trim(),
        goal.trim(),
        plan.unwrap_or("").trim()
    );
    let preview = write::put_at(root, &rel, &content, confirm)?;
    if confirm {
        indexer::ensure(root, false, true, None)?;
    }
    Ok(preview)
}

pub fn checkpoint(
    root: &Path,
    path: &str,
    note: &str,
    confirm: bool,
) -> Result<write::NotePreview> {
    let hash = write::note_hash(root, path)?;
    let heading = format!("Checkpoint {}", util::now_beijing());
    let preview = write::append(root, path, note, Some(&heading), Some(&hash), confirm)?;
    if confirm {
        indexer::ensure(root, false, true, None)?;
    }
    Ok(preview)
}

pub fn finish(
    root: &Path,
    path: &str,
    status: &str,
    outcome: &str,
    evidence: Option<&str>,
    confirm: bool,
) -> Result<write::NotePreview> {
    if !matches!(status, "done" | "blocked" | "open") {
        bail!("status 必须是 done / blocked / open");
    }
    let content = write::read_note(root, path)?;
    let hash = crate::state::content_hash(&content);
    let mut next = set_frontmatter_value(&content, "status", status);
    next.push_str(&format!("\n## Result\n\n{}\n", outcome.trim()));
    if let Some(evidence) = evidence.map(str::trim).filter(|value| !value.is_empty()) {
        next.push_str(&format!("\n## Evidence\n\n{evidence}\n"));
    }
    let preview = write::update(root, path, &next, &hash, confirm)?;
    if confirm {
        indexer::ensure(root, false, true, None)?;
    }
    Ok(preview)
}

pub fn list(root: &Path, status: Option<&str>) -> Result<Vec<Task>> {
    let mut tasks = Vec::new();
    for (rel, path) in indexer::collect_markdown(&root.join(indexer::MEMORY_DIR)) {
        if !rel.contains("/tasks/") {
            continue;
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("读取任务失败: {}", path.display()))?;
        let (front, _) = chunker::parse_front_matter(&content);
        let task_status = front
            .extra
            .get("status")
            .cloned()
            .unwrap_or_else(|| "open".to_string());
        if let Some(status) = status {
            if task_status != status {
                continue;
            }
        }
        tasks.push(Task {
            path: rel,
            title: front.title.unwrap_or_default(),
            status: task_status,
            project: front.extra.get("project").cloned(),
            goal: front.extra.get("goal").cloned(),
            date: front.date.unwrap_or_default(),
        });
    }
    tasks.sort_by(|a, b| b.date.cmp(&a.date).then(a.path.cmp(&b.path)));
    Ok(tasks)
}

fn set_frontmatter_value(content: &str, key: &str, value: &str) -> String {
    let Some(start) = content.find("---") else {
        return format!("---\n{key}: {value}\n---\n\n{content}");
    };
    let after = start + 3;
    let Some(end_offset) = content[after..].find("\n---") else {
        return content.to_string();
    };
    let end = after + end_offset;
    let mut block: Vec<String> = content[after..end].lines().map(str::to_string).collect();
    let mut replaced = false;
    for line in &mut block {
        if line.trim_start().starts_with(&format!("{key}:")) {
            *line = format!("{key}: {value}");
            replaced = true;
            break;
        }
    }
    if !replaced {
        block.push(format!("{key}: {value}"));
    }
    format!(
        "{}{}{}",
        &content[..after],
        block.join("\n"),
        &content[end..]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_frontmatter_status() {
        let content = "---\ntitle: \"T\"\nstatus: open\n---\n\nbody\n";
        let next = set_frontmatter_value(content, "status", "done");
        assert!(next.contains("status: done"));
        assert!(!next.contains("status: open"));
        assert!(next.ends_with("body\n"));
    }

    #[test]
    fn status_update_does_not_add_blank_line_before_closing_fence() {
        let content = "---\ntitle: T\nstatus: open\ngoal: G\n---\n\nbody\n";
        let next = set_frontmatter_value(content, "status", "done");
        assert!(!next.contains("\n\n---"), "unexpected blank line: {next:?}");
        assert!(next.contains("status: done"));
    }

    #[test]
    fn start_preview_does_not_write() {
        let root = std::env::temp_dir().join(format!(
            "colna-task-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("memory")).unwrap();
        let preview = start(&root, "Test goal", Some("Demo"), Some("step"), false).unwrap();
        assert!(!preview.applied);
        assert!(!root.join("memory").join(&preview.path).exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}

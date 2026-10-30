//! 笔记写入:put / append / update / archive。
//!
//! 统一规则:
//! - 只写 `memory/` 下、排除 `_` 前缀目录的 `.md` 文件;
//! - 新建默认落 `00-Inbox/`;
//! - `confirm=false` 只返回预览/ diff,不落盘;
//! - 更新/追加必须带 `expected_hash`,防止覆盖并发修改;
//! - 归档只移动,不删除。

use crate::state;
use crate::util;
use anyhow::{bail, Context, Result};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

const MEMORY_DIR: &str = "memory";
const INBOX_DIR: &str = "00-Inbox";
const ARCHIVE_DIR: &str = "40-Archive";

#[derive(Debug, Clone)]
pub struct NotePreview {
    pub path: String,
    pub content: String,
    pub hash: String,
    pub diff: String,
    pub applied: bool,
}

#[derive(Debug, Clone)]
pub struct ArchivePreview {
    pub from: String,
    pub to: String,
    pub applied: bool,
}

/// CLI `colna add` / `colna clip`:真正写盘 + 交给调用方重建索引。
pub fn add_note(
    root: &Path,
    title: &str,
    tags: &str,
    body: &str,
    extra: &[(&str, &str)],
    reindex: impl FnOnce(&Path) -> Result<()>,
) -> Result<PathBuf> {
    let rel = unique_default_path(root, title)?;
    let content = render_note(title, tags, body, extra);
    let path = memory_path(root, &rel);
    write_atomic(&path, &content)?;
    reindex(root)?;
    Ok(path)
}

/// `kb_put`:新建笔记;默认只预览。
pub fn put(
    root: &Path,
    title: &str,
    body: &str,
    tags: &str,
    path: Option<&str>,
    confirm: bool,
) -> Result<NotePreview> {
    let rel = match path {
        Some(path) => normalize_rel_path(path)?,
        None => unique_default_path(root, title)?,
    };
    let full = memory_path(root, &rel);
    if full.exists() {
        bail!("目标已存在: {rel};请用 kb_update,或换一个 path");
    }
    let content = render_note(title, tags, body, &[]);
    note_preview(root, &rel, "", &content, confirm)
}

/// `kb_append`:追加到已有笔记;默认只预览。
pub fn append(
    root: &Path,
    path: &str,
    body: &str,
    heading: Option<&str>,
    expected_hash: Option<&str>,
    confirm: bool,
) -> Result<NotePreview> {
    let rel = normalize_rel_path(path)?;
    if body.trim().is_empty() {
        bail!("body 不能为空");
    }
    let old = read_note(root, &rel)?;
    validate_expected_hash(&old, expected_hash, &rel)?;

    let mut next = old.clone();
    if !next.ends_with('\n') {
        next.push('\n');
    }
    next.push('\n');
    if let Some(heading) = heading.map(str::trim).filter(|h| !h.is_empty()) {
        next.push_str(&format!("## {heading}\n\n"));
    }
    next.push_str(body.trim());
    next.push('\n');

    note_preview(root, &rel, &old, &next, confirm)
}

/// `kb_update`:整篇替换;必须带 expected_hash。
pub fn update(
    root: &Path,
    path: &str,
    content: &str,
    expected_hash: &str,
    confirm: bool,
) -> Result<NotePreview> {
    let rel = normalize_rel_path(path)?;
    let old = read_note(root, &rel)?;
    validate_expected_hash(&old, Some(expected_hash), &rel)?;
    validate_frontmatter(content)?;
    note_preview(root, &rel, &old, content, confirm)
}

/// `kb_archive`:移动到 `40-Archive/`,永不删除。
pub fn archive(
    root: &Path,
    path: &str,
    expected_hash: Option<&str>,
    confirm: bool,
) -> Result<ArchivePreview> {
    let rel = normalize_rel_path(path)?;
    if rel.starts_with(&format!("{ARCHIVE_DIR}/")) {
        bail!("已经在归档目录: {rel}");
    }
    let old = read_note(root, &rel)?;
    validate_expected_hash(&old, expected_hash, &rel)?;

    let mut target_rel = format!("{ARCHIVE_DIR}/{rel}");
    let stamp = util::now_beijing()
        .replace(['-', ':', ' '], "")
        .chars()
        .take(12)
        .collect::<String>();
    let stem = Path::new(&rel)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("note");
    let parent = Path::new(&rel)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("");
    let mut n = 2;
    while memory_path(root, &target_rel).exists() {
        target_rel = if parent.is_empty() {
            format!("{ARCHIVE_DIR}/{stem}-archived-{stamp}-{n}.md")
        } else {
            format!("{ARCHIVE_DIR}/{parent}/{stem}-archived-{stamp}-{n}.md")
        };
        n += 1;
    }
    let target = memory_path(root, &target_rel);

    if confirm {
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("创建归档目录失败: {}", parent.display()))?;
        }
        std::fs::rename(memory_path(root, &rel), &target)
            .with_context(|| format!("归档移动失败: {rel} → {target_rel}"))?;
    }

    Ok(ArchivePreview {
        from: rel,
        to: target_rel,
        applied: confirm,
    })
}

/// 按指定路径写新文件(预览/确认);供 task 等结构化笔记使用。
pub fn put_at(root: &Path, rel: &str, content: &str, confirm: bool) -> Result<NotePreview> {
    let rel = normalize_rel_path(rel)?;
    let full = memory_path(root, &rel);
    if full.exists() {
        bail!("目标已存在: {rel}");
    }
    note_preview(root, &rel, "", content, confirm)
}

fn note_preview(
    root: &Path,
    rel: &str,
    old: &str,
    new: &str,
    confirm: bool,
) -> Result<NotePreview> {
    validate_frontmatter(new)?;
    let hash = state::content_hash(new);
    let diff = unified_diff(old, new, rel);
    if confirm {
        let path = memory_path(root, rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("创建笔记目录失败: {}", parent.display()))?;
        }
        write_atomic(&path, new)?;
    }
    Ok(NotePreview {
        path: rel.to_string(),
        content: new.to_string(),
        hash,
        diff,
        applied: confirm,
    })
}

/// 路径归一化:去 `memory/` 前缀、拒绝绝对路径/`..`/`_` 目录,补 `.md`。
pub fn normalize_rel_path(input: &str) -> Result<String> {
    let mut path = input.trim().replace('\\', "/");
    if let Some(stripped) = path.strip_prefix("memory/") {
        path = stripped.to_string();
    }
    let path = path.trim_start_matches("./").trim_end_matches('/');
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\0')
        || path
            .split('/')
            .any(|segment| segment == ".." || segment.is_empty())
    {
        bail!("非法路径: {input}");
    }
    if path.split('/').any(|segment| segment.starts_with('_')) {
        bail!("不允许写入 `_` 前缀目录: {input}");
    }
    if path.ends_with(".md") {
        Ok(path.to_string())
    } else {
        Ok(format!("{path}.md"))
    }
}

fn unique_default_path(root: &Path, title: &str) -> Result<String> {
    let base = util::slugify(title);
    let mut rel = format!("{INBOX_DIR}/{base}.md");
    let mut n = 2;
    while memory_path(root, &rel).exists() {
        rel = format!("{INBOX_DIR}/{base}-{n}.md");
        n += 1;
    }
    Ok(rel)
}

fn memory_path(root: &Path, rel: &str) -> PathBuf {
    root.join(MEMORY_DIR).join(rel)
}

/// 直接替换文件内容(仍走路径校验 + 原子写);供 digest/task 等结构化更新使用。
pub fn replace_file(root: &Path, rel: &str, content: &str) -> Result<()> {
    let rel = normalize_rel_path(rel)?;
    validate_frontmatter(content)?;
    let path = memory_path(root, &rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("创建笔记目录失败: {}", parent.display()))?;
    }
    write_atomic(&path, content)
}

/// 当前文件 sha256。
pub fn note_hash(root: &Path, rel: &str) -> Result<String> {
    read_note(root, rel).map(|content| state::content_hash(&content))
}

pub fn read_note(root: &Path, rel: &str) -> Result<String> {
    let path = memory_path(root, rel);
    std::fs::read_to_string(&path).with_context(|| format!("读取笔记失败: {}", path.display()))
}

fn validate_expected_hash(actual: &str, expected: Option<&str>, rel: &str) -> Result<()> {
    if let Some(expected) = expected {
        let actual_hash = state::content_hash(actual);
        if expected != actual_hash {
            bail!("expected_hash 不匹配: {rel} 当前为 {actual_hash};请重新读取后再改");
        }
    }
    Ok(())
}

fn validate_frontmatter(content: &str) -> Result<()> {
    let trimmed = content.trim_start_matches('\u{feff}');
    if !trimmed.starts_with("---") {
        return Ok(());
    }
    let rest = &trimmed[3..];
    if !rest.contains("\n---") {
        bail!("front-matter 未闭合:以 --- 开头但找不到结束的 ---");
    }
    Ok(())
}

fn render_note(title: &str, tags: &str, body: &str, extra: &[(&str, &str)]) -> String {
    let mut content = String::new();
    content.push_str("---\n");
    content.push_str(&format!("title: {}\n", util::yaml_quote(title.trim())));
    content.push_str(&format!("date: {}\n", util::today_beijing()));
    if !tags.trim().is_empty() {
        content.push_str(&format!("tags: {}\n", tags.trim()));
    }
    for (key, value) in extra {
        if matches!(*key, "title" | "date" | "tags") {
            continue;
        }
        content.push_str(&format!("{key}: {}\n", util::yaml_quote(value)));
    }
    content.push_str("---\n\n");
    let body = body.trim();
    if !body.is_empty() {
        content.push_str(body);
        content.push('\n');
    }
    content
}

fn write_atomic(path: &Path, content: &str) -> Result<()> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("note.md");
    let temp = path.with_file_name(format!(
        ".{file_name}.tmp-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .with_context(|| format!("创建临时文件失败: {}", temp.display()))?;
        file.write_all(content.as_bytes())
            .with_context(|| format!("写入临时文件失败: {}", temp.display()))?;
        file.sync_all().ok();
    }
    if let Err(error) = std::fs::rename(&temp, path) {
        let _ = std::fs::remove_file(&temp);
        return Err(error).with_context(|| format!("替换文件失败: {}", path.display()));
    }
    Ok(())
}

fn unified_diff(old: &str, new: &str, rel: &str) -> String {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let base = std::env::temp_dir();
    let old_path = base.join(format!("colna-diff-old-{}-{stamp}.md", std::process::id()));
    let new_path = base.join(format!("colna-diff-new-{}-{stamp}.md", std::process::id()));
    if std::fs::write(&old_path, old).is_err() || std::fs::write(&new_path, new).is_err() {
        return format!(
            "- {} 行\n+ {} 行\n",
            old.lines().count(),
            new.lines().count()
        );
    }
    let output = Command::new("diff")
        .args(["-u", "-L", &format!("a/{rel}"), "-L", &format!("b/{rel}")])
        .arg(&old_path)
        .arg(&new_path)
        .output();
    let _ = std::fs::remove_file(&old_path);
    let _ = std::fs::remove_file(&new_path);
    match output {
        Ok(output) => {
            let text = String::from_utf8_lossy(&output.stdout).to_string();
            if text.trim().is_empty() {
                "（无内容差异）\n".to_string()
            } else {
                text
            }
        }
        Err(_) => format!(
            "- {} 行\n+ {} 行\n",
            old.lines().count(),
            new.lines().count()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(1);

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            let id = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir()
                .join(format!("colna-memo-write-test-{}-{id}", std::process::id()));
            std::fs::create_dir_all(root.join(MEMORY_DIR)).unwrap();
            Self(root)
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn rejects_path_traversal_and_underscore_dirs() {
        assert!(normalize_rel_path("../x.md").is_err());
        assert!(normalize_rel_path("/tmp/x.md").is_err());
        assert!(normalize_rel_path("_templates/x.md").is_err());
        assert!(normalize_rel_path("a//b.md").is_err());
        assert_eq!(
            normalize_rel_path("memory/00-Inbox/x").unwrap(),
            "00-Inbox/x.md"
        );
    }

    #[test]
    fn put_preview_does_not_write_until_confirmed() {
        let root = TestRoot::new();
        let preview = put(&root.0, "Demo", "body", "a,b", None, false).unwrap();
        assert!(!preview.applied);
        assert!(preview.path.starts_with("00-Inbox/"));
        assert!(preview.content.contains("title: \"Demo\""));
        assert!(!memory_path(&root.0, &preview.path).exists());

        let applied = put(&root.0, "Demo", "body", "a,b", None, true).unwrap();
        assert!(applied.applied);
        assert!(memory_path(&root.0, &applied.path).exists());
    }

    #[test]
    fn append_requires_matching_hash() {
        let root = TestRoot::new();
        let note = put(&root.0, "Demo", "body", "", None, true).unwrap();
        let error = append(&root.0, &note.path, "more", None, Some("wrong"), true).unwrap_err();
        assert!(error.to_string().contains("expected_hash"));

        let appended = append(
            &root.0,
            &note.path,
            "more",
            Some("More"),
            Some(&note.hash),
            true,
        )
        .unwrap();
        assert!(appended.applied);
        let content = read_note(&root.0, &note.path).unwrap();
        assert!(content.contains("## More"));
    }

    #[test]
    fn archive_moves_instead_of_deleting() {
        let root = TestRoot::new();
        let note = put(&root.0, "Demo", "body", "", None, true).unwrap();
        let archived = archive(&root.0, &note.path, Some(&note.hash), true).unwrap();

        assert!(archived.applied);
        assert!(!memory_path(&root.0, &note.path).exists());
        assert!(memory_path(&root.0, &archived.to).exists());
        assert!(archived.to.starts_with("40-Archive/"));
    }

    #[test]
    fn update_rejects_unclosed_frontmatter() {
        let root = TestRoot::new();
        let note = put(&root.0, "Demo", "body", "", None, true).unwrap();
        let error = update(&root.0, &note.path, "---\ntitle: X\n", &note.hash, false).unwrap_err();
        assert!(error.to_string().contains("未闭合"));
    }
}

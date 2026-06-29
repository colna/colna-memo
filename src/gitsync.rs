//! P3:`colna add` / `colna sync` —— 封装 git add/commit/pull/push + 自动 reindex。
//!
//! 真源是 memory/ 下的 Markdown(走 git 跨设备同步),本模块把
//! “写笔记 / 跨设备同步” 的常用动作收成一条命令,顺手把本地 zvec 索引重建。

use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

const MEMORY_DIR: &str = "memory";
// 新笔记默认落收件箱(PARA 约定,见 memory/CONVENTIONS.md),后续整理
const NOTES_DIR: &str = "00-Inbox";

/// 在 root 下执行一条 git 命令,返回 stdout(去尾换行)。失败则带上 stderr 报错。
fn git(root: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .with_context(|| format!("执行 git {:?} 失败(git 是否安装?)", args))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        bail!("git {:?} 失败: {}", args, stderr.trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

/// 提交前守卫:扫描 memory/ 是否残留 git 冲突标记(`<<<<<<<` / `>>>>>>>`)。
/// 命中即中止 sync,避免把未解决的冲突文件提交并同步出去(rebase 半途留下的脏文件)。
///
/// 用 `git grep` 而非 `git()` helper:无匹配时 git grep 退出码为 1(`git()` 会误判为失败)。
/// 退出码语义:0 = 找到冲突标记 → 中止;1 = 无匹配 → 干净;其它 = 真错。
/// 同时扫 untracked,新建但未跟踪的文件也覆盖。
fn check_conflict_markers(root: &Path) -> Result<()> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "grep", "-n", "--untracked", "-e", "^<<<<<<<", "-e", "^>>>>>>>", "--", MEMORY_DIR,
        ])
        .output()
        .context("执行 git grep 失败(git 是否安装?)")?;
    match out.status.code() {
        Some(0) => {
            let hits = String::from_utf8_lossy(&out.stdout);
            bail!(
                "检测到未解决的冲突标记,已中止 sync(未提交、未 push)。\n请先解决以下位置再重试:\n{}",
                hits.trim()
            );
        }
        Some(1) => Ok(()),
        _ => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            bail!("冲突标记检测失败: {}", stderr.trim());
        }
    }
}

/// 检查 root 是不是 git 仓库。
fn ensure_repo(root: &Path) -> Result<()> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .context("执行 git 失败(git 是否安装?)")?;
    if !out.status.success() {
        bail!("{} 不是 git 仓库,无法 sync。请先 git init 并配置远端。", root.display());
    }
    Ok(())
}

/// 把标题转成相对安全的文件名:保留中英文与数字,空白转连字符,去掉路径分隔符等。
fn slugify(title: &str) -> String {
    let mut s = String::with_capacity(title.len());
    for ch in title.chars() {
        if ch.is_alphanumeric() {
            s.push(ch);
        } else if ch.is_whitespace() || ch == '-' || ch == '_' {
            s.push('-');
        }
        // 其它字符(/ \ . : 等)直接丢弃,避免越权或扩展名歧义
    }
    let s = s.trim_matches('-').to_string();
    if s.is_empty() {
        "note".to_string()
    } else {
        s
    }
}

/// `colna add`:在 memory/00-Inbox/ 下新建一篇带 front-matter 的笔记,然后增量重建索引。
///
/// body 为正文(可来自 --body 或 stdin);date 用今天;不自动 git 提交(交给 sync)。
pub fn add_note(
    root: &Path,
    title: &str,
    tags: &str,
    body: &str,
    reindex: impl FnOnce(&Path) -> Result<()>,
) -> Result<PathBuf> {
    let notes_dir = root.join(MEMORY_DIR).join(NOTES_DIR);
    std::fs::create_dir_all(&notes_dir)
        .with_context(|| format!("创建笔记目录失败: {}", notes_dir.display()))?;

    // 文件名:slug;若已存在则追加 -2 -3 ... 避免覆盖
    let base = slugify(title);
    let mut path = notes_dir.join(format!("{}.md", base));
    let mut n = 2;
    while path.exists() {
        path = notes_dir.join(format!("{}-{}.md", base, n));
        n += 1;
    }

    let date = today();
    let fm_tags = tags.trim();
    let mut content = String::new();
    content.push_str("---\n");
    content.push_str(&format!("title: {}\n", title.trim()));
    content.push_str(&format!("date: {}\n", date));
    if !fm_tags.is_empty() {
        content.push_str(&format!("tags: {}\n", fm_tags));
    }
    content.push_str("---\n\n");
    let body = body.trim();
    if !body.is_empty() {
        content.push_str(body);
        content.push('\n');
    }

    std::fs::write(&path, content).with_context(|| format!("写入笔记失败: {}", path.display()))?;
    println!("📝 已新建笔记 → {}", path.display());

    // 自动增量重建索引
    reindex(root)?;
    Ok(path)
}

/// `colna sync`:跨设备同步一条龙。
///
/// 步骤:
///   1. git pull --rebase(拉远端最新,真源对齐)
///   2. reindex(把拉下来的变更吸收进本地索引)
///   3. 冲突标记守卫:memory/ 残留 `<<<<<<<` / `>>>>>>>` 则中止
///   4. git add memory/ + 有改动则 commit
///   5. reindex(把本地新改动也吸收进索引)
///   6. git push
fn run_sync(root: &Path, message: &str, reindex: &dyn Fn(&Path) -> Result<()>) -> Result<()> {
    ensure_repo(root)?;

    // 1. pull --rebase(无上游/无远端时给出友好提示,不致命)
    println!("⬇️  git pull --rebase ...");
    match git(root, &["pull", "--rebase"]) {
        Ok(o) => {
            if !o.is_empty() {
                println!("{}", o);
            }
        }
        Err(e) => println!("(跳过 pull:{})", first_line(&e.to_string())),
    }

    // 2. 拉完先 reindex,保证本地索引含远端内容
    reindex(root)?;

    // 3. 提交前守卫:有未解决的冲突标记就中止,不让脏文件进版本库
    check_conflict_markers(root)?;

    // 4. 暂存 memory/ 下的真源改动
    git(root, &["add", MEMORY_DIR])?;
    let staged = git(root, &["status", "--porcelain", "--", MEMORY_DIR])?;
    if staged.is_empty() {
        println!("✅ memory/ 无本地改动,无需提交。");
    } else {
        println!("⬆️  提交本地改动 ...");
        git(root, &["commit", "-m", message])?;
        // 4. 提交后再 reindex 一次(吸收本地新内容)
        reindex(root)?;
    }

    // 5. push
    println!("⬆️  git push ...");
    match git(root, &["push"]) {
        Ok(o) => {
            if !o.is_empty() {
                println!("{}", o);
            }
            println!("✅ 同步完成。");
        }
        Err(e) => {
            return Err(anyhow!("push 失败: {}", first_line(&e.to_string())));
        }
    }
    Ok(())
}

/// 对外入口:见 run_sync。
pub fn sync(root: &Path, message: &str, reindex: impl Fn(&Path) -> Result<()>) -> Result<()> {
    run_sync(root, message, &reindex)
}

/// 取错误信息首行,避免多行 stderr 刷屏。
fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or(s).to_string()
}

/// 今天日期 YYYY-MM-DD(本地时区)。用 date 命令,避免引入额外时间库。
fn today() -> String {
    Command::new("date")
        .args(["+%Y-%m-%d"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_default()
}

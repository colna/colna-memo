//! P3:`colna add` / `colna sync` —— 封装 git add/commit/fetch/rebase/push + 自动 reindex。
//!
//! 真源是 memory/ 下的 Markdown(走 git 跨设备同步),本模块把
//! “写笔记 / 跨设备同步” 的常用动作收成一条命令,顺手把本地 zvec 索引重建。

use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

const MEMORY_DIR: &str = "memory";

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
            "grep",
            "-n",
            "--untracked",
            "-e",
            "^<<<<<<<",
            "-e",
            "^>>>>>>>",
            "--",
            MEMORY_DIR,
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
        bail!(
            "{} 不是 git 仓库,无法 sync。请先 git init 并配置远端。",
            root.display()
        );
    }
    Ok(())
}

/// sync 不能在未完成的 Git 操作上继续,否则后续 commit/reindex 会污染现场。
fn ensure_no_git_operation(root: &Path) -> Result<()> {
    let git_dir = PathBuf::from(git(root, &["rev-parse", "--absolute-git-dir"])?);
    let operation_markers = [
        "rebase-merge",
        "rebase-apply",
        "MERGE_HEAD",
        "CHERRY_PICK_HEAD",
        "REVERT_HEAD",
    ];
    if let Some(marker) = operation_markers
        .iter()
        .find(|marker| git_dir.join(marker).exists())
    {
        bail!("检测到未完成的 Git 操作({marker}),请先处理后再 sync");
    }

    let unmerged = git(root, &["diff", "--name-only", "--diff-filter=U"])?;
    if !unmerged.is_empty() {
        bail!("检测到未解决的 Git 冲突,请先处理后再 sync:\n{unmerged}");
    }
    Ok(())
}

/// 拒绝调用方已经 staged 的非 memory 文件,防止 `colna sync` 顺带提交源码等改动。
fn ensure_no_staged_changes_outside_memory(root: &Path) -> Result<()> {
    let staged = git(root, &["diff", "--cached", "--name-only"])?;
    let outside: Vec<&str> = staged
        .lines()
        .filter(|path| *path != MEMORY_DIR && !path.starts_with("memory/"))
        .collect();
    if !outside.is_empty() {
        bail!(
            "检测到 memory/ 外已有 staged 改动,为避免误提交已中止 sync:\n{}",
            outside.join("\n")
        );
    }
    Ok(())
}

/// `colna sync`:跨设备同步一条龙。
///
/// 步骤:
///   1. Git 状态与冲突标记守卫
///   2. git add memory/ + 有改动则 commit
///   3. git fetch origin main + git rebase --autostash origin/main(两步拉取,不读 FETCH_HEAD 多分支)
///   4. reindex(最终真源状态只处理一次)
///   5. git push
fn run_sync(root: &Path, message: &str, reindex: &dyn Fn(&Path) -> Result<()>) -> Result<()> {
    ensure_repo(root)?;
    ensure_no_git_operation(root)?;
    check_conflict_markers(root)?;
    ensure_no_staged_changes_outside_memory(root)?;

    // 先保存本地 memory 真源,让后续 rebase 有稳定的本地提交可重放。
    git(root, &["add", "--", MEMORY_DIR])?;
    let staged = git(root, &["status", "--porcelain", "--", MEMORY_DIR])?;
    if staged.is_empty() {
        println!("✅ memory/ 无本地改动,无需提交。");
    } else {
        println!("⬆️  提交本地改动 ...");
        git(root, &["commit", "-m", message])?;
    }

    // 两步拉取:先 `fetch origin main` 只抓单分支,再 `rebase --autostash origin/main`
    // 把本地提交重放到 remote-tracking ref 上。**为什么不回到 `git pull --rebase`**:
    // pull(即便显式写 `origin main`)按 `FETCH_HEAD` 里「可合并」的行数判定上游,并发
    // fetch 写出两行即 `fatal: Cannot rebase onto multiple branches.`(2026-11-04 30/30
    // 并发实测);rebase 到 `origin/main` 完全不读 `FETCH_HEAD`,故对多行残留免疫。
    // 本库约定分支为 main;换分支的仓库需同步改这两行。
    println!("⬇️  git fetch origin main ...");
    let fetched = match git(root, &["fetch", "origin", "main"]) {
        Ok(out) => out,
        Err(error) => {
            // 保留 git 原始 stderr 全量,不截首行;失败即停,不进入 rebase。
            eprintln!("{}", error);
            bail!("git fetch origin main 失败,已停止 rebase/reindex/push");
        }
    };
    if !fetched.is_empty() {
        println!("{}", fetched);
    }

    let head_before = git(root, &["rev-parse", "HEAD"])?;
    println!("⬇️  git rebase --autostash origin/main ...");
    if let Err(error) = git(root, &["rebase", "--autostash", "origin/main"]) {
        eprintln!("{}", error);
        let conflicts = git(root, &["diff", "--name-only", "--diff-filter=U"]).unwrap_or_default();
        // 冲突现场一律保留:不自动 abort、不自动 continue。
        if rebase_in_progress(root) {
            bail!(
                "rebase 冲突,已停止 reindex/push(未自动 abort/continue,现场保留):\n{}",
                if conflicts.is_empty() { "(见 git status)" } else { conflicts.as_str() }
            );
        }
        bail!(
            "rebase --autostash 恢复失败,已停止 reindex/push: {}\n未合并文件:\n{}",
            first_line(&error.to_string()),
            if conflicts.is_empty() { "(见 git status)" } else { conflicts.as_str() }
        );
    }
    let head_after = git(root, &["rev-parse", "HEAD"])?;
    if head_after == head_before {
        println!(
            "✅ 已是最新:origin/main 无新提交,无需 rebase(HEAD {})",
            short_sha(&head_before)
        );
    } else {
        println!(
            "✅ 已 rebase 到 origin/main:{} → {}",
            short_sha(&head_before),
            short_sha(&head_after)
        );
    }

    ensure_no_git_operation(root)
        .map_err(|error| anyhow!("rebase 后 autostash 恢复未完成,已停止 reindex/push: {error}"))?;

    // commit + rebase 完成后真源才稳定,只需重建一次。
    reindex(root)?;

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

/// rebase 是否仍在进行中(冲突未解决时 `.git/rebase-merge` / `rebase-apply` 存在)。
fn rebase_in_progress(root: &Path) -> bool {
    git(root, &["rev-parse", "--absolute-git-dir"])
        .map(|dir| {
            let dir = PathBuf::from(dir);
            dir.join("rebase-merge").exists() || dir.join("rebase-apply").exists()
        })
        .unwrap_or(false)
}

/// 取 sha 前 7 位用于日志展示。
fn short_sha(sha: &str) -> String {
    sha.chars().take(7).collect()
}

/// 取错误信息首行,避免多行 stderr 刷屏。
fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or(s).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(1);

    struct TestRepo {
        sandbox: PathBuf,
        local: PathBuf,
        remote: PathBuf,
    }

    impl TestRepo {
        fn new() -> Self {
            let id = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
            let sandbox = std::env::temp_dir().join(format!(
                "colna-memo-gitsync-test-{}-{id}",
                std::process::id()
            ));
            std::fs::create_dir_all(&sandbox).unwrap();

            let remote = sandbox.join("remote.git");
            let local = sandbox.join("local");
            run_git(
                &sandbox,
                &["init", "--bare", "--initial-branch=main", path_str(&remote)],
            );
            run_git(&sandbox, &["clone", path_str(&remote), path_str(&local)]);
            configure_repo(&local);

            std::fs::create_dir_all(local.join("memory")).unwrap();
            std::fs::create_dir_all(local.join("src")).unwrap();
            std::fs::write(local.join("memory/base.md"), "base\n").unwrap();
            std::fs::write(local.join("src/app.txt"), "original\n").unwrap();
            run_git(&local, &["add", "memory", "src"]);
            run_git(&local, &["commit", "-m", "initial"]);
            run_git(&local, &["push", "-u", "origin", "main"]);

            Self {
                sandbox,
                local,
                remote,
            }
        }

        fn clone_peer(&self) -> PathBuf {
            let peer = self.sandbox.join("peer");
            run_git(
                &self.sandbox,
                &["clone", path_str(&self.remote), path_str(&peer)],
            );
            configure_repo(&peer);
            peer
        }
    }

    impl Drop for TestRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.sandbox);
        }
    }

    fn path_str(path: &Path) -> &str {
        path.to_str().unwrap()
    }

    fn configure_repo(repo: &Path) {
        run_git(repo, &["config", "user.name", "colna-test"]);
        run_git(repo, &["config", "user.email", "colna-test@example.com"]);
        run_git(repo, &["config", "commit.gpgsign", "false"]);
        run_git(repo, &["config", "core.hooksPath", "/dev/null"]);
    }

    fn run_git(root: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_string()
    }

    #[test]
    fn sync_preserves_local_dirty_source_and_integrates_remote_memory() {
        let repo = TestRepo::new();
        let peer = repo.clone_peer();
        std::fs::write(peer.join("memory/remote.md"), "remote\n").unwrap();
        run_git(&peer, &["add", "memory"]);
        run_git(&peer, &["commit", "-m", "remote memory"]);
        run_git(&peer, &["push"]);

        std::fs::write(repo.local.join("memory/local.md"), "local\n").unwrap();
        std::fs::write(repo.local.join("src/app.txt"), "dirty source\n").unwrap();
        let reindex_calls = Cell::new(0);

        run_sync(&repo.local, "local memory", &|_| {
            reindex_calls.set(reindex_calls.get() + 1);
            Ok(())
        })
        .unwrap();

        assert_eq!(reindex_calls.get(), 1);
        assert_eq!(
            std::fs::read_to_string(repo.local.join("src/app.txt")).unwrap(),
            "dirty source\n"
        );
        assert_eq!(
            run_git(&repo.local, &["status", "--short", "--", "src"]),
            " M src/app.txt"
        );
        assert_eq!(
            run_git(&repo.remote, &["show", "main:memory/local.md"]),
            "local"
        );
        assert_eq!(
            run_git(&repo.remote, &["show", "main:memory/remote.md"]),
            "remote"
        );
        assert_eq!(
            run_git(&repo.remote, &["show", "main:src/app.txt"]),
            "original"
        );
    }

    #[test]
    fn sync_rejects_staged_changes_outside_memory_without_reindexing() {
        let repo = TestRepo::new();
        std::fs::write(repo.local.join("memory/local.md"), "local\n").unwrap();
        std::fs::write(repo.local.join("src/app.txt"), "staged source\n").unwrap();
        run_git(&repo.local, &["add", "src/app.txt"]);
        let reindex_calls = Cell::new(0);

        let error = run_sync(&repo.local, "local memory", &|_| {
            reindex_calls.set(reindex_calls.get() + 1);
            Ok(())
        })
        .unwrap_err();

        assert!(error.to_string().contains("memory/ 外"));
        assert_eq!(reindex_calls.get(), 0);
        assert_eq!(
            run_git(&repo.local, &["diff", "--cached", "--name-only"]),
            "src/app.txt"
        );
        assert_eq!(
            run_git(&repo.local, &["log", "-1", "--pretty=%s"]),
            "initial"
        );
    }

    #[test]
    fn fetch_failure_stops_before_rebase_reindex_and_push() {
        let repo = TestRepo::new();
        let missing_remote = repo.sandbox.join("missing.git");
        run_git(
            &repo.local,
            &["remote", "set-url", "origin", path_str(&missing_remote)],
        );
        std::fs::write(repo.local.join("memory/local.md"), "local\n").unwrap();
        let reindex_calls = Cell::new(0);

        let error = run_sync(&repo.local, "local memory", &|_| {
            reindex_calls.set(reindex_calls.get() + 1);
            Ok(())
        })
        .unwrap_err();

        assert!(error.to_string().contains("已停止 rebase/reindex/push"));
        assert_eq!(reindex_calls.get(), 0);
        assert_eq!(
            run_git(&repo.local, &["log", "-1", "--pretty=%s"]),
            "local memory"
        );
        assert_eq!(
            run_git(&repo.remote, &["log", "-1", "--pretty=%s"]),
            "initial"
        );
    }

    #[test]
    fn autostash_restore_conflict_stops_before_reindex_and_push() {
        let repo = TestRepo::new();
        let peer = repo.clone_peer();
        std::fs::write(peer.join("src/app.txt"), "remote source\n").unwrap();
        run_git(&peer, &["add", "src/app.txt"]);
        run_git(&peer, &["commit", "-m", "remote source"]);
        run_git(&peer, &["push"]);

        std::fs::write(repo.local.join("memory/local.md"), "local memo\n").unwrap();
        std::fs::write(repo.local.join("src/app.txt"), "local dirty source\n").unwrap();
        let reindex_calls = Cell::new(0);

        let error = run_sync(&repo.local, "local memory", &|_| {
            reindex_calls.set(reindex_calls.get() + 1);
            Ok(())
        })
        .unwrap_err();

        assert!(error.to_string().contains("autostash"));
        assert_eq!(reindex_calls.get(), 0);
        assert_eq!(
            run_git(&repo.local, &["diff", "--name-only", "--diff-filter=U"]),
            "src/app.txt"
        );
        assert_eq!(
            run_git(&repo.remote, &["log", "-1", "--pretty=%s"]),
            "remote source"
        );

        let second_error = run_sync(&repo.local, "second sync", &|_| Ok(())).unwrap_err();
        assert!(second_error.to_string().contains("未解决的 Git 冲突"));
    }

    #[test]
    fn rebase_conflict_blocks_retry_and_abort_restores_local_changes() {
        let repo = TestRepo::new();
        let peer = repo.clone_peer();
        std::fs::write(peer.join("memory/base.md"), "remote memo\n").unwrap();
        run_git(&peer, &["add", "memory/base.md"]);
        run_git(&peer, &["commit", "-m", "remote memo"]);
        run_git(&peer, &["push"]);

        std::fs::write(repo.local.join("memory/base.md"), "local memo\n").unwrap();
        std::fs::write(repo.local.join("src/app.txt"), "local dirty source\n").unwrap();
        let reindex_calls = Cell::new(0);

        let error = run_sync(&repo.local, "local memory", &|_| {
            reindex_calls.set(reindex_calls.get() + 1);
            Ok(())
        })
        .unwrap_err();

        assert!(error.to_string().contains("rebase 冲突"));
        assert_eq!(reindex_calls.get(), 0);
        assert_eq!(
            run_git(&repo.local, &["diff", "--name-only", "--diff-filter=U"]),
            "memory/base.md"
        );

        let second_error = run_sync(&repo.local, "second sync", &|_| Ok(())).unwrap_err();
        assert!(second_error.to_string().contains("未完成的 Git 操作"));

        run_git(&repo.local, &["rebase", "--abort"]);
        assert_eq!(
            std::fs::read_to_string(repo.local.join("memory/base.md")).unwrap(),
            "local memo\n"
        );
        assert_eq!(
            std::fs::read_to_string(repo.local.join("src/app.txt")).unwrap(),
            "local dirty source\n"
        );
        assert_eq!(
            run_git(&repo.local, &["log", "-1", "--pretty=%s"]),
            "local memory"
        );
        assert_eq!(
            run_git(&repo.remote, &["log", "-1", "--pretty=%s"]),
            "remote memo"
        );
    }
}

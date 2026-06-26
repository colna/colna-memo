---
title: colna sync 排错
date: 2026-06-25
tags: troubleshooting, colna-memo, git
---

# colna sync 排错

## pull 被拒:cannot pull with rebase: You have unstaged changes

- **现象**:`colna sync` 输出 `(跳过 pull:... cannot pull with rebase: You have unstaged changes)`,远端未真正拉取对齐。
- **根因**:旧版 `run_sync` 顺序是「pull --rebase → reindex → add/commit → push」。`git pull --rebase` 要求工作区干净,而此时 memory/ 的本地改动还没 commit,rebase 被拒;代码用 `match ... Err => 友好提示` 把失败吞掉,所以只是"跳过 pull",不报错。
- **修法**(`src/gitsync.rs` run_sync):
  1. 把 **add + commit memory/ 提到 pull 之前**,保证 pull 时工作区干净。
  2. pull 加 **`--autostash`**(`git pull --rebase --autostash`),兜底任何残留的非 memory 未提交改动(如源码改动),rebase 后自动恢复。
  3. reindex 由两次合并为一次,放在 commit + pull 之后(真源状态稳定再建索引)。
- **教训**:`git pull --rebase` 对脏工作区零容忍;封装同步命令时「先提交本地、再拉远端」是更安全的顺序,`--autostash` 是廉价兜底。

## 提交前冲突标记守卫(2026-06-26 新增)

- **背景**:一次 rebase 半途 `rebase --continue` 把带 `<<<<<<<`/`>>>>>>>` 冲突标记的 Daily 文件提交进了版本库,`colna sync` 不校验、照常 push 出去,污染真源。
- **修法**(`src/gitsync.rs`):新增 `check_conflict_markers()`,在 reindex 后、`git add`+commit 前调用。用 `git grep -n --untracked -e '^<<<<<<<' -e '^>>>>>>>' -- memory`(覆盖未跟踪文件);退出码 0=命中→`bail!` 中止(未提交未 push)、1=干净、其它=真错。不能复用 `git()` helper,因为 git grep 无匹配时退出码 1 会被误判为失败。
- **教训**:封装 sync 命令要在「写入版本库前」做一道脏数据守卫;冲突标记只查 `<<<<<<<`/`>>>>>>>`(distinctive),不查 `=======`(markdown setext 标题误伤)。

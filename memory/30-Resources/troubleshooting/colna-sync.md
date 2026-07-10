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

## colna-memo 的提交作者不是 colna(2026-07-09 发现,未修)

- **现象**:`colna sync` 产生的提交,作者是 `MacBook <max@MacBooks-MacBook-Air.local>`,不是 colna。最近 12 个提交全是。
- **根因**:`colna-memo` 仓库**没有配 `user.name` / `user.email`**,回落到系统默认。

  | 仓库 | 身份来源 | 生效作者 |
  |---|---|---|
  | `sitin-next` | 自己的 `.git/config` | `colna <richardzhang1999@163.com>` ✓ |
  | `colna-memo` | 无配置 → 系统默认 | `MacBook <max@…>` ✗ |

- **`CLAUDE.md` 描述的机制不存在**:它说本工作区靠 `~/.gitconfig` 的 `includeIf` → `zhangzheng/.gitconfig` 统一身份。
  实际 `~/.gitconfig` 里**只有** `includeIf "gitdir:~/Dev2/buchuan/"` 一条,**没有 zhangzheng**。
  `sitin-next` 之所以正确,是因为身份直接写死在它自己的 `.git/config` 里,与 includeIf 无关。
- **已修(2026-07-09)**:给 `~/.gitconfig` 补上 —— 真正实现 CLAUDE.md 描述的机制:

  ```gitconfig
  [includeIf "gitdir:~/Dev2/zhangzheng/"]
      path = ~/Dev2/zhangzheng/.gitconfig
  ```

  生效后 `colna-memo` / `sitin-next2` / `sitin-demo-webapp` 都从 `Dev2/zhangzheng/.gitconfig` 取身份;
  `sitin-next` 仍走自己的 `.git/config`(仓库级优先级更高,值相同)。`buchuan` 工作区不受影响。
  另修正 `zhangzheng/.gitconfig` 注释里写错的路径(`/Users/user` → `~`)—— 那正是这次误判的源头。
- **历史提交的作者不改**(需要 filter-branch / rebase 重写历史,得不偿失)。
- **教训**:**「约定写在 CLAUDE.md 里」不等于「机制真的生效」。** 涉及身份/凭据的约定,要用 `git config --show-origin` 之类**查生效值**,别信文档。

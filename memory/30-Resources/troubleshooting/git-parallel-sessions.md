# 两个 agent 会话同时改一个仓库

来源：2026-09-10 改 Luka 划卡列表，撞上另一个会话在同一 worktree 上提交。

## 症状（按出现顺序）

1. `git status` 里有**你没改过**的文件。
2. `git checkout -b` 之后 `git log -1` 是 A，过一会儿变成了 B——而你没提交过。
3. `git rebase` 明明「远端多 1、我多 0」，结果却多出一个你不认识的提交。
4. 新提交的 `Date:` 就是**几秒前**，作者是你，内容你没写过。

第 4 条是最硬的判据：**时间戳对得上但内容对不上，就是有人在并行操作**。
到这一步立刻停手，先 `git reflog -15` 把因果理清楚，别再 rebase / reset。

## 三个连环坑

- **`git stash` 是全仓库共享的一个栈。** 你 stash 走的可能是别的会话正在做的改动，
  它的下一个提交就会缺文件（编译不过）。
- **`checkout -b` 之后 HEAD 是共享的。** 别的会话此刻 `git commit`，提交会落到**你**
  刚建的分支上。
- **`rebase` 会把误落的提交一起搬走**，于是它出现在你的分支历史里。

## 收拾办法

```bash
git reflog -15                       # 先看清谁在什么时候动了 HEAD
git stash pop                        # 把误拿的还回去（对方若已自行补上，pop 后无净变化）
git branch -f <我的分支> origin/<基线>  # 甩掉误落的提交，基线要挑「已推送」的
git worktree add ../<my-wt> <我的分支>  # 之后全程在独立工作树里干活
```

## 预防

- 多会话同仓库，**先 `git worktree add` 再动手**。
- 新 worktree **不继承 `node_modules`**，要单独 `pnpm install`（monorepo 有全局 store，不算慢）。
- 需要暂存时别用裸 `stash`：用一个临时 WIP 提交，或
  `git stash push -u -m "<唯一标签>"` + `git stash apply <sha>`（不要 `pop`）。

## 开 PR 前必查：base 选对了吗

并行会话把基线分支删掉/合并掉之后，你的分支会「悬空」——历史还在，但没有任何
远端分支包含它的基点。这时直接开 PR，会把基线上那一串**别人尚未合并的提交**
一起算成你的改动。

```bash
git branch -r --contains <我的基点>        # 只剩我自己 → 基线没了，别直接开 PR
git log --oneline -1 origin/<候选 base>    # 信息里带 (#NNN) 的通常就是集成分支
git rev-list --count origin/<base>..HEAD   # 领先数 > 我实际的提交数 → 基线选错了
```

摘出自己那几个提交：

```bash
# 先确认我改的文件在两边一致，rebase 不会冲突
git diff --stat origin/<base> <我的基点> -- <我改的文件>     # 空 = 一致
git rebase --onto origin/<base> <我的基点>
git push --force-with-lease
```

**rebase 之后一定重跑一遍 typecheck / lint / test 再推** —— 换了基线，之前那次验证
证明的是另一棵树。

另外：仓库有多条产品线时，`origin/main` 未必是你这条线的 base。
先 `git show origin/main:<你改的文件>`，报 “不存在” 就说明这条线还没合进 main。

## 脏工作区拉取远端，而不碰并行 WIP（2026-09-23 实操）

共享 worktree 有别人未提交的改动时，`git pull --rebase --autostash` 会在 pop 时
与重叠文件冲突，把冲突标记写进别人的 WIP。可先用只读预检确认会冲突
（`git merge-tree --write-tree origin/<br> $(git stash create)`），再走下面这套：

```bash
git fetch origin <branch>
git rev-list --left-right --count HEAD...origin/<branch>
# 本地若只有「与远端等价」的提交（git show <c> | git patch-id 相同），直接：
git reset origin/<branch>            # mixed，不动 worktree，等价提交丢进 reflog
# 远端改过、本地没改的文件 → 恢复到新 HEAD：
git diff --name-status <old-HEAD> origin/<branch> | cut -f2 | sort > /tmp/inc.txt
git status --short | awk '{print $2}' | sort > /tmp/dirty.txt   # 拉取前先存
comm -23 /tmp/inc.txt /tmp/dirty.txt | while read -r p; do git checkout HEAD -- "$p"; done
# 两边都改的文件 → 逐个三方合并，只有干净才写回：
git merge-file -q -p <wip文件> <old-HEAD版本> <新HEAD版本> > /tmp/merged \
  && cp /tmp/merged <文件> || echo "冲突，留给用户"
```

- 关键点：`reset`（不带 `--hard`）只动 HEAD+index，worktree 是 WIP 的唯一副本；
  绕过 autostash 就没有「pop 冲突污染工作区」这一步。
- **判据**：拉取前先把 `git status --short` 存一份到 /tmp，之后靠它对账
  「哪些是原 WIP、哪些是新出现的」，也能立刻发现并行会话**正在**写入的文件。
- 等价 patch 提交（patch-id 相同、parent 不同）在裸 `reset` 下会被丢弃；
  它已在远端，丢的只是本地副本，`reflog` 可追。

## 共享 worktree 里 `git commit` 会捎上并行会话**已 staged** 的文件（2026-09-24 实操）

- **症状**：只 `git add` 了自己的 2 个文件，`git commit` 后 `--stat` 却有 7 个 —— 多出的
  5 个是并行会话已 `git add` 的 WIP（`git status --short` 第一列是 `M`，第二列空）。
- **根因**：`git commit` 提交的是**整个 index**，不只是你刚 `add` 的那几个。
- **未推送前的收拾**（HEAD 已动、index/worktree 原样保留，非破坏性）：

  ```bash
  git reset --soft HEAD~1                    # 撤销提交，index 仍是全部 7 个
  git restore --staged <别人的 5 个文件>       # 把他们移出 index
  git commit -m "..."                        # 只提交自己的
  git add <别人的 5 个文件>                    # 还原他们原来的 staged 状态
  git status --short                         # 对账：应与操作前逐字符一致
  ```

- **预防**：commit 前先 `git status --short | rg '^[MADRC]'` 看 index 里有没有别人的东西；
  或直接用 `git commit -m "..." -- <我改的文件>`（只提交给定路径，index 中其余 staged
  保持 staged；注意它提交的是**工作区版本**，部分暂存过就得走上面那条路）。
- **已推送才发现在里面**：共享分支上别为这个 `rebase --force`；要么留着，要么
  `git revert` 别人那部分并知会对方。

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

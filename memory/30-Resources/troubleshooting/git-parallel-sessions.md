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

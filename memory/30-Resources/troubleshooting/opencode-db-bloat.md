---
title: opencode 数据库膨胀排查与清理
date: 2026-11-23
tags:
  - opencode
  - 磁盘
  - 数据库
  - sqlite
  - troubleshooting
---

## 症状

`~/.local/share/opencode/opencode.db` 异常膨胀（本例 21G），磁盘被占满。

## 根因

- `event` 表是**只增事件日志**：`message.updated` 等事件会反复记录完整消息，历史会话删除后事件不清理（本例 95 万条 ≈17G，而现存会话仅 45 个）。
- SQLite 删除行后文件不缩小（freelist 可能为 0，页面仍是分配状态）。

## 排查

```bash
ls -lh ~/.local/share/opencode/opencode.db*
opencode db "PRAGMA page_count"   # ×4096 估算大小
opencode db "SELECT type, COUNT(*), ROUND(SUM(LENGTH(data))/1048576.0,1) mb FROM event GROUP BY type ORDER BY mb DESC"
opencode db "SELECT COUNT(*) FROM session"
# 注意：dbstat 全表扫描在 20G 级 DB 上会超时，别用
```

## 清理

```bash
# 1. 删旧会话（会连带删除其 event；每个约 3s，可 xargs -P 4 并行）
opencode db "SELECT id FROM session WHERE date(time_updated/1000,'unixepoch','localtime') < 'YYYY-MM-DD'"
opencode session delete <sessionID>

# 2. 收缩数据库（需独占锁，先腾出 ≥ 目标大小的临时空间）
opencode db "VACUUM"

# 3. 截断 WAL
opencode db "PRAGMA wal_checkpoint(TRUNCATE)"
```

## 结果

本例：DB 21G → 3.0G，WAL 3.1G → 36K；617 个旧会话删除、近 7 天 45 个保留。

## 预防

- 定期删旧会话后 VACUUM；仅删除会话不 VACUUM 不会释放磁盘。
- 不要在 opencode 正跑长任务时做 VACUUM（需独占锁，可能 SQLITE_BUSY）。

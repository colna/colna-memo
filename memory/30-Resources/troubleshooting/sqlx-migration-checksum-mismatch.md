---
title: sqlx migration checksum 冲突处理
date: 2026-11-17
tags: troubleshooting, rust, sqlx, postgres, migration
---

# sqlx migration checksum 冲突处理

## 现象

服务启动报：

```
Error: run sqlx migrations
Caused by:
    migration 7 was previously applied but has been modified
```

## 根因

sqlx 把每个 migration 文件的 SHA-384 记录在 `_sqlx_migrations.checksum`。文件在应用后又被编辑（哪怕只改注释/空白），checksum 就对不上。

## 处理

先确认数据库结构与当前 SQL 等价，再二选一：

1. **只改 checksum（推荐，不重跑 DDL）**

```bash
shasum -a 384 crates/onia-server/migrations/0007_automation_settings.sql
docker compose exec -T postgres psql -U onia -d onia -c \
  "update _sqlx_migrations set checksum = decode('<sha384-hex>','hex') where version = 7;"
```

2. **删除记录后重跑（仅当 migration 幂等，如 `ADD COLUMN IF NOT EXISTS`）**

```sql
delete from _sqlx_migrations where version = 7;
```

然后重启服务，让 sqlx 重新应用并写入新 checksum。

## 注意

- 不要无条件改 checksum。必须先在 `information_schema.columns` / `\d` 确认结构确实已应用，否则会跳过真实 DDL。
- 生产库若也应用过旧版本，部署前要做同样的校准，或者把该 migration 改为幂等后删记录重跑。
- 根治：migration 一旦应用就不要改；新变更另起一个 migration。

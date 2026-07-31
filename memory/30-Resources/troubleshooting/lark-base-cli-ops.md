---
title: lark-cli base(多维表格)操作踩坑
date: 2026-07-30
tags: [troubleshooting, lark-cli, base, bitable, 飞书]
---

# lark-cli base(多维表格 / Bitable)操作踩坑

写周报到飞书「前端周会」文档里张峥的内嵌多维表格时踩的坑,汇总备查。真源以 `.claude/skills/lark-base` 为准。

## 命令 / 参数

- **子命令是 `+field-list` / `+record-list` / `+record-get`**,不是 `+list-fields` / `+list-records`(后者报 `unknown subcommand`)。
- **token 参数是 `--base-token`,不是 `--app-token`**。文档内嵌 `<bitable token="..." table-id="...">` 的 `token` 直接当 `--base-token`,`table-id` 当 `--table-id`(孤立 raw token 不用走 `+url-resolve`)。
- **`+record-list` / `+record-get` 默认输出是 markdown 表格,不是 JSON** → 直接 `json.load` 会炸。要 JSON 加 `--format json`;或直接 grep markdown 行(`^\| rec`)解析。`+field-list` 默认是 JSON(结构 `data.fields[]`,字段 `id`/`name`/`type`/`options`)。

## 批量增删改

- **删除**:`+record-delete` 是 high-risk-write,要 `--yes`。多条别用 `--record-id` 重复拼(macOS `paste -sd' '` 拼参会失败、变量空)→ 直接 `--json '{"record_id_list":[...]}'` 最稳。
- **批量建**:`+record-batch-create --json '{"fields":["列1","列2"],"rows":[[v1,v2],...]}'`,rows 按 fields 顺序;**返回 `data.records` 可能为空(看着 created=0)但其实已建成功** → 别信返回计数,重新 `+record-list` 核实。**返回不含新 record_id** → 要拿新记录 id 得重新 list 按任务名反查(建父级再挂子记录时尤其要这步)。
- **批量改**:`+record-batch-update --json '{"record_id_list":[...],"patch":{...}}'`,键是 **`patch` 不是 `fields`**(用 `fields` 报 `Provide patch for records selected by record_id_list`)。patch 是"同值批量":同一份 patch 套到所有目标记录。

## CellValue 写入格式(`lark-base-cell-value.md`)

- 单选 select:选项名字符串 `"已结束"`(多选用数组)。写未知选项平台会**自动新建选项**,别拿近义词乱传。
- datetime:`"YYYY-MM-DD HH:mm:ss"` 字符串最稳。
- **link 关联字段**(如"父记录"层级表):写 `[{"id":"recXXX"}]`;**清空用 `patch:{"字段名":[]}`**(删了父级后子记录 link 会悬空,用这招清干净)。
- 层级表 = 靠一个指向本表的 link 字段(如"父记录")做父子分组;先建父级记录 → list 拿父级 record_id → 建子记录时 link 到父级。

## 周会文档结构(前端周会)

- 「前端周会」文档 = 每人一个 `👤 @姓名` 的 h2,下面三段:**本周工作(内嵌 bitable 多维表格)** / 风险与阻塞 / 下周计划。
- 张峥的「本周工作」表字段:任务(text 主键)、状态(select 未开始/进行中/已结束)、进度(text 如 "100%")、截止时间(datetime)、父记录(link 到本表,做分类分组)、备注(text 放 PR#+要点)。
- 周报按分类写 = 建分类父级(如"一、sitin-next PWA")+ 子记录挂父级。

相关:[[lark-cli-auth-and-perms]]

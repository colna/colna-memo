---
title: 飞书周报 — 张峥模块的填写方式与形式
date: 2026-07-02
tags: [feishu, 周报, 张峥, lark-doc, lark-base, 流程]
---

# 飞书周报（张峥模块）填写方式

每周填张峥的周报:**本周工作 + 风险与阻塞 + 下周计划**。身份用张峥 user(app `cli_a95cafcf9cb9dcd4`,先 `lark-cli auth list` 确认)。团队共享文档,**写前先给用户过目**。

## 周报文档

- URL:`https://presence.feishu.cn/wiki/QLRzwj45SiGF9skJaq3cuKC6nuf`(docx token `VUyFdyhd3ovio6xpfKecRmvBnWe`)。
- 张峥模块 = h2 `👤 @张峥`,下含三个 h3:**本周工作** / **风险与阻塞** / **下周计划**。
- **block id 每周可能变**,每次先 `docs +fetch --scope outline --detail with-ids` 拿最新 id,再定位。

### 三栏的形式(重要)

- **本周工作 = 内嵌多维表格**(不是纯文本!):bitable token `LOS7bfw2BaMRTTsVvq7cZsGTn2c`,table `tble5ZibKHUd7nD3`。字段:`任务`(text)/`状态`(单选:未开始/进行中/已结束)/`进度`(text,如 "80%")/`备注`(text)/`父记录`(link,自关联,做项目分组)/`截止时间`(datetime)。
- **风险与阻塞 = 无序列表**(`<ul><li>`)。
- **下周计划 = checkbox 列表**(`<checkbox done="false">…</checkbox>`)。

## 填写流程

### 1. 本周工作(改内嵌表格,不是改文档正文)

从 colna memo 本周 Daily 汇总。**按项目分组、用「父记录」划分子集**(和历史结构一致):

1. 读旧行:`base +record-list --base-token LOS7bfw2BaMRTTsVvq7cZsGTn2c --table-id tble5ZibKHUd7nD3`,拿到上周 record_id。
2. 删旧行:`base +record-delete … --json '{"record_id_list":[…]}' --yes`。
3. 建**项目父行**(每项目一行,`任务`=项目名,`状态`=汇总态):`base +record-batch-create … --json @parents.json`,`{"fields":["任务","状态"],"rows":[…]}`,**记下返回的 record_id_list**。
4. 建**子任务行**,`父记录` 指向对应父 id:`{"fields":["任务","状态","进度","备注","父记录"],"rows":[[…, [{"id":"rec父id"}]], …]}`。
   - 单选状态写字符串(`"进行中"`);空 cell 写 `null`;`父记录` 是 `[{"id":"rec…"}]`。
   - `--json @file` 的 file 必须是**相对当前目录**路径 → 先 `cd scratchpad` 再 `--json @./x.json`。

> 注意共享表并发:别人可能同时加行,`record-list` 里出现不认识的行别删,只动自己这批。

### 2. 风险与阻塞(改文档 li)

`docs +update --command block_replace --block-id <li的id> --content '<li>…</li>'`。

### 3. 下周计划(改文档 checkbox)

从**下周任务表**读张峥的任务,`block_replace` 掉上周的 checkbox,换成本周的 checkbox 列表(可加 `<p><b>项目名</b></p>` 领起)。

- 下周任务表:`https://presence.feishu.cn/wiki/WZ5HwLiguiLUQHk4UnycuLlPnOd?table=tblSUYX7AgP9bRD5&view=vew6dL6BLV`。
- **坑**:该 wiki 空间**没加应用** `cli_a95cafcf9cb9dcd4`,user/bot 读都 131005 not found。目前张峥直接**截图/贴文字**给任务(如「SITIN 4.0 - 纯真不穿」下的前端需求),我据此填。要 API 直读需先把应用加进该 wiki 空间。

## 相关

- 工作内容真源:`50-Daily/` 本周日报;项目见 [[pwa-verify-tasks]] / [[pwa-chat-input-bar]] / [[sitin-next]]。
- 飞书身份/上线申请约定见项目 `CLAUDE.md`「飞书侧使用规则」。

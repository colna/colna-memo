---
title: 找回 opencode 桌面端会话里用户贴的图（opencode.db）
date: 2026-09-16
tags: [opencode, tooling, image2, troubleshooting]
---

# 找回 opencode 桌面端会话里用户贴的图

用户贴进对话的图片**不在文件系统里**（`~/Downloads` / `/tmp` 都没有），写进了桌面端 SQLite：

- 位置：`~/.local/share/opencode/opencode.db`
- 表：`part`；`data` 列是 JSON，文件类的行满足 `json_extract(data,'$.type')='file'`
- 内容：base64 data URL（`data:image/png;base64,...`）

取最近一张（换成自己的解码方式，字段名以回读为准）：

```bash
sqlite3 ~/.local/share/opencode/opencode.db \
  "select data from part where json_extract(data,'\$.type')='file' order by time_created desc limit 1"
```

把取出的 JSON 里 data URL 的 base64 段解码成 PNG 即可（2026-09-15 Bond sheet / video match 那些参考稿都是这样还原的）。

**别去 `~/Library/Caches`、浏览器 Network 面板里翻** —— 试过，方向是错的。

## 多条 part 时的两个坑（2026-09-20 实战）

- `time_created` 是**毫秒**时间戳，查「最近 N 分钟」用
  `time_created > (strftime('%s','now') - N) * 1000` —— 比全表扫 `json_extract(data,'$.type')='file'`
  快得多（库有 9GB）。
- **不要在一个循环里把多条命中都写进同一个输出文件**：窗口里可能有并行会话刚贴的图，
  互相覆盖后拿到的是别人的图（本次就抽到了别的会话的登录页截图）。要精确复原某一条，
  用时间点框定（`time_created BETWEEN <起点ms> AND <终点ms>`）只取那一条。
- `part.id` 是 `prt_` + 大小写混合随机串（`P`/`p`、`l`/`I` 肉眼难分），手抄 id 很容易查不到
  （返回 NULL）；能按时间/内容过滤就别抄 id。飞书发来的图 `filename` 指向飞书容器路径，
  但 `data` 列里的 base64 data URL 才是可直接解码的图，不必碰飞书目录。

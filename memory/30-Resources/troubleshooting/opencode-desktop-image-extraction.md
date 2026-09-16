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

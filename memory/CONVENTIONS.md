---
title: memory 书写约定
date: 2026-06-25
tags: meta, conventions
---

# memory/ 书写约定

本目录是 colna-memo 的**唯一真源**(走 git 跨设备同步),`.colna/` 索引是派生物、可随时重建。
笔记约定与 Obsidian 知识库规则保持一致。

## 目录结构(PARA + Inbox + Daily)

- `00-Inbox/` — 收件箱,未分类的笔记、剪藏先丢这里,后续整理
- `10-Projects/` — 项目(有目标 + 截止日期)
- `20-Areas/` — 领域(长期关注的方向)
- `30-Resources/` — 资源(书摘、文章、参考资料);可复用排错经验放 `30-Resources/troubleshooting/<topic>.md`
- `40-Archive/` — 归档(完成的、过期的);从 Daily 移出的内容进 `40-Archive/daily-trash/YYYY-MM-DD.md`
- `50-Daily/` — 日记 `YYYY-MM-DD.md`
- `_attachments/` — 图片、PDF 等附件
- `_templates/` — 笔记模板(daily / note / work-log)

## front-matter(YAML)

每篇笔记开头用 `--- ... ---` 包裹,索引只读取以下三个键(`chunker.rs`):

```yaml
---
title: 标题
date: YYYY-MM-DD
tags: 标签1, 标签2
---
```

- `title` 缺省时回退到第一个 H1,再回退到文件名。
- `tags` 用逗号分隔。
- 其余 Obsidian 属性(aliases、cssclasses 等)可写,但不参与索引。

## 切块规则

- 按 Markdown 标题行(`#`、`##`、`###`…)切段,每段一个可检索 chunk。
- chunk 文本 = 标题 + 该段正文;空段跳过。
- 稳定 id = `source_path + heading + 序号` 的 sha256。

## 链接与语法

- 笔记间链接用 Obsidian wikilink:`[[note-name]]` 或 `[[path/note|别名]]`。
- callout、embed 等 Obsidian 特有语法可用,但只有纯文本参与向量/FTS 检索。

## 操作约定

- `colna add` 默认把新笔记落到 `00-Inbox/`;明确指定分类时放对应目录。
- 写完笔记自动增量重建索引(`colna add` / `colna sync` 内置)。
- **归档不是删除**:从 Daily 移出的内容进 `40-Archive/daily-trash/`,永不 `rm`。
- 不在 `memory/` 之外乱放真源;附件统一进 `_attachments/`。

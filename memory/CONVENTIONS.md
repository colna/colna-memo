---
title: memory 书写约定
date: 2026-06-25
tags: meta, conventions
---

# memory/ 书写约定

本目录是 colna-memo 的**唯一真源**(走 git 跨设备同步),`.colna/` 索引是派生物、可随时重建。
所有内容是普通 Markdown(PARA + Inbox + Daily 组织)。

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
- 其余 front-matter 属性可写,但不参与索引。

## 切块规则

- 按 Markdown 标题行(`#`、`##`、`###`…)切段,每段一个可检索 chunk。
- chunk 文本 = 标题 + 该段正文;空段跳过。
- 稳定 id = `source_path + heading + 序号` 的 sha256。

## 时间口径(重要)

- **自 2026-07-31 起,所有 memo 的日期与时间戳一律记「北京时间」(UTC+8)。** Daily 文件名 `YYYY-MM-DD`、工作日志 `HH:MM` 均按北京时间。
- 本机时钟是**美国中部时间**(夏令时 CDT=UTC-5,冬令时 CST=UTC-6),不是北京时间。写入前取北京时间用:
  `TZ=Asia/Shanghai date "+%Y-%m-%d %H:%M"`(不要直接用裸 `date`)。
- **历史换算**:**≤2026-07-30** 的所有 Daily 日期与 `HH:MM` 是**机器本地(美中)时间**,原样保留、不回改。换算成北京时间:
  - 2026 年 6~7 月(夏令时 CDT):**+13 小时**(可能跨天,如美中 `22:30` = 北京次日 `11:30`)。
  - 冬令时(CST)时段:+14 小时。
- 之所以历史不统一改:绝大多数历史条目只有日期、没有 `HH:MM`,无法可靠判定跨天重定日期,机械改会破坏真源。

## 链接

- 笔记间链接用相对路径 Markdown 链接:`[别名](path/note.md)`。

## 操作约定

- `colna add` 默认把新笔记落到 `00-Inbox/`;明确指定分类时放对应目录。
- 写完笔记自动增量重建索引(`colna add` / `colna sync` 内置)。
- **归档不是删除**:从 Daily 移出的内容进 `40-Archive/daily-trash/`,永不 `rm`。
- 不在 `memory/` 之外乱放真源。
- 通用附件(截图、PDF、外来素材、无 md 上下文的资源)统一进 `_attachments/`。
- 项目自持的可视化 HTML(与同名 md 姐妹,如 `10-Projects/xxx/foo.md` 配 `foo.html`)可就近放项目目录下;
  chunker 只索引 md,HTML 无论放哪都不会被切段。就近的好处是维护/引用/搬迁一起走。

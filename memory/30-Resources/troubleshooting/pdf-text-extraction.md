---
title: 模型读不了 PDF → pdftotext（简历/PDF 文本提取）
date: 2026-10-14
tags: [troubleshooting, pdf, tooling, 简历]
---

# 模型读不了 PDF → pdftotext

**现象**：会话里直接 Read 用户发的 PDF 报 `this model does not support pdf input`（2026-10-13、10-14 读候选人简历连续踩到）。

**修法**（本机已装 poppler）：

```bash
pdftotext -layout "<file.pdf>" /tmp/out.txt   # -layout 保留版式，简历类最佳
```

- 提取后按普通文本 Read 即可。
- 备用：`python3 -c "import fitz"`（pymupdf 已装）可转文本/图片；实在不行用 `sips` / `qlmanage` 转 PNG 再按图片读。
- 坑：带水印的导出简历（如飞书招聘导出）文本里会混入水印串（`44963b7f…` 之类），忽略即可。

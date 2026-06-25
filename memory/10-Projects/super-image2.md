---
title: super-image2 (SuperImage)
date: 2026-06-25
tags: project, ai, image-generation, react, turborepo, gpt-image
---

# super-image2 (SuperImage)

基于 GPT Image 2 的文生图对话平台,带无损图像管线。

## 基本信息

- 仓库:`git@github-colna:colna/super-image2.git`(个人 colna 名下)
- 路径:`/Users/user/Dev2/zhangzheng/super-image2`
- 当前分支:`main`
- monorepo:Turborepo + pnpm workspaces

## 核心特性

- **无损管线**:`response_format: "b64_json"` 拿原始 PNG → Blob → IndexedDB 零压缩存储
- **对话式工作流**:聊天式生图,每个 session 保留完整 prompt / 参数 / 结果历史
- **图像编辑模式**:对任意生成图「Edit」开启链式编辑(每次基于上一结果)
- **参考图附件**:clip 按钮 / 拖拽 / 粘贴 → 送入 `/images/edits` 作参考
- **文件附件**:附 .txt/.md/.json/.csv 等丰富 prompt
- **隐私优先**:API Key 与图像数据仅存本地(IndexedDB + localStorage),不发第三方
- **多 session 管理、双语 UI、全局快捷键、响应式布局、Lightbox**

## 备注

- 双语 README(English / 中文),自带 `docs/`。

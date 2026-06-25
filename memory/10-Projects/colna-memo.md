---
title: colna-memo
date: 2026-06-25
tags: project, rust, knowledge-base, zvec, embedding, mcp, para
---

# colna-memo

跨设备个人知识库:**Git 存 Markdown 为唯一真源 + zvec 本地语义索引**。本目录的 `memory/` 既是这个项目的产物,也是知识库本体;此笔记记录的是**项目/工具本身**(与知识库内容区分)。

## 基本信息

- 仓库:`git@github-colna:colna/colna-memo.git`(个人 colna 名下)
- 路径:`/Users/user/Dev2/zhangzheng/colna-memo`
- 当前分支:`main`
- 语言:Rust

## 架构

- **Git 为唯一真源**:所有内容是 `memory/` 下 Markdown,走 git 跨设备同步。
- **zvec 为派生索引**:每台设备本地建语义向量索引,`.colna/` 不入 git,可随时 `colna index` 重建。
- 绕开嵌入式向量库"单进程写"限制,无多设备写同一 DB 冲突。

## 技术栈

- [zvec](https://github.com/zvec-ai/zvec-rust) v0.5.0 — 进程内向量库(git 依赖)
- [fastembed](https://github.com/Anush008/fastembed-rs) — 本地 embedding,模型 `multilingual-e5-small`(384 维,中英多语言,离线)
- 检索:向量 + FTS 混合(RRF 融合)
- MCP server(stdio)供 Claude 调用 `kb_search` / `kb_get`

## CLI(`./colna` 包装脚本,处理 zvec 动态库 rpath)

| 命令 | 作用 | 推远端 |
|------|------|--------|
| `index` | 扫描 memory/ 增量重建索引 | 否 |
| `search` | 语义 / 混合检索 | 否 |
| `mcp` | 以 MCP server(stdio)运行 | 否 |
| `add` | 新建笔记到 `00-Inbox/` + 自动 reindex | 否 |
| `sync` | git pull --rebase → reindex → commit → push | **是** |

## 关键约定

- 知识库内容采用 PARA + Inbox + Daily,书写约定见 `memory/CONVENTIONS.md`。
- `_` 开头目录(`_templates/` `_attachments/`)不入索引。
- 构建:`cargo build`(首次下载 zvec 预编译库);跑二进制需 `DYLD_LIBRARY_PATH`,用 `./colna` 自动处理。
- **改完 memory/ 即 `colna sync` 同步到远端**(工作区规则授权的自动动作)。

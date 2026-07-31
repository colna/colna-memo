---
title: colna-memo
date: 2026-06-25
tags: project, rust, knowledge-base, zvec, embedding, mcp, para
---

# colna-memo

跨设备个人知识库:**Git 存 Markdown 为唯一真源 + zvec 本地语义索引**。本目录的 `memory/` 既是这个项目的产物,也是知识库本体;此笔记记录的是**项目/工具本身**(与知识库内容区分)。

## 基本信息

- 仓库:`git@github.com:colna/colna-memo.git`(使用本机 SSH 配置)
- 路径:`/Users/colna/WORK/colna-memo`
- 当前分支:`main`
- 语言:Rust

## 架构

- **Git 为唯一真源**:所有内容是 `memory/` 下 Markdown,走 git 跨设备同步。
- **zvec 为派生索引**:每台设备本地建语义向量索引,`.colna/` 不入 git,可随时 `colna index` 重建。
- **state v2 做完整性证明**:记录文件 hash、每文件 chunk IDs 和总数;增量前核对 zvec doc_count / index completeness,不可信即全量恢复。
- **有界索引**:embedding 与写入按 16 chunks 串行批处理,写完执行 zvec optimize。
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
| `sync` | commit memory → pull --rebase --autostash → reindex → push | **是** |

## 关键约定

- 知识库内容采用 PARA + Inbox + Daily,书写约定见 `memory/CONVENTIONS.md`。
- `_` 开头目录(`_templates/` `_attachments/`)不入索引。
- 构建:`cargo build`(首次下载 zvec 预编译库);跑二进制需 `DYLD_LIBRARY_PATH`,用 `./colna` 自动处理。
- **改完 memory/ 即 `colna sync` 同步到远端**(工作区规则授权的自动动作)。

## 2026-07-31 索引 / 同步修复

- `fastembed 4.9.1` 默认把最多 256 条分成多个 Rayon 并发 ONNX batch,每个 Session 又使用全部 CPU 线程;1355 chunks 曾达到约 29.5 GB footprint / 28.6 GB swap。
- 应用层改为每批 16 chunks 串行,并显式传 `Some(batch.len())`;真实全库 108 files / 1355 chunks 用时 107.35 秒,最大 RSS 2.04 GB,swap 0。
- 全量中断先清 state;成功前校验写入计数、zvec doc_count、schema 中 `text` / `embedding` 索引存在性和 vector completeness,防止部分 index 被误报为完成。zvec v0.5.0 的 stats 只列向量索引,FTS 存在性需通过 schema 单独检查。
- zvec v0.5.0 的 `delete_by_filter` 在本 schema 的优化后 collection 上成功返回但不删文档;state v2 改存 chunk IDs,增量按主键幂等删除。
- `sync` 在 pull 前后都检查 Git 中间态 / unmerged;autostash 恢复冲突即停止,不再 reindex / push。

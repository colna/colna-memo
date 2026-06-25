---
title: 欢迎使用 colna-memo
date: 2026-06-25
tags: meta, getting-started
---

# colna-memo 是什么

colna-memo 是一个跨设备的个人知识库,用来记录 Claude 的上下文与个人笔记。

## 设计理念

- **Git 为唯一真源**:所有内容以 Markdown 存放在 `memory/` 目录,通过 git 跨设备同步。
- **zvec 为派生索引**:每台设备在本地用 zvec 构建语义向量索引,索引文件不入 git,可随时重建。
- **不存在多设备写同一 DB**:绕开嵌入式向量库单写的限制。

## 如何使用

1. 在 `memory/` 下新增或编辑 Markdown 文件。
2. 运行 `colna index` 构建/更新本地语义索引。
3. 运行 `colna search "你的问题"` 做语义检索。

## embedding 模型

默认使用本地 multilingual-e5-small 模型(384 维),中英文都支持,离线可用,数据不外传。

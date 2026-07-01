---
title: MCP servers 配置与使用（Claude Code）
date: 2026-07-01
tags: [mcp, claude-code, tooling, figma]
---

# MCP servers 配置与使用（Claude Code）

本工作区（`/Users/max/Dev2/zhangzheng`）里给 Claude Code 配置的 MCP server 记录。

> 安全约定:**带密钥的 MCP 一律用 `--scope local`**,配置落在本机 `~/.claude.json`,密钥不进任何仓库、也不写进本笔记。本笔记只记「怎么配、怎么用」,不记明文 key。

## 通用操作

- **添加(stdio)**:`claude mcp add <name> --scope <local|project|user> -- <启动命令与参数>`
  - `--` 之后是 server 的启动命令;`--` 之前是 Claude 自己的参数。
  - scope:`local` = 只本机本项目(写 `~/.claude.json`);`project` = 写进仓库 `.mcp.json`(团队共享,**别放密钥**);`user` = 你所有项目。
- **添加(远程)**:`claude mcp add --transport <http|sse> <name> <url>`
- **查看/删除/连通性**:`claude mcp get <name>` / `claude mcp remove <name>` / `claude mcp list`(list 会实拉起测连通,显示 ✔ Connected)。
- **生效时机(重要)**:MCP 在**会话启动时**加载,**新配的 MCP 不会热加载进当前运行中的会话**;要 `/reset` 或开新会话,其工具才出现在工具列表。MetaBot 是长驻会话,尤其注意。

## Figma —— `figma-developer-mcp`

- **配置命令**:
  ```bash
  claude mcp add figma --scope local -- \
    npx -y figma-developer-mcp --figma-api-key=<FIGMA_API_KEY> --stdio
  ```
  - key(`figd_…`)在本地 `~/.claude.json`,不入库。
- **用法**:reset 生效后,给一个 Figma 文件/frame/节点 URL(`https://www.figma.com/design/<key>/...?node-id=<id>` 或 `/file/`),让它抽取该设计的布局/样式/组件结构。
  - 主要工具:`get_figma_data`(取设计数据)、`download_images`(下载图片素材)。
  - **选中具体 frame 的 `node-id`** 比整文件更省 token、更准。
- **典型场景**:按 Figma 稿实现/还原 UI,配合 `.claude/skills` 里的 `frontend-design` / `apple-design`。

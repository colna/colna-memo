# colna-memo

跨设备的个人知识库,用来记录 Claude 的上下文与个人笔记。

## 架构

```
        ┌──────────── Git 远端(唯一真源) ────────────┐
        │   memory/  (Markdown: 笔记 / 对话 / 文档)      │
        └──────────────────────────────────────────────┘
              ↑ git push/pull(跨设备同步靠这个)
   设备A 本地                          设备B 本地
   ├ memory/        (md 真源)         ├ memory/
   ├ .colna/index.zvec ← 本地派生索引  ├ .colna/index.zvec ← git pull 后重建
   └ colna (CLI)                       └ colna (CLI)
```

- **Git 为唯一真源**:所有内容是 `memory/` 下的 Markdown,走 git 跨设备同步。
- **zvec 为派生索引**:每台设备本地用 [zvec](https://github.com/alibaba/zvec) 建语义向量索引,
  索引文件 `.colna/` **不入 git**,可随时由 `colna index` 重建。
- 这样绕开了嵌入式向量库"单进程写"的限制,不存在多设备写同一 DB 的冲突。

## 技术栈

- Rust
- [zvec](https://github.com/zvec-ai/zvec-rust) v0.5.0 — 进程内向量数据库(git 依赖)
- [fastembed](https://github.com/Anush008/fastembed-rs) — 本地 embedding,模型 `multilingual-e5-small`(384 维,中英多语言,离线、数据不外传)

## 构建

```bash
cargo build            # 首次会下载 zvec 预编译库
```

## 使用

由于 zvec 预编译动态库的 rpath 限制,直接跑二进制需要设 `DYLD_LIBRARY_PATH`。
仓库提供了 `./colna` 包装脚本自动处理:

```bash
./colna index                      # 扫描 memory/ 全量重建本地语义索引
./colna search "怎么跨设备同步"      # 语义检索
./colna search "xxx" --topk 8       # 指定返回条数
```

## 目录

```
memory/            真源 Markdown(入 git)
  notes/           笔记
.colna/            本地 zvec 索引(不入 git)
src/
  chunker.rs       Markdown 切块 + front-matter 元数据
  embedder.rs      fastembed 本地 E5 向量
  store.rs         zvec 封装(建库/写入/检索)
  main.rs          CLI(index / search)
colna              运行包装脚本(自动设置 dylib 路径)
```

## 路线图

- [x] **P0**:md → 切块 → 本地 embedding → zvec → CLI 语义检索
- [ ] **P1**:增量索引(按 mtime/hash 只重嵌入变化块)+ FTS 混合检索
- [ ] **P2**:MCP server,Claude 直接调用 `kb_search` / `kb_get`
- [ ] **P3**:`colna add` / `colna sync`(封装 git add/commit/push/pull + 自动 reindex)
```

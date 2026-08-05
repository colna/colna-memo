---
title: colna reindex 维度不匹配 / sync 死循环 排错
date: 2026-08-04
tags: [colna, troubleshooting, embedding, zvec]
---

# colna `dimension mismatch, expected 768 but got 384`

## 现象
- `colna sync` / `colna index` 报 `zvec error InvalidArgument: ... field[embedding] dimension mismatch, expected 768 but got 384`。
- 连带 `git pull --rebase` 被 `error: cannot pull with rebase: You have unstaged changes` 跳过,memo 一直推不上远端。

## 根因
`./colna` 包装脚本默认跑 **debug** 二进制(`COLNA_PROFILE:-debug` → `target/debug/colna`)。
`src/embedder.rs` 的 embedding 模型/维度升级过(e5-small=384 → e5-base=768,commit 68c0869),
索引也重建成 768,但**二进制没重编**,仍是旧构建 → 运行时产 384 向量塞进 768 索引 → 报错。

死循环:reindex 挂 → `colna sync` 不 commit → 当天 daily 笔记留在未暂存 → 反过来挡住下次 `pull --rebase`。

## 修法
```bash
cd colna-memo
export PATH="$HOME/.cargo/bin:$PATH"   # 后台/精简 shell 常无 cargo
cargo build            # 刷新 debug(包装脚本默认用它)
cargo build --release  # 若平时用 COLNA_PROFILE=release,也要一起重编
./colna index          # 增量重建,确认无维度报错
./colna sync           # pull→reindex→commit→push
```

## 要点
- **改了 `src/embedder.rs` 的模型/维度,必须 `cargo build` 重编二进制**;debug 与 release 分别重编。
- `.colna`(索引)、`.fastembed_cache`(模型缓存)都是 **git 忽略的本地派生物**,重建/删除不影响 git 真源(memory/ 的 Markdown)。彻底重建可删 `.colna` 后 `colna index`。
- 判断二进制是否过期:`ls -la target/debug/colna` 的时间 vs `git log -1 -- src/embedder.rs`。

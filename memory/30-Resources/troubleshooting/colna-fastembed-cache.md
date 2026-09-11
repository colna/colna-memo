# colna 的模型缓存会落在「当前目录」，不是知识库

**症状**：某个无关仓库（如 `sitin-rn/`）根目录突然多出一个未跟踪的 `.fastembed_cache/`，1.1 GB；
`git status` 从此一直脏。KB 根自己也有一份（1.5 GB，含 e5-base 与更早的 e5-small）。

**根因**：colna 的语义向量走 `fastembed`（Rust crate；`src/embedder.rs` 用
`EmbeddingModel::MultilingualE5Base`，768 维、中英多语言）。fastembed 4.9.1 的 `src/common.rs`：

```rust
const DEFAULT_CACHE_DIR: &str = ".fastembed_cache";
pub fn get_cache_dir() -> String {
    std::env::var("FASTEMBED_CACHE_DIR").unwrap_or(DEFAULT_CACHE_DIR.into())
}
```

`DEFAULT_CACHE_DIR` 是**相对路径**，按**进程 cwd** 解析；colna 自己没设 `FASTEMBED_CACHE_DIR`
（`grep -rn FASTEMBED src/` 无命中）。于是**在哪个目录调 colna，模型就下到哪个目录** ——
用绝对路径 `/Users/colna/WORK/colna-memo/colna search …` 也一样：绝对路径只决定跑哪个二进制，
不决定缓存位置。

**代价**：一次错位的调用会重下 1.0 GB 权重 + 17 MB tokenizer（2026-09-11 那次 18:19→18:22 三分钟
下完），而且**跨 cwd 不复用** —— 每个目录各下一份。缓存本身可再生，删掉不丢数据，只是下次在那个
目录再跑又要重下。

**处置**

- 删掉杂散那份：`rm -rf <repo>/.fastembed_cache`（先确认里面只有可再生的模型缓存）。
- 想在任意 cwd 调 colna 又不落盘：`export FASTEMBED_CACHE_DIR=~/.cache/fastembed`；更彻底的是
  写进 `colna` 包装脚本一处生效（未做，改前要确认）。
- KB 的 `.gitignore` 已忽略 `/.fastembed_cache` 与 `.fastembed_cache/`；别的仓库没有这条，才会
  显示成未跟踪。
- sitin-rn 也补上了（2026-09-11，`.gitignore` 的 `# Tooling caches` 段、`.turbo/` 旁边）。
  **换个仓库做题时冒出 `?? .fastembed_cache/`**：先 `git check-ignore -v .fastembed_cache`，
  没命中就补一行 ignore；**别 `rm`** —— 那是可再生的模型缓存，删了下次还要重下 1 GB。

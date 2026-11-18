---
title: dotenvy 遇到未加引号的值会中断后续变量加载
date: 2026-11-18
tags: dotenvy, rust, env, config, troubleshooting
---

# dotenvy 遇到未加引号的值会中断后续变量加载

**场景**：Onia 本地 `.env` 新增 `ONIA_GOOGLE_CLIENT_IDS` 后，重启 `onia-server`，`/v1/auth/google` 仍返回 422「google sign-in is not configured」，但同一变量用 shell 显式导出就正常。

**坑**：

- `.env` 里原有 `ONIA_EMAIL_FROM=Onia <no-reply@onia.asia>`，值含空格和尖括号且未加引号。
- `dotenvy::from_path(".env")` 直接返回 `Error parsing line: 'Onia <no-reply@onia.asia>', error at line index: 5`。
- 项目里的 `apps/server/src/main.rs` 是 `let _ = dotenvy::from_path(&candidate);`，错误被忽略；服务照常启动（前面的 `DATABASE_URL` / `ONIA_JWT_SECRET` 等已加载），但该行之后的变量全部没生效，很难从日志发现。

**根因**：dotenvy 解析失败后不会继续解析剩余行；服务能启动不代表 `.env` 全部加载成功。

**修法**：

- 给含空格/特殊字符的值加引号：`ONIA_EMAIL_FROM="Onia <no-reply@onia.asia>"`。
- 排查时用一个最小 Rust 程序只调 `dotenvy::from_path` 并检查目标变量：

```rust
fn main() {
    match dotenvy::from_path(".env") {
        Ok(()) => println!("loaded .env"),
        Err(e) => println!("dotenv error: {e}"),
    }
    println!("{:?}", std::env::var("ONIA_GOOGLE_CLIENT_IDS").map(|v| v.len()));
}
```

- 或把 `load_dotenv` 改成失败时打印警告，而不是静默忽略。

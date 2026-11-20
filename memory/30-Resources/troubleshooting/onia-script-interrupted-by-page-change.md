---
title: Onia 脚本执行被 WebView 刷新/跳转中断导致 Todo 阻塞
date: 2026-11-20
tags: onia, webview, script, behavior, troubleshooting
---

# 现象

发帖/发快拍等脚本执行中，用户刷新 Instagram 页面或发生跳转；设备端没有回写 `behavior.result`，服务端 Todo 一直 `running`，后续同动作 Todo 因 `peer_conflict` 一直等待。

# 根因

1. WebView 文档级导航会销毁脚本 JS 上下文；正在 await 的 Promise 不会回包。
2. `_waitForResumeReady` 在非终态但 `scriptReady=false` 时没有硬超时，可能无限等。
3. 服务端没有 running behavior 的兜底超时；设备/进程异常时 Todo 永久占用 peer mutex。

# 修复（2026-11-20）

- `InstagramSession.onPageStarted`：若存在未完成的 RPC 且不是“脚本主动导航后等待 resume”，立即以 `script` 错误中止 pending，不再等 45s/无限。
- `DeviceRuntime._waitForResumeReady`：增加 20s 硬超时；超时且非终态返回 `scriptInterruptedByPageChange`，Todo 失败而非阻塞。
- `pipeline::expire_stale_behaviors`：scheduler 每 15s 扫描创建超过 10 分钟仍 `dispatched/running` 的 behavior，标记 failed 并同步 Todo failed，然后重新调度 user。
- 仍保留原有 checkpoint/pending 机制处理脚本主动导航。

# 注意

- 页面刷新导致的执行失败不会自动重试（error_class=script），需要人工在追踪页重试。
- 设备/生产服务需要更新到包含上述代码的版本；旧版本仍可能复现。

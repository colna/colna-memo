---
title: social-proxy-scripts-container-app
date: 2026-06-25
tags: project, tauri, rust, android, instagram, snapchat, automation
---

# social-proxy-scripts-container-app

基于 Tauri 2 的脚本容器应用(桌面 + Android)。在应用内加载社交平台网页(Instagram / Snapchat),并把 `scripts/` 目录下的 JS 脚本注入页面执行。

> ## ⚠️ 这是**测试工装**,与线上业务无关(2026-07-16 用户确认)
>
> **不要把它写进线上 SP 架构图,也不要在讨论线上方案时把它和 `GraceChat-Earn-Android` 并列。**
>
> - **线上唯一的 SP 容器是 `GraceChat-Earn-Android`(haven)** —— `haven/src/main/java/com/harbor/prod/socialproxy/`,含 `AndroidBridge` / `WebViewManager` / `SocialProxyWSClient`(WebSocket 连 sp-server)。
> - 本项目只用来**开发/调试注入脚本**,不接 sp-server、不参与线上 `Signal → Todo → Behavior` 管道。
> - 因此 FCM / FGS / Android 进程冻结 / 多社媒 WebView 内存方案([[android-webview-multi-social-memory]])这些约束**只对 haven 成立**,与本项目无关。
>
> **踩坑记录**:2026-07-16 我曾因本地没 clone 到它(见下"本地状态"),先误判整条笔记是幻觉、要删;实际它真实存在且有 06-25 / 07-09 多处一手记录。**真正的问题不是它不存在,而是这条笔记当时缺"测试专用"这个标签**,导致它被误当成线上架构的一部分。

## 基本信息

- 仓库:`git@github-colna:presence-io/social-proxy-scripts-container-app.git`
- 当前分支:`main`,版本 `0.1.0`
- **本地状态**:`/Users/max/Dev2/zhangzheng/social-proxy-scripts-container-app` **只是个空目录,从未 clone**。需要时:
  ```bash
  git clone git@github-colna:presence-io/social-proxy-scripts-container-app.git
  ```

## 项目结构

```
ui/                  # 前端页面(纯 HTML/JS,无构建步骤)
  index.html         # 主界面:平台选择、WebView、控制面板、日志
scripts/             # 待注入的 JS 脚本(每个 .js 对应一个注入按钮)
  example-hello.js
src-tauri/           # Tauri Rust 后端
  src/lib.rs         # list_scripts 命令:读取 scripts/ 下所有 .js
  tauri.conf.json    # 打包时将 scripts/ 作为资源内置
```

## 环境

- Node.js + pnpm
- Rust 工具链(rustup)
- Android 额外:Android Studio、`ANDROID_HOME`/`NDK_HOME`、Rust Android targets
  (`aarch64-linux-android` 等)

---
title: social-proxy-scripts-container-app
date: 2026-06-25
tags: project, tauri, rust, android, instagram, snapchat, automation
---

# social-proxy-scripts-container-app

基于 Tauri 2 的脚本容器应用(桌面 + Android)。在应用内加载社交平台网页(Instagram / Snapchat),并把 `scripts/` 目录下的 JS 脚本注入页面执行。

## 基本信息

- 仓库:`git@github-colna:presence-io/social-proxy-scripts-container-app.git`
- 路径:`/Users/user/Dev2/zhangzheng/social-proxy-scripts-container-app`
- 当前分支:`main`,版本 `0.1.0`

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

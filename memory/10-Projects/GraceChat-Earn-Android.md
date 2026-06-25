---
title: GraceChat-Earn-Android
date: 2026-06-25
tags: project, android, kotlin, gradle, im, haven
---

# GraceChat-Earn-Android

GraceChat Earn(代号 Haven),Harbor 团队的重点 Android 项目,集成即时通讯、Ins 运营辅助、语音/视频通话、浮窗等业务形态。

## 基本信息

- 仓库:`git@github-colna:presence-io/GraceChat-Earn-Android.git`
- 路径:`/Users/user/Dev2/zhangzheng/GraceChat-Earn-Android`
- 当前分支:`haven/release`
- Gradle 根工程名:`GraceChat_Earn_Android`,主构建入口 `haven/build.gradle`

## 架构(多模块 Gradle)

| 模块 | 说明 |
|------|------|
| `haven` | 主 App:UI、业务逻辑、ABTest、第三方 SDK |
| `common` | 业务通用组件:MVVM、ListLiveData、事件总线 |
| `network` | 网络与 API 封装(retrofit/okhttp) |
| `utility` | 底层工具方法(设备信息、扩展函数) |
| `tuicallkit-kt` | 腾讯通话组件 Kotlin 包装 |
| `jsbridge` | WebView JS Bridge(H5↔原生) |
| `faceverify` | 人脸认证与相册 SDK |
| `base_theme` | 公共主题资源 |

## 环境

- Android Studio Koala+(至少 Bumblebee)
- JDK 17
- NDK 默认只打包 `armeabi-v7a`、`arm64-v8a`
- 需自备 `local.properties`(SDK 路径、密钥)
- 签名文件 `scripts/sign/*.jks` 随项目存放,**勿外传**

---
title: aichat-v2 + Hindsight 本地搭建踩坑
date: 2026-11-26
tags: [troubleshooting, aichat-v2, hindsight, deepseek, gradle, rosetta, maven]
---

# aichat-v2 + Hindsight 本地搭建踩坑

本地项目：`/Users/colna/WORK/aichat-v2-hindsight`（clone 自私有仓 `presence-io/aichat-v2`，未提交/未推送）。

> 2026-11-26 16:13 更新：该目录已被整理/合并为 `/Users/colna/WORK/aichat-hindsight`（更完整实现，见 `docs/local-chat.md`），原路径已不存在；下文命令中的路径请替换为 `aichat-hindsight`。

## 1. Maven Central 经本机代理 TLS handshake 断开

- **现象**：Gradle `Could not download ... > Remote host terminated the handshake`，构建卡在 `Calculating task graph` 十几分钟。
- **根因**：本机代理（127.0.0.1:7897）对 `repo.maven.apache.org` 的 Java TLS 握手不稳定；curl 正常，Gradle 的 Java HttpClient 失败。
- **修法**：项目 `.gradle/aliyun-mirror.init.gradle.kts`（gitignored）在 `allprojects.repositories` 里前置阿里云镜像，构建时传：
  ```bash
  ./gradlew ... --init-script .gradle/aliyun-mirror.init.gradle.kts
  ```
  镜像：`https://maven.aliyun.com/repository/public`、`.../google`、`.../gradle-plugin`。

## 2. gRPC 1.62.2 的 `osx-aarch_64` 产物实际是 x86_64

- **现象**：`generateProto` 报 `protoc-gen-grpc-java-1.62.2-osx-aarch_64.exe: program not found or is not executable`。
- **根因**：Maven 上该 classifier 的文件 `file` 显示 `Mach-O 64-bit executable x86_64`，Apple Silicon 需要 Rosetta。
- **修法**：
  ```bash
  softwareupdate --install-rosetta --agree-to-license
  ```
  之后 `generateProto` 正常。

## 3. Hindsight/uvicorn 不支持 h2c upgrade

- **现象**：Java HttpClient 调 Hindsight 返回 400 `Invalid HTTP request received` / 422 `Field required`；Hindsight 容器日志 `WARNING: Unsupported upgrade request.`。
- **根因**：JDK HttpClient 默认尝试 HTTP/2 cleartext（h2c）upgrade，uvicorn 不支持。
- **修法**：`HttpClient.newBuilder().version(HttpClient.Version.HTTP_1_1)`。

## 4. Hindsight 行为备注

- bank 不存在时 `POST /memories/recall` 返回 404；`POST /memories` retain 会自动建 bank。
- bank id 按 user 隔离：`aichat-v2-user-<userId>`；同一 user 跨 conversation 共享。
- retain 用 `async=true` + `operation_id` 幂等；召回在 PromptRouter 之后追加到 systemPrompt。

## 5. 验证命令

```bash
cd /Users/colna/WORK/aichat-v2-hindsight
set -a; source .env; set +a
./gradlew compileKotlin --no-configuration-cache \
  --init-script .gradle/aliyun-mirror.init.gradle.kts
./gradlew :test --tests "io.presence.aichat.HindsightConfigTest" \
  --no-configuration-cache --init-script .gradle/aliyun-mirror.init.gradle.kts
scripts/run-local.sh
```

实测：DeepSeek `deepseek-flash` 返回 COMMITTED；两轮后 Hindsight bank 出现 Alex/Yosemite/flat white；新 conversationId 仍 recall 5 条并答出 flat white。

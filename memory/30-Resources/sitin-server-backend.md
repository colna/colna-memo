---
title: sitin-server 后端项目全貌（Java 多模块）
date: 2026-07-09
tags: [sitin-server, 后端, java, spring-boot, grpc, kafka, 架构]
---

# sitin-server：后端 monorepo

**仓库**：`git@github.com:presence-io/sitin-server.git`
**本地**：`/Users/max/Dev2/zhangzheng/sitin-server`（工作区内，`includeIf` 生效，提交身份 colna）
**活跃分支**：`release/test`（不是 `main`；`main` 落后）

## 技术栈（已核实，非 agent 转述）

- **Java 21**（每个模块 `JavaLanguageVersion.of(21)`），无 Kotlin 源码，Gradle 脚本才是 `.kts`
- **Spring Boot 4.0.0** / Spring Cloud 5.0.0（`gradle/libs.versions.toml`）
- **gRPC 1.71.0**，用 `spring-grpc-spring-boot-starter`
- Kafka：Confluent 8.1.1 + GCP Managed Kafka（SASL_SSL / OAUTHBEARER）
- 镜像：**JIB**（`llm-schedule-admin` 例外，走 Docker）
- **无根 `build.gradle.kts`**，每个模块自包含；模块清单在 `settings.gradle.kts`

## 模块地图

**命名规律**：`*-api` = gRPC 读/写服务，`*-infra` = 纯库（无 main），`*-worker` = Kafka 消费者 / 定时任务。

| 模块 | 类型 | 职责 | 端口 |
|---|---|---|---|
| `chat-service-api` | gRPC | 从 PG 读会话/任务状态，供 gateway + aichat | gRPC 9090（默认） |
| `chat-service-worker` | Kafka | 处理 TIM 回调、任务管理、推 IM | mgmt 8089 |
| `chat-service-infra` | 库 | JPA + Redis + 事件 schema | — |
| `user-service-api` | gRPC | 用户资料 / 头像上传 GCS / 封禁 / 登录 | gRPC 10086, HTTP 8080 |
| `user-service-worker` | Kafka | 头像 AI 流水线（Gemini 经 LLM schedule） | mgmt 8090 |
| `payment-service-api` | gRPC | 提现（分档表、日限额、PayPal） | gRPC 10086, HTTP 8095 |
| `payment-service-worker` | Kafka | **收益计算 / 扣款** | mgmt 8096 |
| `task-service-worker` | XXL-Job | 真人接管的定时/异步任务 | 随机 |
| `llm-schedule-api` | **WebFlux + R2DBC** | LLM 网关：key 管理、按权重路由、`generateContent` 代理 | HTTP 10010, gRPC 10086 |
| `llm-schedule-worker` | Kafka | 落调用记录、自动调权重、自动封禁坏 key | 无 web |
| `llm-schedule-admin` | **Next.js 16** | LLM 管理后台（非 Gradle 模块，Docker 构建） | 3000 |
| `dora-queue-coordinator` | K8s Job 编排 | 分片生成每日 Show Queue，拉起 worker Job | — |
| `dora-queue-worker` | Kafka | 每次消费 1 条，跑 Gemini 推荐流水线 → Redis | — |
| `pwa-profile-worker` | Kafka + gRPC | 爬 IG 资料（Apify/RapidAPI）+ Gemini 打标 | gRPC 10086, HTTP 8095 |
| `sitin-middleware:*` | 库 | `llm-schedule-client` / `tim-message` / `datatester-client` | — |

**目录但非 Gradle 模块**：`llm-schedule-admin`、`user`（只放 `user/api/user_api.proto`）、`deploy`（运维）。
**被注释掉**：`sitin-middleware:byteplus-datatester`。

## PWA 请求怎么进来

**仓库里没有 Java gateway 模块。** 前端不直连这些服务：

```
PWA ──HTTP+Protobuf──> presence-server-gateway（外部仓库）──gRPC──> chat-service-api
                                                                   user-service-api
                                                                   payment-service-api
```

- gateway 职责：**从 session 注入 userId**（前端不传）、gRPC Status → ChatServiceCode 异常转换、枚举映射
- `deploy/{prod,test}/envoy/envoy.yaml` 里的 **Envoy 只挡在 `llm-schedule-api` 前面**（10010/10000 → 两个副本），不是 PWA 的入口
- 服务间：gRPC 按 DNS 名（`llm-schedule-api:10086`、`user-service-api:10086`、`datatester-v2:50051`）
- 异步：Kafka topics `im.message.push` / `im.message.send` / `chat.state.event` / `ins.content.crawled` / `user.avatar.update`
- 配置：新 worker（chat/payment/dora/pwa）走 **Nacos**（阿里 MSE），其余读 profile YAML

> **proto 不在这个仓库**，在独立仓库 `sitin-server-proto`，发成 Maven artifact。
> 仓库里唯一的 `.proto` 是 `user/api/user_api.proto`。

## sitin4.0 任务中心：四服务

`docs/sitin4/README.md`：

```
Msg Gateway ──TIM 回调──┬──> checker（风险打分，独立服务，不在本仓库）
                        └──> chat-service-worker（事件处理）
checker ──状态变化/探针──> chat-service-worker
chat-service-worker ──Kafka──> payment-service-worker（收益计算）
chat-service-worker ──Kafka──> aichat v2（托管状态）
chat-service-worker ──TIM 推送──> PWA
PWA ──gRPC──> chat-service-api（查询/上报）
```

**设计原则**（`docs/sitin4/payment-service-worker.md`）：
> 任务中心**不算收益**：`rewardCents` 仅用于前端展示，实际由 payment 独立计算。

## 部署

- `Jenkinsfile.k8s.dev` / `.prod`，共享 `@Library('presence-pipeline')`，`buildType: 'jib'`
- **实际部署的只有 10 个**：`chat-service-worker`、`llm-schedule-{api,worker,admin}`、`user-service-{api,worker}`、`dora-queue-{coordinator,worker}`、`task-service-worker`、`pwa-profile-worker`
  → `*-infra` 是库；**`chat-service-api` / `payment-service-api` / `task-service-infra` 不独立部署**
- `dora-queue-worker` 特殊：镜像注入 coordinator 的 env，不是常驻 Deployment
- registry `us-east1-docker.pkg.dev/heyhru-server/dora-service`，cluster `dora-cluster`（us-east1-b）
- 命名空间 **`dora-dev-k8s`** / **`dora-prod-k8s`**（与 Loki 查询用的 label 一致，见全局 CLAUDE.md）
- prod 触发 ArgoCD sync；ArgoCD 看外部仓库 `presence-io/dora-k8s-config`

## docs/ 值得读的

| 文件 | 内容 |
|---|---|
| `ARCHITECTURE.md` | 系统总览图 + 模块依赖 + 关键数据流 |
| `docs/sitin4/*` | **4.0 任务中心真源**（proto / database / data-structures / 各服务职责） |
| `docs/deployment.md` | Jenkins 流程、GKE 集群/命名空间、端口、环境变量 |
| `docs/database-schema.md` | PG 16 表结构（LLM schedule 的 keys/models/call-records 按日分区，保留 7 天） |
| `docs/dora-queue/*` | Show Queue 架构与设计 |
| `docs/phone-verification-anti-abuse.md` | 手机号验证反刷（Prelude） |

## ⚠️ 安全问题（已发现，未处理）

1. **`gradle.properties` 里提交了 GCP 服务账号私钥**（`artifactRegistryMavenSecret`，base64 的 service-account JSON，含完整 `private_key`）。
   - `project_id: heyhru-server`，`client_email: java-gradle-package@heyhru-server.iam.gserviceaccount.com`
   - 自 `6b2a8373 feat(llmss): api & worker` 起一直被 git 跟踪
   - 讽刺的是同目录的 `local.properties`（Nexus 凭据）**被 `.gitignore` 忽略了** —— 有人知道该忽略，这把 key 漏了
   - **需吊销 + 轮换 + 改环境变量注入**
2. 多个 `application-dev.yaml` 里有**明文 PG 密码 / TIM key / admin 密码**
   （`user-service-api`、`llm-schedule-api`、`task-service-worker` 等）

## 相关

- 前端：`sitin-next/packages/app-pwa`
- 拉黑链路的后端缺口见 [[troubleshooting/sitin4-endchat-backend-gap]]
- Loki 线上日志查询见全局 `~/.claude/CLAUDE.md`

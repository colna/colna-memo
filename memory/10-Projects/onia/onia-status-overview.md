---
title: Onia 功能与技术现状梳理
date: 2026-11-19
tags: project, onia, status, overview, social-proxy, architecture
---

# Onia 功能与技术现状梳理

更新于 2026-11-19（亚洲/上海）。本文是给商业助手 / 协作方阅读的**独立成稿**：覆盖产品能做什么、各端怎么协作、关键技术约束与现阶段边界，并按**已实现 / 进行中 / 下一阶段**分层。不编造 QPS、延迟、容量或用量数字；不做竞品或商业模式分析。

**资料真源**：本机 colna-memo `memory/10-Projects/onia/`。现码只读核对：`/Users/colna/WORK/onia`（私有仓库 [colna/onia](https://github.com/colna/onia)）。详细方案仍以同目录专题文档为准；本文件是现状快照，不替代架构与阶段方案正文。

**相关文档**：[overview.md](overview.md) · [architecture.md](architecture.md) · [status.md](status.md) · [instagram-dm-sync.md](instagram-dm-sync.md) · [instagram-auto-reply.md](instagram-auto-reply.md) · [instagram-actions-pipeline.md](instagram-actions-pipeline.md) · [instagram-chat-agent-kotlin.md](instagram-chat-agent-kotlin.md) · [script-ota.md](script-ota.md) · [google-sign-in.md](google-sign-in.md) · [signal-multi-action.md](signal-multi-action.md) / [signal-multi-action-impl.md](signal-multi-action-impl.md) · [admin-data-observability.md](admin-data-observability.md)

---

## 1. 产品是什么

Onia 是个人项目的 **Social Proxy（SP）**：用户在本机登录社媒账号，服务端做事实收录、策略决策与调度，原设备 WebView 注入脚本执行动作，结果回传服务端。管线语义为：

```
Signal（事实）→ Policy（决策）→ Todo（待办）→ Scheduler → Behavior（设备执行）→ 结果回写
```

首期平台是 **Instagram**；协议按多社媒预留（`type + platform`），不把平台名写进类型名。目标是「容器 + 管线 + 自动回复」，不是另做一套创作者工作台。

---

## 2. 功能点（产品能做什么）

### 2.1 账号、设备与登录

- 用户注册 / 登录；**Google 登录**已接入（服务端校验 ID token；App 走 Desktop OAuth client + 自定义 scheme + PKCE，secret 不上端，由服务端 `/v1/auth/google/exchange` 换码）。
- 设备绑定：App 作为设备上线，经 WebSocket `device.hello` / 心跳上报能力与运行时；服务端维护在线态与能力画像。
- Instagram 账号绑定：在设备 WebView 登录后探测并绑定 `account_id` / handle；工作台可见绑定与在线状态。
- 控制台（`admin.onia.asia`）按角色分流：普通 `user` 看自己的数据；`super_admin` 看全量并管用户/角色（决策已定，实现程度见 §5）。

### 2.2 运营动作（设备侧脚本执行）

经统一入口 `SocialProxy.execute(action, params)`，常见动作包括（不完全列表，以脚本与协议登记为准）：

| 能力域 | 示例动作 | 说明 |
|--------|----------|------|
| 资料 / 同步 | `sync_profile`、`get_profile` 等 | 拉资料；profile media 能力已落地并生产部署 |
| 收件箱 | `pull_inbox`、`pull_thread` | 列表未读、会话全量/增量；游标落 PostgreSQL |
| 监听 | `start_dm_listener` / `stop_dm_listener` / 状态 | 新消息事件上报；现行客户端在 WebView 登录确认后**自动拉起监听**（相对早期「必须由服务端下发才听」的方案有演进） |
| 发送 | `send_message`、`send_greet`、`send_reply` | 三个动作分开；发文本主路径优先 GraphQL `IGDirectTextSendMutation`，失败再 DOM 回退 |
| 关系 | `follow_user` 等 | 获客漏斗中的回关等 |
| 发布 | `create_post` / `create_story` | 后台有发布页与定时计划；**live 上传仍未在脚本打通**（页面有「上传未打通」警告） |

Signal 支持**多动作串行**（一条定义展开为有序 Todo）与**定时计划**（一次性 / 周期，`source=cron`）。

### 2.3 私信同步与自动回复

- **阶段 A/B**：升级 `pull_thread`（GraphQL 全量/增量 + DOM 回退）、监听器、`signal.report`（`DM_RECEIVED` / `INBOX_ACTIVITY`）。弱路径：`INBOX_ACTIVITY` → `pull_inbox` → 规范 `thread_fbid` → `THREAD_PULL_NEEDED` → `pull_thread`，不编造 `message_id`。
- **阶段 C（自动回复）**：会话视图更新后 ingest 系统 Signal → Agent 写 `reply_drafts` → Policy 按模式产不产生 `send_reply`。
  - 模式：`review`（默认先审后发）/ `auto` / `suggest`；挂在账号级设置；会话可人工接管停发。
  - 仅一对一：无法判定或群聊不调用 Agent。
  - **Mock Agent** 已真机跑通出站（2026-11-13）；**Kotlin 聊天 Agent**（DeepSeek、AIPolice、RAG、跨会话重复缓存）已接入管线，真机用真实模型的完整验证在今日待办中仍列为待确认项。

### 2.4 客户端 App（Flutter）

- 三 Tab：工作台 / 数据 / 我的。
- 工作台：WebView、绑定、任务可见状态、刷新等。
- 数据页：today/7d/30d、账号筛选、KPI、动作分布、柱图、最近执行、漏斗、周报推送、CSV 导出（剪贴板）。
- 设置：语言（中/英 + 跟随系统）等。
- 约定：Android + iOS **双端同步改**；平台差异单独记。

### 2.5 官网与控制台

- **官网** `apps/web`：介绍与下载；已有隐私政策 / 服务条款等 legal 页；Google 注册相关已接。
- **控制台** `apps/admin`：设备与能力、Signal/Todo/Behavior 追踪、脚本发布/回滚、内容发布与定时、数据可观测、CSV 等；品牌与 onboarding 已重建过一版。

### 2.6 脚本 OTA

- 服务端存 `script_releases`（版本、SHA-256、内容、激活态）；设备 hello / 激活时推 `script.update`；客户端下载校验后热刷新（有 Behavior 在跑时先缓存）。
- 生产端到端热更新已验证（2026-11-17）：激活即推送，CDP 可读到注入源码 marker 切换。内置 assets 仍作首次/离线回退。尚未做 Ed25519 签名与灰度字段落地。

---

## 3. 各端如何协作

```
┌──────────────┐     HTTPS / WS      ┌─────────────────────────────┐
│ Flutter App  │◄──────────────────►│ onia-server（Rust，单进程）   │
│ + WebView    │  hello/心跳/下发     │ Signal·Policy·Todo·Scheduler │
│ + 脚本注入   │  signal/behavior     │ Behavior·Identity·草稿表     │
└──────┬───────┘                      └───────────┬─────────────────┘
       │ 仅设备本机打 IG / CDN                    │ HTTP（密钥头）
       ▼                                          ▼
 Instagram 域                          apps/chat-agent（Kotlin/Ktor）
                                       只生成草稿，不投递、不打 IG
       ▲
       │ 同源页面登录态
 packages/scripts（TS → social_proxy.js）
```

| 端 | 职责 | 明确不做 |
|----|------|----------|
| **设备客户端** | 登录态、脚本执行、监听、拉会话、发送、发送前锚点检查、媒体本机拉取后上传、能力上报 | 不内置 LLM；不把「拉完再回复」写死在脚本里 |
| **脚本** | 动作真源；`SocialProxy.execute` | 不调模型；不直写业务库 |
| **onia-server** | 鉴权、设备会话、事实幂等、会话视图、Policy、调度、草稿落库、控制台 API、脚本版本、统计 | 不持 IG cookie；不代发/反代 IG；不自己拉 IG 历史给 Agent 以外的「平台请求」 |
| **onia-agent crate** | `ChatAgent` 端口；`HttpChatAgent` 调 Kotlin；URL 空则进程内 Mock | 不实现完整提示词与模型路由 |
| **chat-agent（Kotlin）** | AgentLoop + Hooks + `commit_draft` + 安全 + 按账号+会话串行；Mock 或 DeepSeek | 不发 Instagram；不持会话材料 |
| **Policy** | 纯函数：事实 + 视图 → Todo 计划（获客 / 收件箱 / 回复包） | 无 I/O；不调模型 |
| **控制台 / 官网** | 直连服务端同一信封；官网偏静态 | 无 BFF；官网不发任务 |

追踪贯通：`signal_id → todo_id → behavior_id`。统一信封字段（`v` / `kind` / `id` / `trace_id` / `idempotency_key` / `ts` / `payload`），设备、后台、桥共用。

**自动回复主路径（简化）**：

1. 设备上报 `DM_RECEIVED`（或弱路径经 `pull_inbox` 产出 `THREAD_PULL_NEEDED`）
2. Policy → `pull_thread` → 设备拉回消息 → 服务端写会话视图
3. 编排过滤后调 Agent → 写 `reply_drafts` → ingest `REPLY_DRAFT_READY`
4. Policy 按 `review`/`auto`/`suggest` 决定是否 `SEND_REPLY`
5. Scheduler 下发 `send_reply` Behavior → 设备发送 → 结果回写；发送前用锚点 mid，对方有新消息则弃旧草稿

---

## 4. 关键技术约束与现阶段边界

### 4.1 硬约束（2026-11-19 细化）

1. **凡打向 Instagram 私有域 / CDN / GraphQL / Direct 私有接口的请求，只能由设备端发出**（WebView 脚本或宿主本机）。服务端只调度、存游标/事实/会话视图、校验并收 `behavior.result` / `signal.report`。**例外**：用户显式创建的 Official Channel 可调用 Meta **官方 Graph/Messenger 端点**；这不等于私有接口，也不允许用该 token 打私有接口。
2. 服务端**不得**持 WebView cookie / `sessionid` 等会话材料，不得反向代理 IG，不得用服务端出口去拉 IG 私有媒体 URL。Official Channel 只持**加密的官方 Graph access token**（AES-256-GCM），token 不回显、不进日志。
3. **Agent 只写草稿，不发消息、不持社媒会话、不操作 WebView、不直接创建发送 Todo**；是否发送由 Policy 决定。Official Channel 的入站事实可触发 Agent，但发送仍由 Policy 的官方 Todo 决定。
4. **禁止在 Behavior 里写死「拉完再回复」**；必须事实/视图更新后再进 Policy。
5. **默认不代发**：普通 DM 默认 `device`（Mode A）；只有通道显式设为 `official`（Mode B）且用户确认后，服务端才经官方 Graph 代发。官方发送失败不静默回落私有接口。
6. 不手造 `message_id` 或入站事实；sessionid 等敏感值不进日志、测试、memo、回复。
7. 不迁入公司私有仓源码、内部接口、TIM / Nacos / 公司身份字段等。

### 4.2 技术栈与仓库边界（现码核对）

Monorepo `/Users/colna/WORK/onia` 主要结构：

- `crates/`：`onia-protocol`、`onia-signal`、`onia-policy`、`onia-todo`、`onia-behavior`、`onia-agent`、`onia-identity`、`onia-scheduler`、`onia-device`、`onia-server`、`onia-client-core`
- `apps/`：`server`、`client`（Flutter）、`admin`、`web`、`chat-agent`（Kotlin）、`dev-device`
- `packages/scripts`：TypeScript 脚本真源 → 产物 `social_proxy.js`

业务主库 **PostgreSQL**（`sqlx` 迁移）；首期服务端单进程组装。生产域名侧可见 `api.onia.asia`、`admin.onia.asia` 等（以部署记录为准）。

### 4.3 现阶段产品边界

- 平台：Instagram 私信自动回复为主；评论转私信已通过 Official Channel 接入（关键词 + media 范围 + 首条 Private Reply），公开评论回复可选。
- 默认回复模式仍是 **`review`**；测试账号可切 `auto`。
- 一对一以外不自动回复。
- 内容发布：计划与下发框架在，**帖子/快拍上传链路未打通**。
- 脚本 OTA：生产热更新已通；**无签名、无灰度**。
- iOS：双端同步约定已立；Google 登录在 iOS 上需换 `ASWebAuthenticationSession`（Custom Tab / inAppBrowserView 不支持 custom scheme）——属已知遗留。
- 客户端 Code Review（2026-11-19）仍标出若干工程债（如 `DeviceRuntime.stop` dispose 后 notify、token 明文 SharedPreferences、默认 API base 曾为 http 等）；不在此展开为性能数字。

---

## 5. 已实现 / 进行中 / 下一阶段

状态判断以 memo（含 `status.md` 至 2026-11-17 条目与 Daily 至 2026-11-19）+ 现码目录为准。相对 2026-11-13 早期记忆有多处更新，见文末「与 11-13 记忆的差异」。

### 5.1 已实现（可视为落地，含生产或真机证据）

| 领域 | 要点 | 证据来源（摘要） |
|------|------|------------------|
| P0 管线 | 协议、PostgreSQL、设备握手、Signal→Todo→Behavior→脚本→回写、控制台追踪 | 初始提交与后续 main；架构/status |
| 设备绑定 / 发信 / 打招呼 | 绑定账号；`send_message` / `send_greet`；API 优先发送 | status、actions pipeline |
| 私信同步 A/B | `pull_thread` 升级、监听、`signal.report`、未读判断补丁 | status 11-13；dm-sync |
| 自动回复阶段 C（Mock） | 草稿→Policy→`send_reply`；2026-11-13 真机出站成功（含 `thread_kind_unknown` 修复、WebView 即听） | status 16:52 / 17:03 |
| Kotlin chat-agent | `apps/chat-agent`；HTTP 契约；Mock + DeepSeek；AIPolice；关键词 RAG；跨会话重复缓存；Rust `HttpChatAgent` | status 19:20–20:14；方案 md |
| 鉴权加固 | JWT secret 强制、bootstrap admin 开关、CORS 白名单配置、Agent 密钥头 | status 11-17 |
| 脚本 OTA | 上传/激活/下载/推送；生产 0.1.x 热更新 E2E | script-ota.md；提交 `2a8148c` 等 |
| 定时 Signal + 发布页 | 周期/一次性 cron；`/publish`；生产迁移与冒烟 | status 11-17 |
| 数据 Tab / 统计 API | 客户端数据页；`/v1/me/stats/*`；周报推送；CSV | status 21:30；`ae87193` |
| 官网 + legal | web 构建部署；隐私/条款 | google-sign-in / Daily 11-18 |
| Google 登录 | 控制台 OAuth、服务端、App scheme 回调；OPPO 真机进入工作台 | Daily 11-19 14:16；`4299c12` |
| Profile media | 服务端/脚本能力落地并随 OTA 相关提交上线 | status；profile-media 研究文档 |

### 5.2 进行中（已有代码或半闭环，待收尾 / 真机确认 / 工程债）

- **真实模型自动回复真机闭环**：DeepSeek + AIPolice + RAG + 跨会话缓存已在服务链路验证过能力；Daily 11-19 仍把「真机验证自动回复（真实模型）」列为待办——不宜写成「生产用户侧已稳定依赖真实 Agent」。
- **生产 CORS**：配置项已有；日志仍提示生产未设 `ONIA_CORS_ALLOWED_ORIGINS`（待补）。
- **Google 登录 iOS 路径**：Android 已通；iOS 需 `ASWebAuthenticationSession`。
- **控制台角色分流与邮箱验证**：11-17 决策已写清（`user` / `super_admin`、注册仅 user、邮箱验证后可用）；实现完整度需按后台代码继续收敛，本文不把它写成 100% 完成。
- **客户端工程债修复**：Code Review 指出的 dispose/重连/存储等（Daily 11-19）。
- **工作树未提交改动**：现码 `git status` 仍有脚本/客户端等本地修改（只读观察，未操作仓库）；与「已推送 main」并存。
- **发帖/快拍上传打通**：发布编排在，上传未通。
- **周报推送时机**：仅 hello/heartbeat 投递，无独立定时任务。

### 5.3 下一阶段（明确未做或仅方案/部分能力）

- Agent 增强：向量 RAG、Redis 级跨用户重复缓存、全量 Prompt 模块、专用小分类模型等（status 已列「仍未做」）。
- 脚本 OTA：**Ed25519 签名**、按设备/用户**灰度**（`gray` 字段未用）。
- 第二社媒平台（只加策略与脚本、不改信封）——架构 P2，未开工为产品闭环。
- iOS 能力降级与后台策略的完整产品化；推送叫醒在断线场景的系统级验证深度。
- 评论等非 DM 频道的自动回复。
- 独立创作者工作台（内容库/日历/分析）——架构「待确认」，默认不做。
- 受控工具调用（Agent 侧）——架构 P3 可选。

---

## 6. 假设（单独标注）

下列为推进中的工作假设或产品默认；若产品定义不同应改决策，而不是 silently 改实现口径。

1. Onia 定位是个人可运维的 Social Proxy：**本机登录、服务端决策、原设备执行**；首期单进程部署，不承诺多活或容量数字。
2. 首期平台 **Instagram**，自动回复首期以**私信一对一**为主。
3. 账号默认回复模式是 **`review`**；`auto` 用于测试账号跑通发送，不是把全体默认改成自动发送。
4. Agent **只生成、不投递**；真实模型只换适配器 / LLM 端口，不改 Signal / Policy / `send_reply` 契约。
5. 客户端以 **Flutter +（规划中的）Rust 核心** 为主路径；当前以 Flutter 客户端 + TS 脚本为主力交付面（Rust `onia-client-core` 在 monorepo 中，与 UI 的桥接完成度以现码为准，本文不夸大）。
6. 业务主库仅为 **PostgreSQL**；媒体走对象存储（若启用），不进库。
7. 文档真源在 **colna-memo**，不使用飞书；代码在私有 `github.com/colna/onia`。
8. 不把公司 sitin-next / aichat-v2 源码或内部接口迁入；只借鉴可独立成立的架构原则。
9. 双端约定：App 改动默认 **Android + iOS 一起改、一起验证**。
10. 超级管理员在 App 内**不做跨用户代操作**；额外权限只在控制台。

---

## 7. 与 2026-11-13 参考记忆的差异（核对结论）

Colna Bot 提供的 11-13 记忆在当时正确，但之后 memo/仓库已前进。本文核对到的主要不一致如下：

| 11-13 记忆 | 核对后现状（至 2026-11-19） |
|------------|------------------------------|
| 自动回复因 `thread_kind_unknown` 跳过草稿，尚未闭环 | **已修复并真机闭环**（11-13 17:03 Mock 出站成功） |
| 真 Agent 属下一阶段；本版仅 Mock | **Kotlin Agent 已接入**；DeepSeek / AIPolice / RAG / 跨会话重复缓存已落地；真机真实模型验证仍可能待确认 |
| 官网、脚本 OTA 属下一阶段 | **官网已上线（含 legal）**；**脚本 OTA 已实现且生产热更新 E2E 通过** |
| Kotlin 方案仅设计文档 | 方案文档仍在；**`apps/chat-agent` 已在 monorepo 实现并被 server 调用** |
| P0 各端「已在做」 | P0 管线及大量 P1 能力已合入 main 并多次生产部署；另有数据 Tab、定时 Signal、Google 登录等 |

---

## 8. 给下游读者的读法

- 要改架构原则 → [architecture.md](architecture.md)
- 要改自动回复契约 → [instagram-auto-reply.md](instagram-auto-reply.md) + [instagram-chat-agent-kotlin.md](instagram-chat-agent-kotlin.md)
- 要改拉消息/监听 → [instagram-dm-sync.md](instagram-dm-sync.md)
- 要追「哪天合了什么」→ [status.md](status.md) + `memory/50-Daily/`
- 要做竞品 / 付费模式 → **不要用本文凑数**；交给商业助手另开，且不得把未证实的用量数字写进商业材料

---

## 9. 官方通道增量（2026-11-19）

- 新增 **Official Channel**：Meta/Instagram Webhook 入站 + 评论转私信。实现为独立模块（`crates/onia-server/src/official/`）与独立迁移 `0016_official_channel.sql`，不复用 DeviceSession，不持有 WebView cookie/sessionid。
- 入站映射：`messages` → `DM_RECEIVED`（幂等键 `instagram:{mid}`，与设备监听一致）；`comments` / `live_comments` → 评论 Signal；`messaging_postbacks` / `message_reactions` / `messaging_seen`；story reply / story mention。**Mode A**：`pull_thread` 由设备补全会话，`send_reply` 仍由设备发送。**Mode B**：Webhook 直接把入站消息写入 `igsid:<peer>` 的一对一会话视图，服务端生成草稿后按 Policy 经官方 Graph 发送；无需设备在线，但仍要求通道绑定过设备账号。
- 官方出站：评论 `SEND_PRIVATE_REPLY`（Graph Private Replies，7 天窗口、一评论一次由唯一索引保证）、`REPLY_COMMENT`（公开回复）、Mode B `SEND_OFFICIAL_REPLY`（24 小时窗口校验）。官方发送失败**不回落**私有接口，也不自动重试；DM 可人工重试，Private Reply 保持一次。
- 控制台 `/official`：通道 CRUD、token 加密存储/轮换（不回显）、Mode A/B 切换、入站/Private Reply 开关、`INBOX_ACTIVITY` 兜底开关、实验性官方会话历史读取、关键词 + media_id 规则、最近 Webhook 与官方发送审计。新建通道默认 `inbox_activity_fallback_enabled=false`，官方 Webhook 为权威来源。
- 环境：`ONIA_META_APP_SECRET`、`ONIA_META_WEBHOOK_VERIFY_TOKEN`、`ONIA_OFFICIAL_TOKEN_KEY`、`ONIA_GRAPH_BASE_URL`。Webhook 路径 `/v1/official/instagram/webhook`。
- 仍未做：Flow 可视化、Handover Protocol、Story/mention 完整产品化、转化看板。Mode B 的 peer IGSID 依赖 Webhook 或最近一次设备/官方会话消息；对不上时只记录事实，不静默降级到设备。官方会话历史读取为实验开关（`conversation_read_enabled`），失败时只用 Webhook 单条消息。

---

## 10. 商业化增量（2026-11-19）

- **套餐与计费域**：协议 `crates/onia-protocol/src/billing.rs`（`plan.get` / `plan.purchase`、PlanLimits/PlanUsage/PlanSubscription/PlanCatalogItem）；服务端 `billing.rs` + 迁移 `0015_plans_and_usage.sql`（`user_subscriptions`、`plan_orders`、`usage_counters`）。
- **档位**（代码常量，改价改代码）：Free $0（1 设备/1 账号、50 草稿/月、仅 review/suggest）；Solo $15（$150/年，2/2、500、数据导出）；Pro $35（$350/年，5/5、2,000、auto、官方触发开关）；Studio $79（$790/年，10/10、5,000、优先支持）。年付按 10 个月。
- **限额执行**：`device.bind` 设备上限、`account.bind` 账号上限、`account_reply_settings.set` 的 `auto` 需 Pro（降级后旧 auto 在草稿生成时自动按 review 处理）、真实 ChatAgent 草稿/招呼按次扣月度额度（Mock 不扣、失败退还）、`stats.export` 需 Solo+；`super_admin` 内部账号豁免。
- **购买**：当前为**模拟购买即时生效**（写 `plan_orders` 状态 `simulated_paid`，不扣款）；接入 PSP 时替换 checkout 适配层，订阅/权益模型不变。
- **官网**：新增 `/pricing` 独立页与首页 `#pricing` 区块（`apps/web/src/Pricing.tsx`），nav/footer 入口，月/年切换与四档能力对比；Pro 的官方触发标注「Meta App Review 后开放」。
- **控制台**：新增 `/billing`（当前套餐、AI 草稿/设备/账号用量、四档对比、模拟购买确认）；概览套餐卡；设备额度用尽、自动化 auto 锁定、草稿额度用尽处均有升级入口。
- **App**：设置页「套餐与用量」卡（当前档与用量、额度用尽提示、跳转控制台）；设备/账号/导出限额错误映射为中文升级提示。
- **部署状态（2026-11-19）**：商业化 + 官方通道首次发布提交 `9670fc9` 已部署生产（迁移 0015–0017 success，admin/web dist 已同步）。后续 `93ae291`（模式 B conversation read + outbound 审计，迁移 `0018_official_conversation_read.sql`）已 push，**尚未部署**。
- **待办**：生产 `.env` 尚未配置 `ONIA_META_APP_SECRET` / `ONIA_META_WEBHOOK_VERIFY_TOKEN` / `ONIA_OFFICIAL_TOKEN_KEY`，官方通道在配置前不可用；Meta App / Business Verification / App Review 仍待真实主体申请；App 新 APK 未分发。

---
title: Onia 当前进展
date: 2026-11-13
tags: project, onia, status, testing
---

# Onia 当前进展

## 2026-11-17 鉴权配置加固

- Rust server：`ONIA_JWT_SECRET` 必填且至少 32 位，拒绝旧开发值；`ONIA_BOOTSTRAP_ADMIN_ENABLED` 显式开启才创建管理员；新增 `ONIA_CORS_ALLOWED_ORIGINS` 白名单。
- Kotlin chat-agent：启动必须提供 `ONIA_CHAT_AGENT_SECRET`，`/v1/replies`、`/v1/greetings`、`/v1/traces` 均校验 `X-Onia-Agent-Secret`。
- `.env.example` 改为安全默认；本机 `.env` 保留两个 dev 开关，本地开发不受影响。
- 验证：server `cargo check` + 6 个 config 单测通过；chat-agent `gradlew test` 通过，无密钥接口实测 401。
- migration 7 checksum 冲突已处理：核对结构一致后校准 `_sqlx_migrations`，本地启动与 `/health`、401 验证通过。
- 推送前全量验证通过（Rust workspace、scripts 132、admin/web build、chat-agent、Flutter 89、dev-device typecheck）；admin 的 React 19 类型污染已用 tsconfig paths 修复。已提交并推送 `d9fc43b` 到 `origin/main`。

更新于 2026-11-13 16:14（亚洲/上海）。文档只写在 colna-memo，不使用飞书。相关代码在本机 `/Users/colna/WORK/onia`，GitHub 私有仓库 [colna/onia](https://github.com/colna/onia)。方案真源见同目录 [architecture.md](architecture.md)、[instagram-dm-sync.md](instagram-dm-sync.md)、[instagram-auto-reply.md](instagram-auto-reply.md)（自动回复方案文件级提交 `ed8fc486`）。本条只同步实现与复测进展，不改方案正文。

## 本机环境

- 服务端默认监听约 `:8080`，PostgreSQL 在 Docker 容器 `onia-postgres`，主机端口 `5433`。后台 Umi 开发服务约 `:5173`。
- Flutter 在 `/Users/colna/development/flutter/bin`。真机联调时不要启动 `apps/dev-device`，不要把 `local-sim` 或旧设备 `0bce1f33-…` 当成真实流程的通过依据。
- 硬约束不变：凡打向 Instagram 域或其 CDN / GraphQL 的请求，只能由设备 WebView 脚本或宿主在本机发出；服务端不持 cookie、不代发、不反向代理。不得手造 `message_id` 或入站事实。不要把 sessionid 值写入日志、上报、测试、memo 或回复。

## 真机与账号

- 当前测试机是 OPPO Reno7 5G，ADB serial `f7031061`。在线设备 `4f311d52-6f63-4a5d-b26e-cc84f497f104`（属 `111@163.com`），已绑定 Instagram `@hwangdohyeon165` / `account_id` `24264435413`。
- 对测会话是一对一 `colna.null`，规范键 `thread_fbid` `2411750266243016`。
- 测试自动回复发送闭环时，该测试账号需切到 `auto`；架构默认模式仍是 `review`。

## 已落地、准备推送的功能

以下均在 `/Users/colna/WORK/onia` 未提交工作树（推送前 HEAD 仍是 `7573f87`），用户已要求把可用功能 commit / push。本次没有把尚未修好的草稿跳过问题一并改进去。

### 阶段 A / B（聊天记录与监听）

- 已按 `instagram-dm-sync.md` 实现阶段 A（升级 `pull_thread`）和阶段 B（监听器与 `signal.report`）。
- 过程中已修：空参 `pull_thread` 的可见会话与规范 `thread_id` 解析、失败/成功状态转移与幂等回写、监听进会话后停死、`INBOX_ACTIVITY` 缺幂等键、`DM_LISTENER_STATUS` 十分钟分桶互吞等。

### 阶段 C（自动回复基础闭环）

- 已按 `instagram-auto-reply.md` 实现基础闭环：事实/视图更新后由 Policy 再产 Todo；Mock Agent 从 `['hi','how are u','u?','im good']` 随机取一条；`send_reply` 只由 Policy 产出，不与 `send_message` / `send_greet` 合并。
- 已修：`send_reply` 发送前把历史对方消息误判成新消息，导致草稿被丢弃后永久漏回。评审返工后结论曾为可合。
- 弱路径仍是 `INBOX_ACTIVITY` → `pull_inbox` → 归一规范 `thread_fbid` → `THREAD_PULL_NEEDED` → `pull_thread`，不编造 `message_id`。

### 未读判断（`pull_inbox`）

- 已读 GraphQL 的 `slide_messages`、水位回执和 mailbox id（EIMU）；空回执或身份对不上时保持不可判定，不把 watermark 折成 0，也不把没有证据当成已读。
- 导航栏收件箱链接角标（与监听同源的 `navBarBadgeCount`）作为页面级未读主信号，`document.title` 的 `(N)` 只作次要；只提升 `unknown` 会话，不压过已观测为已读的会话。
- DOM 行未检出未读标记时标 `unknown`，不再写 `observed_read`。`observed_read` 只留给 Direct v2 `read_state=1`、水位已覆盖 `last_activity` 这类正向已读证据。
- 2026-11-13 16:04 复审结论：上述未读补丁可合。两条建议改也已做：资料页「发消息」等待降至 20 秒；输入框 placeholder 改为精确匹配，并排除 `Message requests` 一类入口。
- 同日约 16:02 的真机复测主路径上，上一轮「写成 `unread=0` / 空预览」未再出现：`pull_inbox` 认出 `colna.null` / `2411750266243016`，`unread=1`，预览为「你为什么不回我」，并进入 `THREAD_PULL_NEEDED`。

### 发送

- `send_message` / `send_reply` / `send_greet` 仍是三个动作，发文本主路径改为设备侧 API 优先：走 `IGDirectTextSendMutation`（doc_id `28737274835943013`）静默发送；解析不到会话或 GraphQL 不可用时回退已有 DOM 输入框路径（含占位条 / iframe 识别）。一旦 mutation 可能已经发出（例如网络中断），不再回退 DOM，避免重复发送。
- `send_greet` 若还没有会话，不发明建会话接口，回退现有「进主页点发消息」DOM 路径。
- 本机 `@onia/scripts` 单测最近一次为 125 通过。设备侧 API 发送和最新输入框回退尚未用包含本轮脚本的新包在真机上验证。

### 脚本产物

- 当前未压缩 IIFE：`apps/client/assets/scripts/social_proxy.js`，227,760 字节，SHA-256 `847900c92708b31e1ef3d434167403c15d48cf4fb2b12e5b08d2a6c52d9a646c`，末行仍是 `//# sourceURL=social_proxy.js`。
- 产物同时包含未读判断、输入框回退和 API 优先发送。真机要看到这些改动，需要再打一版同签名包覆盖安装；在此之前不要用旧包验证这些项。

## 尚未修好（不要当成已完成）

用户于 16:13（亚洲/上海）要求先停止继续修改，只同步进展。以下问题尚未改代码。

- 主路径自动回复仍未出站。约 16:02:15 新入站正文「你为什么不回我」后，`pull_thread` 已拉到新 mid `mid.$cAH8_LSLLovaoBTMEhWh0-27S7ays`，但 16:02:18 出现 `REPLY_DRAFT_SKIPPED`，原因 `thread_kind_unknown`：游标 `is_group` 为空，该次增量结果也没有 `is_group` / 参与者人数，因此没有针对这条入站的草稿，也没有主路径 `send_reply`。这是已知一对一会话，规范 id 和别名都在，不应把字段缺失当成未知群聊而丢掉回复。
- 附带、尚未查完：约 16:05 二次 `resume` 的 `pull_thread` 疑似把 11-12 月旧 mid `mid.$cAH8_LSLLovaoANrD0Ghz5V65XmX-`（正文 `what are u doing now`）写成最新入站并生成草稿；随后 `send_reply` 失败，`error_class=script`，文案是「已确认 Instagram 已登录，但还没读到用户名，暂不能绑定。」真机无出站。

## 下一步

- 修一对一会话因 `is_group` 为空而跳过草稿的问题；并查清旧消息被当成最新入站、以及 `send_reply` 误走绑定/用户名探测错误的原因。
- 再打同签名包覆盖安装，复测自动回复出站和设备侧 API 发送。
- 仓库 commit / push 与 MR 仍走 Git 助手。

## 2026-11-13 16:52 本版本改动（WebView 即听 + 任意入站自动回复修复）

用户新要求：任意账号给 App 内账号发消息后自动回复；listener 在 WebView 启动时注入并开始监听。本条记录本机 `/Users/colna/WORK/onia` 的未提交改动，方案正文不变。

### 已改动

- **WebView 即听**：bundle 在页面加载注入并确认 Instagram 登录后，`InstagramSession` 自动调用页面内 `SocialProxy.startListener`。不再依赖服务端先下发 `start_dm_listener` 才启动；SPA 路由由脚本内 URL watcher 续挂。显式执行 `stop_dm_listener` 成功时，本 App 会话不再自动拉起（重启 App 后恢复默认自动监听）。
- **`pull_thread` 线程类型**：全量、增量、续拉结果统一补 `is_group` / `participant_count`；线程元数据缺失时按消息 sender 推断。修复增量路径因 `is_group` 为空触发 `REPLY_DRAFT_SKIPPED` / `thread_kind_unknown`。
- **`send_reply` 身份校验**：允许「已登录但暂时读不到用户名」的会话继续发送（只校验 account_id）；发送前 Direct v2 锚点检查继续优先用绑定 account_id，缺失时回退 viewerEIMU。
- **旧消息误判**：`DM_RECEIVED` 产出的 `pull_thread` 参数增加 `inbound_message_id`；服务端只在「该会话最新一条消息是对方入站」时生成草稿，`THREAD_SYNC_CONTINUE` 只补历史、不重新生成回复，避免旧 mid 被当成最新入站。
- **监听兜底**：实时事件拿不到 viewerEIMU 时改报 `INBOX_ACTIVITY`，不伪造 `DM_RECEIVED`。

### 验证

- `pnpm --filter @onia/scripts typecheck` 通过；单测 127 通过（新增 2 条 `pull_thread` kind 回归）。
- `cargo test --workspace` 通过（onia-server 22、onia-policy 27 等）。
- `flutter analyze` 无问题。
- 脚本产物 `apps/client/assets/scripts/social_proxy.js` 更新为 229,228 字节，SHA-256 `b4ca5ac26fe80d5d647bb898d67fdc4df451a106257e272c31e2ef7d8cdf80db`。

### 仍需真机验证

- 用包含本轮脚本的新签名包覆盖安装，确认 WebView 一启动 `DM_LISTENER_STATUS` 即为 running。
- 用 `colna.null` 或任意新账号发新私信，确认 `pull_thread` 带回 `is_group=false`，草稿生成后按账号模式（测试账号 `auto`）出站。
- 其它账号默认仍是 `review`；若要让所有绑定账号默认自动发送，需要另改产品默认或逐账号配置。

### 2026-11-13 17:03 真机验证结果（通过）

- 覆盖安装本轮 release APK（同签名，保留数据与 IG 登录）后，WebView 启动即听：`DM_LISTENER_STATUS running=true, mode=navbar`。
- 任意账号（测试用 `colna.null`）新私信「你在干什么」触发完整链路：`INBOX_ACTIVITY`（navbar badge 1）→ `pull_inbox`（unread=1）→ `THREAD_PULL_NEEDED` → `pull_thread`（incremental，`is_group=false`）→ `THREAD_CONTEXT_READY` → `REPLY_DRAFT_READY`（`how are u`，auto/approved）→ `send_reply` 成功。
- 发送走 GraphQL `IGDirectTextSendMutation`，`transport=api`，返回平台 `message_id` `mid.$cAH8_LSLLovaoBWmv1hjnj0fZlAxZ`；出站消息已写入 `thread_messages`，草稿状态 `sent`。用户确认对端收到。
- 备注：旧入站的一条 08:05 approved 草稿仍保留（未发送），不阻塞或重复回复新入站。

## 2026-11-13 19:20 Kotlin 聊天 Agent（C1）已接入

- 新服务：`apps/chat-agent/`，Kotlin 2.1 + Ktor 3（本机 JDK 17，JVM target 17；文档建议 21 受环境限制）。
- 契约：`POST /v1/replies/generate`（同步，只生成草稿）、`GET /health`、`GET /v1/traces/{traceId}`；共享密钥头 `X-Onia-Agent-Secret`。
- 运行时：AgentLoop + Pre hooks（一对一、入站锚点、历史过滤、Persona YAML）+ `commit_draft` + 输出安全 + 按 account+thread 的 Lane（作废旧轮次）+ 内存 trace。
- Mock 文案仍用当前更长的 4 条中文（优先用户最新要求，覆盖文档里的 4 条短字符串）；真实模型只替换 `LlmPort`。
- Rust：`onia-agent` 新增 `HttpChatAgent`，`onia-server` 通过 `ONIA_CHAT_AGENT_URL/SECRET/TIMEOUT_MS` 选择远程服务；URL 为空时保留进程内 Mock。`classify_draft` 已区分 `failed_safety` / `failed_generate`。
- 本地启动：`just chat-agent`（会加载仓库 `.env`）；Kotlin 单测 `just chat-agent-test`。当前 Kotlin 服务在 `127.0.0.1:8090` 运行，Rust server 已配置并重启。
- 验证：Kotlin 5 测试通过；`cargo test --workspace` 通过；Kotlin `/health` 与 `generate` 直调返回 `mock_v1` 草稿。真机端到端待下一条消息确认。

### 2026-11-13 19:42 Agent 能力补齐（aichat-v2 方向）

- Kotlin Agent 已有 AIPolice：硬正则 + DeepSeek 分类 + 最多 2 次自然语言重写 + 重复检测；违规仍存在则失败不发送。
- Prompt 边界已覆盖：AI 身份/技术痕迹、联系方式、线下见面、平台付费、编造经历/地点、语言/长度、重复开头。
- RAG：Rust server 从 `thread_messages` 检索早于当前窗口的相关历史（ASCII 词 + 中文 bigram，top5/回落 3 条），通过 `rag_context` 传给 Kotlin Agent，注入 system 的“相关过去对话”段。
- 真实 DeepSeek `deepseek-flash` 已验证：带 RAG 的猫咪上下文能被正确引用；trace 记录 policeVerdict/regenCount/ragItems。
- 仍未做：aichat-v2 的跨用户重复缓存（Redis）、全量 Prompt 模块、LLM 分类器专用小模型、RAG 向量检索；当前是本地关键词 RAG。

### 2026-11-13 20:14 跨会话重复缓存已接

- 两种口径一起做：`peer_outbound`（同 peer、当前窗口之外，40 条）和 `account_outbound`（同账号跨会话，80 条）。
- 规则：同 peer 任意同开头即重写；同账号同开头出现 ≥2 次才重写；trigram 阈值 0.55；纯问候豁免。
- 数据由 Rust server 从 `thread_messages` 查询后随请求传给 Kotlin Agent；未引入 Redis。

## 2026-11-17 客户端双端同步约定

- 以后所有 Onia App（`apps/client` Flutter）的修改，默认 Android + iOS 双端一起改、一起验证；不得只改一端。
- 每一步至少确认：`flutter analyze`、相关测试、Android dev/prod 构建、iOS release/simulator 构建（或明确说明无法验证的原因）。
- 平台差异（WebView、权限、签名、通知、后台）单独记录，但共享 Dart/UI/协议/i18n 代码必须同步。
- dev-only 能力（如服务器地址输入）通过编译开关控制，Android/iOS 的 dev/prod 配置保持一致。

## 2026-11-17 账号与后台权限决策（待实现）

- 统一使用 `admin.onia.asia`，不拆分用户后台域名；登录后按角色分流。
- 普通 `user` 只能看到自己的控制台：自己的设备、账号绑定、Signals/Trace、草稿、自动化设置、账号设置。
- `super_admin` 可看全量后台数据，并拥有后台用户/角色管理；现有 `admin` 角色迁移为 `super_admin`。
- 注册接口只创建 `user`；注册用户需要邮箱验证后才能访问后台/App 功能（验证方式待定）。
- 超级管理员是否需要在 App 内跨用户操作仍未确认；默认理解为正常登录 App 使用全部功能，不做跨用户静默代操作。

## 2026-11-17 超级管理员 App 权限澄清

- 超级管理员「访问 App 所有功能」= 正常登录 App，使用普通用户可用的全部功能；不做跨用户查看/代操作。
- 超级管理员的额外权限只体现在 `admin.onia.asia`：全量后台数据 + 用户/角色管理。
- 因此 App 不需要新增跨用户管理 UI；后台统一 admin.onia.asia，按 role 分流。

## 2026-11-17 定时 Signal 与内容发布

- Signal 定义新增目标设备/账号与定时计划（一次性/周期），迁移 `0010_signal_definition_schedule.sql`；服务端新增 `schedule.rs` 每 15s 扫描到期定义，用 `source=cron` 创建 MANUAL_ACTION 实例，幂等键按 `definition_id + scheduled_for`；周期任务跳过错过的时段；归档会停用定时。
- 管理后台：定义编辑器新增定时面板、定义列表新增「定时」列；新增 `/publish` 内容发布页（发帖/发快拍，立即下发或保存为一次性/周期计划），导航位于「编排与执行」。
- 权限：定时目标设备校验归属；普通用户只能绑定自己的设备，super_admin 可跨用户选择设备。
- 验证：`cargo test --workspace` 全过（server 37，含真实 DB 定时下发测试）；admin build 通过；scripts 133 tests、dev-device/web typecheck 通过；本地迁移 0010 已应用；API 冒烟创建定时定义返回 target/schedule。
- 边界：live `create_post` / `create_story` 上传仍未在脚本打通，发布页和节点保留「上传未打通」警告；定时定义要求步骤参数完整，否则保存被拒。

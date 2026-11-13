---
title: Onia 当前进展
date: 2026-11-13
tags: project, onia, status, testing
---

# Onia 当前进展

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

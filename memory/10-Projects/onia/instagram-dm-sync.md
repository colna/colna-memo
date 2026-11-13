---
title: Onia Instagram 聊天记录与新消息监听接入方案
date: 2026-11-12
tags: project, onia, social-proxy, architecture, instagram, dm, pipeline
---

# Onia Instagram 聊天记录与新消息监听接入方案

本文是把 sitin-next 本地分支 `feature/sp-ig-dm-api` 工作树里「获取聊天全量记录」和「监听新消息」的脚本能力，接入现有 Onia 调度（Signal → Policy → Todo → Scheduler → Behavior）的技术方案。对照来源是该分支未提交文件的只读副本（box `/workspace/sp-ig-dm-api/`），以及 colna-memo 既有约定和本机 `/Users/colna/WORK/onia` 的活代码（含未提交改动）。**只出方案，不改 Onia 仓库，也不改 sitin-next。**

## 结论

1. **拉会话是一次执行，监听是设备侧运行时状态。** `getChatMessages` 升级现有动作 `pull_thread`（能力 `inbox.pull_thread.v1`，Todo `PULL_THREAD`），仍走管线，一次 Behavior 只跑这一个动作。`startDMListener` / `stopDMListener` / `getDMListenerStatus` 登记为新动作 `start_dm_listener` / `stop_dm_listener` / `get_dm_listener_status`（建议能力 `inbox.dm_listener.v1`）：start 在监听挂上后立即成功终态，**不要**把监听期间做成一条一直不结束的 Behavior。
2. **游标和消息正文只走 PostgreSQL 与 `behavior.result`。** 不迁 `AndroidBridge`（`getLastMessageId` / `isFullSyncDone` / `uploadMessages`），不发明脚本直传业务库的旁路。服务端按绑定账号维护游标；截断（`truncated`）时不得把 `full_sync_done` 标真，且必须可续跑。
3. **监听到的事件必须变成 `signal.report`，再由 Policy 产出 Todo。** 有平台 `message_id` 的实时事件报 `DM_RECEIVED`（幂等键 `platform + message_id`），本阶段策略产出 `pull_thread`，**不**产出回复。只有红点或会话列表、没有可靠 `message_id` 时，报已有的 `INBOX_ACTIVITY` → `PULL_INBOX`。Behavior 里不要写死「拉完再回复」。
4. **脚本不得在注入约 20 秒后自行启动监听。** 这会绕过 Todo / Scheduler。需要监听时，由服务端经管线下达 `start_dm_listener`；脚本重注入后若监听丢失，也由管线再下发，而不是页面定时器自启动。
5. **落地先升级 `pull_thread`，再接通监听与事件上报。** 自动回复 / Agent 先审后发是阶段 C，方案见 [instagram-auto-reply.md](instagram-auto-reply.md)；本文阶段 A/B 仍不把发送做成必须项。
6. **所有 Instagram 接口调用必须由设备客户端发出**（WebView 内脚本或宿主在本机发起），覆盖全量 / 增量拉会话和监听新消息两条路径。Onia 服务端只做调度、游标、校验，以及收 `behavior.result` / `signal.report`。不得由服务端代发或反向代理 Instagram GraphQL / 私有接口 / 媒体地址，也不得持有 cookie 或 session 去打 Instagram。目的是避免这类请求的出口 IP 都落在同一台服务器上。

## 假设

1. 活代码在 `/Users/colna/WORK/onia` 的未提交工作区之上继续，不要回退已有登录地址 `?mtn`、`ONIA_WEBVIEW_INSPECT` 默认关闭、现有探测 / 发送路径，以及 `pull_inbox` / `sync_profile`。
2. 单动作契约、脚本内 pipeline、Flutter 按 `behavior_id` pending / resume，仍按 [instagram-actions-pipeline.md](instagram-actions-pipeline.md) 执行。Signal 多动作串行按已确认的 [signal-multi-action-impl.md](signal-multi-action-impl.md)：编辑页参数是可选默认值，必填在发送时再校验，不要写回「编辑时必填」。
3. sitin-next 分支 `feature/sp-ig-dm-api` 的 HEAD 与 `feature/sp` 同为 `f31b49a77`，能力在未提交工作树。本文只吸收可复用的页面内算法与 GraphQL 形状，不迁公司业务协议。
4. `packages/business-pwa-proto/proto` 子模块指针从 `43db9d6f` 变为 `7af7c6e8`（合入 `feature/list-distribution-server` 到 `release/test`，以及 SITIN 4.4 用户 / 内容等协议）。该 diff 与这三份脚本无关，**不作为本方案的一部分**，也不迁入 Onia。
5. 下文出现的 `PAGE_SIZE = 20`、`MAX_PAGES = 50`、总时长预算 `240s`、自动启动延迟 `20s`、熔断连续失败 5 次 / 冷却 `300s`、以及宿主脚本 RPC 默认等待 `45s`，都是对照来源或本机代码里的既有常量，只用于说明如何对齐超时与续跑，**不是**容量或性能承诺。实现时可沿用这些脚本内部上限，但不要把它们写成系统 SLA。
6. 本文（阶段 A/B）平台仅 Instagram。自动回复、`send_reply`、Agent 先审后发见阶段 C 方案 [instagram-auto-reply.md](instagram-auto-reply.md)；脚本 OTA 仍不在必须范围。

## 与既有文档的关系

- [architecture.md](architecture.md) 的分层继续有效：执行必须经 Todo → Scheduler → Behavior；设备事实走 `signal.report`（`device_id` 以会话为准）；「拉完再回复」要先形成新的 Signal，再交给 Policy，禁止在 Behavior 服务里写死下一步。
- [instagram-actions-pipeline.md](instagram-actions-pipeline.md) 已把 `getChatMessages` 映射为 `pull_thread`，并规定游标落 PostgreSQL、消息走 `behavior.result`。本文是该动作的 **GraphQL 全量 / 增量升级**，以及监听这条新能力的接入；不是另开一套动作名，也不推翻信封。其中「媒体分片失败后由服务端 Range 回退去拉 URL」与下文硬约束冲突，**本方案覆盖该回退**：Instagram 及 CDN 地址只允许设备本机拉取，服务端不再代取。
- [signal-multi-action-impl.md](signal-multi-action-impl.md) 继续约束多步 Signal：一条 N 步定义展开为 N 条有序 Todo；`pull_thread` 只是其中一步。截断续跑不得偷偷给同一条手动 Signal 追加隐藏步骤。

## 硬约束：Instagram 请求只出设备

拉会话（含 GraphQL 全量 / 增量、Direct v2 或 DOM 回退）和监听新消息（含探测 `getIGDNotificationEventEmitter`、收件箱列表、底栏红点，以及随后因此触发的 `pull_thread` / `pull_inbox`）都必须遵守：

- 凡是打向 Instagram 域或其 CDN 的请求，只允许从**该设备**上的 WebView 脚本或宿主发出，使用页面已有的登录态。出口 IP 是设备当前网络，不是 Onia 服务器。
- 服务端职责仅限于：调度（Signal / Todo / Behavior）、游标与别名、参数校验、收 `behavior.result` 与 `signal.report`、以及把期望监听状态再经管线下达。服务端可以把游标写回下次 `pull_thread` 的 params，但**不能**拿着这些参数自己去拉消息。
- 禁止：服务端持 cookie / `sessionid` / `fb_dtsg` 等会话材料；服务端或边缘节点反向代理 `/api/graphql`、`/api/v1/direct_v2/` 等私有接口；为了「统一出口」或省掉客户端实现而加的服务端拉取。
- 消息结果里可以带附件 URL，作为设备侧元数据。若媒体需要落 Onia 对象存储：由**客户端在本机拉取**后再上传到 Onia；服务端只收设备上传的对象，不得拿 Instagram URL 去代下。本轮若还不做对象存储，就只在 `behavior.result` 里保留 URL 与文本，仍然不要让服务端去取这些地址。

与先前方案的冲突检查：本文本身没有建议服务端代打 GraphQL。唯一需要对齐的，是 [instagram-actions-pipeline.md](instagram-actions-pipeline.md) 里媒体 `media.range_download.v1` 的「宿主本机直连失败后，服务端 Range 回退」。宿主本机直连仍然允许，也是推荐路径；**服务端再去拉 Instagram / CDN URL 不再采用**。设备侧 HTTP 桥若只在本机用 WebView Cookie 发请求（既有发帖口径），仍算客户端发出，不是服务端代发；本轮不顺手改发帖。

## 对照来源：保留什么、改掉什么

### 拉会话（`getChatMessages.js`）

源脚本已是 API-first：同源 GraphQL `IGDThreadDetailQuery` + Before / After 分页；失败再回退 DOM。策略是：无 `lastMessageId` 或尚未 `fullSyncDone` → 全量；已全量则先在最近一页里找锚点，再 After 增量；锚点丢失或增量超过一页则全量补齐。无 thread key 时会导航收件箱并点击第一条未读。空结果会 `emitScriptEvent(CHAT_HISTORY)`，非空则 `AndroidBridge.uploadMessages`。

Onia 要保留的是：**GraphQL 优先、DOM 回退、全量 / 增量 / 锚点丢失补齐、截断可恢复。** 要改掉的是：Android 本地库游标、`womanSocialId` / `manSocialId` / `creatorId` / `chatId=woman:man`、空结果触发的公司侧 `CHAT_HISTORY` / `CLOSE_CHAT` 语义、用昵称点击未读线程。空消息在 Onia 是一次成功的空结果，不额外报脚本事件。

还要注意：源脚本在 `mode` 为 `full` 或 `resume` 时，只要消息非空就会 `setFullSyncDone`，**没有**判断 `apiTruncated`。Onia 不得照搬这一点。只有本轮全量或续跑在未截断的情况下跑完，才允许把 `full_sync_done` 写为真。

### 监听（`startDMListener.js`）

源脚本是纯监听 + 上报，不跳转、不进聊天。优先页面内 `getIGDNotificationEventEmitter` 的 `newMessage`，否则收件箱 DOM 或底栏红点。经 `bridge.emitScriptEvent` 上报 `NEW_MESSAGE`。有熔断；SPA 路由变化时，非 realtime 模式会重启，realtime 不重启。脚本加载约 20 秒后自动 `SP.execute("startDMListener")`。区分自己 / 对方必须用 `viewerEIMU`，不能用 `ds_user_id`。

Onia 要保留探测顺序、自己消息过滤、页面内去重与熔断、非 realtime 随路由重启。要改掉自动启动，以及把 `NEW_MESSAGE` 直接打到公司事件总线。改为宿主转 `signal.report`。

### GraphQL 上下文（`insGraphql.js` 的 `viewerEIMU`）

Onia 已有 `packages/scripts/src/instagram/graphql.ts`，但 `PageContext` 目前只有 `viewerId`，且读取时可能把 EIMU 填进 `viewerId`。接入时必须把 **`viewerId`（登录用户，约等于 `ds_user_id`）和 `viewerEIMU`（`interop_messaging_user_fbid`）拆开**。方向判断和监听过滤只用 EIMU。不要把 sitin 源文件拷进仓库，只复用已有同源 GraphQL 辅助并补字段。

## 身份与会话键

对照坐在真会话上的验证记录：同一会话同时存在 `thread_fbid`、`thread_key`（出现在 `/direct/t/<key>/`）和更长的 `thread_id`（Direct v2 形态）。三者**不是**同一个值。当前 Onia `pull_thread` 走 `/api/v1/direct_v2/threads/{thread_id}`，游标表主键里的 `thread_id` 很可能是 Direct v2 形态；GraphQL 查询用的是 `thread_fbid`（URL 里的 `thread_key` 也可以当查询参数，返回真正的 `thread_fbid`）。

因此：

- 协议和发令表单仍用字段名 `thread_id`，含义定为 **规范键 = `thread_fbid`**。脚本若只拿到 URL key 或 Direct v2 id，应在结果里同时回传 `thread_id`（规范键）、`thread_key`、`ig_direct_thread_id`（若能拿到）。
- `thread_cursors` 继续以 `platform + account_id + thread_id` 为主键，`thread_id` 存规范键。另外增加别名查找（可用附加列或附属表），避免旧游标和新拉回的 id 对不上而误当成「无游标」，重复全量。对不上时宁可先按未全量处理，也不要写到另一行后把旧行当成权威。
- 对方身份用 `target_handle`；数字用户 id 若脚本解析到，只放结果，不当控制台必填项。
- 不迁 `womanSocialId`、`manSocialId`、`creatorId`、`pwa_user_id`。

## `pull_thread` 如何升级

### 参数与校验

表单仍是选填：`target_handle`、`thread_id`、`after_message_id`。编辑页只存默认值，发送时再校验；三者都可以空。服务端在创建 Todo 时，若调用方未显式给 `after_message_id`，则按游标补上 `after_message_id`、`full_sync_done`，以及截断续跑所需的 `resume_oldest_message_id`（若有）。这些是服务端写入的隐藏参数，不要做成发令必填，也不要出现在编辑页的必填校验里。

解析目标的优先级：

1. 已给 `thread_id`（或能从当前 URL 取出 key / fbid）→ 直接拉该会话。有 GraphQL 时不必先等聊天 DOM。
2. 只给 `target_handle` → 用收件箱列表（优先已有或同源会话列表，失败再走现有 Direct v2 inbox）匹配对方用户名，再进入该会话。
3. 都空：若当前已在某个聊天页，拉当前会话；否则进入收件箱，选一条未读。没有未读则本动作成功返回空消息（不要做成必须失败），由运营决定是否换目标再发。不要用「用昵称点击第一条未读」的公司路径。

### 拉取策略（与源脚本对齐，游标改由服务端给）

脚本入参同时看 `after_message_id` 与 `full_sync_done`（以及可选的 `resume_oldest_message_id`）：

- 无锚点或 `full_sync_done = false`：Thread Detail + Before 翻旧。若带了 `resume_oldest_message_id`，从该锚点继续向旧消息翻，而不是每次都从最新一页重头拉。
- 已全量：先在最近一页里找锚点，只取更新的消息；找不到再 After；After 还有下一页，或锚点彻底丢失，则全量补齐（补齐过程仍可能截断，规则同上）。

API 失败才回退 DOM。回退时同样遵守服务端游标，不得再读 Android 本地库。演示模式（`host.demo = true`）保持现有演示结果，不得把 live 截断或空会话写成演示成功。

### 截断、宿主超时与续跑

源脚本内部有分页上限和 240s 总预算；Onia 宿主对一次脚本 RPC 的默认等待是 **45 秒**（`InstagramSession._callJs`）。一次全量若在同一次 RPC 里翻很多页，会先撞上宿主超时，被标成 `network` 失败，游标也可能没写回。

因此分两层，不要只靠把 45 秒改成 240 秒：

1. **同一条 Behavior 内 pending / resume（优先）。** 在接近宿主等待上限之前，脚本主动返回 `pending`，checkpoint 带上已拉取消息、规范 `thread_id`、下一页 `resume_oldest_message_id`、已用页数和启动时间。Flutter 已有按 `behavior_id` 的恢复，续跑仍是这一次执行，Todo 不提前成功。这与跳转页面的 pending 是同一套机制，不是新开直派接口。
2. **跨 Behavior 的截断成功。** 达到脚本内部页数或总预算上限时，本动作 **成功** 返回 `truncated: true`、`full_sync_done: false`、已有消息和续跑锚点。服务端更新游标。这条 Todo 成功终态。若它属于多步 Signal，后续步骤按已确认的串行规则继续，**不要**给同一条手动 Signal 暗插一步。需要自动补齐时，由服务端再写入一条系统事实（建议类型 `THREAD_SYNC_CONTINUE`，不可由后台任意创建），Policy 产出一条新的 `PULL_THREAD`，带上同一 `thread_id` 和续跑锚点。同类未完成 Todo 仍按等待 / 去重处理，避免堆积。

源脚本「truncated 要可恢复」在 Onia 里靠第 2 层保证；第 1 层是为了不跟宿主 RPC 等待打架。

### 结果与游标回写

`behavior.result` 至少包含：规范 `thread_id`、`thread_key`、`ig_direct_thread_id`（能拿到的）、`target_handle`、`messages[]`（`message_id`、`direction`：`in` / `out` / `unknown`，方向用 `viewerEIMU` 比 `sender_fbid`、正文、时间、可选附件 URL）、`last_message_id`（本批最新）、`oldest_message_id`、`full_sync_done`、`truncated`、`mode`（`full` / `incremental` / `resume`）、`transport`（`api` / `dom`）。不要在结果里塞媒体二进制。附件若要进对象存储，由设备本机拉取后再上传，服务端不得根据这些 URL 去 Instagram 代下。

服务端只在 `ok = true` 时 upsert 游标：

- `last_message_id` 取本批最新（不要用最老锚点覆盖最新游标）。
- `full_sync_done` 仅当结果明确为真且 `truncated` 不为真。
- 保存 `resume_oldest_message_id` 以便续跑。
- 失败（含宿主超时且未走 pending）不更新游标。

现有「表单已填 `after_message_id` 则不覆盖」的规则保留。

## 长驻监听如何接入调度

监听不是「做完就结束」的工作，不能把它建模成一条 running 到停止才结束的 Behavior。推荐把「是否应该在听」做成账号 / 设备上的期望状态，把 start / stop / status 做成普通短动作。

| 动作 | 能力 | Todo | 语义 |
|------|------|------|------|
| `start_dm_listener` | `inbox.dm_listener.v1` | `START_DM_LISTENER` | 在页面内挂上监听后立即成功，结果带 `running`、`mode`（`realtime` / `inbox` / `navbar`）。已在运行则视为成功，不报冲突失败。 |
| `stop_dm_listener` | 同上 | `STOP_DM_LISTENER` | 解开监听后立即成功。 |
| `get_dm_listener_status` | 同上 | `GET_DM_LISTENER_STATUS` | 只读：是否在跑、模式、`viewer_eimu`、熔断是否激活、最近上报时间。 |

能力在 `device.hello` 里声明。设备离线不是失败；缺能力按现有规则取消或等待。start 成功后，服务端记下「期望监听为开」；stop 成功后记为关。心跳或 status 可用于对账，但对账发现「期望开、实际关」时，仍要再走管线下发 `start_dm_listener`，不要由 Flutter 或脚本擅自启动。

**明确建议：禁止源脚本那种「加载约 20 秒后自动启动」。** 理由：Onia 会在注入、跳转恢复、重新注入时反复加载脚本，自动启动等于绕过 Scheduler，也无法表达运营「这台设备现在不要听」。若产品以后希望「设备上线就听」，正确做法是：hello 之后由服务端按期望状态经管线下发 start，而不是把定时器写回脚本。本文按「禁止自动启动」锁定；若评审要改，需要明示。

Behavior 超时只约束 start / stop / status 这一次短调用，不约束之后的监听寿命。监听过程中的熔断、去重、模式切换留在页面内；熔断不是把已经成功的 start Todo 改判失败。SPA 路由变化时，非 realtime 由脚本在**已经处于运行中的监听**内部重启；realtime 不重启。脚本被卸载或整页重载导致监听丢失，属于运行时状态消失，由对账 + 管线再 start 恢复。

`start_dm_listener` 不要当成多步 Signal 里「一直占着设备」的步骤。它可以出现在发令页；一般不必编进要串行等待的长链。它与 `pull_thread` 不互斥：拉会话可能打断 inbox / navbar 的 DOM 观察，但 realtime 监听按源脚本可以继续。不要用 `already_open` 把后续手动 `pull_thread` 吞掉。

## 事件如何变成 Signal，Policy 如何再拉会话

宿主把脚本上报译成 `signal.report`，`device_id` 只信任会话，忽略载荷里任何自报设备或 `creatorId`。

| 页面事件 | 条件 | Signal | 幂等 | 本阶段 Policy |
|----------|------|--------|------|----------------|
| realtime `newMessage` | 有 `message_id`，且 `sender_fbid` 不是当前 `viewerEIMU` | `DM_RECEIVED` | `platform + message_id` | 产出一条 `PULL_THREAD`，带上 `thread_id`（以及能解析到的 `target_handle`）。已有同类未完成 Todo 时按现有等待 / 去重，不合并会话。 |
| inbox DOM 未读变化 | 能给出 `thread_id` 或 `thread_fbid`，但没有可靠 `message_id` | 仍报 `INBOX_ACTIVITY`，payload 可附会话键 | 沿用现有 inbox 幂等，不要伪造 `message_id` | 继续只产出 `PULL_INBOX`。本阶段不自动对每个未读再拆 `pull_thread`。 |
| 底栏红点 | 通常没有会话键和 `message_id` | `INBOX_ACTIVITY` | 同上 | 同上。 |

不要为了「快一点」让监听脚本自己调用 `getChatMessages` 或 `SocialProxy.execute("pull_thread")`。拉会话必须再走一遍管线。自己发出的消息在脚本侧过滤后不上报；若漏报，Policy / 后续回复包仍应按 `viewerEIMU` 再过滤一次。

本阶段 **不** 启用架构里的回复包（`DM_RECEIVED` → Agent / `SEND_REPLY`）。现有代码里 `DM_RECEIVED` 只落事实、不产 Todo，需要改成「先拉会话」。回复仍是后一阶段：那时也是新的 Signal 或会话视图更新后再由回复包产出 Todo，而不是在 `pull_thread` 成功回调里写死发送。

建议后台不要把 `DM_RECEIVED` 做成可随意手造的管理员类型（现有 `is_admin_creatable` 已排除它）；`INBOX_ACTIVITY` 仍可在「发送 Signal」的来源事件分段里提交，语义不变。

## 宿主要补的桥，以及不迁什么

现有 `OniaBridge.postMessage` 已承担脚本 RPC 回包、`pending`、媒体分片和 SPA 导航。监听事件是**无请求号的上送**，需要新增一种宿主消息（建议 `type: "signal_report"`），由 Flutter 转成 WebSocket `signal.report`。不要在脚本里复刻名为 `emitScriptEvent` 的公司接口，也不要让脚本自己打 Onia HTTP。媒体分片只允许宿主在本机直连下载；不要再走服务端 Range 去取 Instagram 地址。

`HostContext` 可以提供 `reportSignal(type, payload)` 供脚本调用，内部仍走上述桥。`device_id`、身份字段由宿主 / 会话补齐。日志继续走现有宿主日志，不必迁公司 `eventTrack`。

**不迁入 Onia：**

- `AndroidBridge` 及本地消息库（`getLastMessageId` / `isFullSyncDone` / `setFullSyncDone` / `uploadMessages`）
- `womanSocialId`、`manSocialId`、`creatorId`、`pwa_user_id`、`chatId=woman:man`
- 公司 `emitScriptEvent`、`CHAT_HISTORY`、`CLOSE_CHAT`、以及任何 sitin JWT / URL / 私有上传接口
- `packages/business-pwa-proto` 及本次子模块指针变更
- 脚本加载后 20 秒自动 `startDMListener`
- 把 `ds_user_id` 当作 DM 发送者身份
- 服务端持会话或反向代理去调用 Instagram GraphQL / 私有接口 / 媒体 CDN

## 与 `pull_inbox`、多步 Signal、发令页的关系

- `pull_inbox` 保持现有动作和结果形状，本轮不要改坏。监听里的 inbox / navbar 事件先触发它；会话正文仍由之后的 `pull_thread` 负责。`PolarisDirectInboxQuery` 不能替代 `IGDThreadDetailQuery` 去拿完整媒体 URL，这一点源验证已经说明。
- 发令页继续是单动作入口，可以发升级后的 `pull_thread`，也可以发 start / stop / status。不要把发令改成流程图。
- 多步 Signal 可以把 `pull_thread` 编进链里。该步的成功以这一次 Behavior 的 `ok` 为准（含截断成功）。自动续跑走独立系统事实，不打乱已展开的步骤序号。
- `create_post` / `create_story` 仍按既有口径，本轮不顺手改它们。

## 建议落地阶段

**阶段 A（先做，可单独验收）：** 升级 `pull_thread` 为 GraphQL 全量 / 增量 + DOM 回退；补 `viewerEIMU`；扩展游标与别名；截断不得标全量完成；同一 Behavior 内用 pending 避开 45 秒 RPC 等待；消息只经 `behavior.result` 回写。发令和多步 Signal 都能发这个动作。可用现有真机路径验收，不依赖监听。

**阶段 B：** 宿主 `signal_report` 通道；start / stop / status 动作与期望状态；禁止自动启动；`DM_RECEIVED` → `pull_thread`，弱事件走 `INBOX_ACTIVITY` → `PULL_INBOX`。在真机 WebView 上验证 realtime 是否能取到页面模块；取不到则 inbox / navbar 回退，不得假装已经在听 realtime。

**阶段 C（已单独成稿，见 [instagram-auto-reply.md](instagram-auto-reply.md)）：** 回复包 / Agent；本版本用 mock 跑通发送闭环，并保留先审后发。按未读列表拆 `pull_thread` 作为弱事件衔接写入该方案。「设备上线即听」若产品要做，也只通过 hello 后的管线下达。

## 实现时建议改动的位置

实现等本方案确认后再开始。不要先改信封字段名，不要新增绕过 Todo 的直派接口，也不要新增服务端代打 Instagram 的接口。

1. `crates/onia-protocol`：`pull_thread` 结果扩展；游标续跑字段；新动作 / 能力 / Todo 类型；`THREAD_SYNC_CONTINUE`（若采用）。`just gen`。
2. 迁移：`thread_cursors` 增加续跑锚点与别名，只向前兼容。旧行不要擅自当规范键覆盖掉。
3. `onia-policy` + `onia-server`：`DM_RECEIVED` 产出 `pull_thread`；截断后续跑的系统事实；游标补参与 upsert 规则。
4. `packages/scripts`：在现有 `instagram/graphql.ts` 上拆分 `viewerEIMU`；重写 `pullThread` 的传输层；新增监听三个动作，去掉自动启动。Flutter 增加 `signal_report` 通道，以及脚本重注入后的监听对账（只上报状态或触发管线，不本地自启动）。
5. `apps/admin`：发令与编辑页挂上新动作；`pull_thread` 表单仍全部选填；追踪能看到 `truncated`、`transport`、监听模式。不要把未打通的回复画成已能自动发送。

实现完成后把进度和如何复现直接告诉用户。不要 commit / push Onia 或 sitin-next，仓库与 MR 交给 Git 助手。代码评审交给 Code Reviewer，本方案不代替评审。

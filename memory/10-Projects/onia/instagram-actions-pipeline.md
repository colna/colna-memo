---
title: Onia Instagram 动作与 Pipeline 方案
date: 2026-11-12
tags: project, onia, social-proxy, architecture, instagram, scripts, admin, pipeline
---

# Onia Instagram 动作与 Pipeline 方案

本文是给 Coder 落地用的实现方案。对照 colna-memo 中的 [architecture.md](architecture.md)、[real-pipeline-app.md](real-pipeline-app.md)，以及本机仓库 `/Users/colna/WORK/onia`（含未提交改动）写成。行为参考本机 `sitin-next` 在提交 `f31b49a77`（`feature/sp`）上的 Instagram 方法与 `executeAction` / `insGraphql` / `mediaChunk`，**只作对照，不迁入源码，不切 `sitin-next` 分支，不改 Onia 仓库，不提交、不合入**。

## 结论

本轮在已打通的「真管线 + 真实 Instagram 登录 + `host.demo = false` 实打」之上，把 SP 中用户点名的 Instagram 能力迁到 Onia，并用后台表单发令：

1. 脚本真源仍是 TypeScript（`packages/scripts`），用现有 `pnpm --filter @onia/scripts bundle`（或 `just bundle-scripts`）打成**单一** `apps/client/assets/scripts/social_proxy.js`。客户端继续 `rootBundle` 注入这一份产物。不要改成 SP 那种按组下发/注入多个脚本，本轮也不做脚本 OTA。
2. 执行仍走 Signal → Todo → Scheduler → Behavior。`behavior.execute` 信封保持 `behavior_id`、`todo_id`、`action`、`params`。后台「选动作 + 填表单」创建已登记 Signal，由策略产出 Todo，**不得**新增绕过管线直派 Behavior 的接口。
3. 动作名在协议、后台表单、脚本三侧统一为 **snake_case**，与现有 `pull_inbox` / `sync_profile` 一致，Behavior 与脚本动作 1:1，不手写映射表。
4. 现有 live 路径必须保留：官方 WebView、会话探测、登录地址 `https://www.instagram.com/accounts/login/?mtn`（路径判断只看 path）、WebView 远程调试仅测试包且 `ONIA_WEBVIEW_INSPECT` 默认 false。不得改坏 `pull_inbox` / `sync_profile`。
5. 只做 Instagram，不迁 Snapchat。不把 SP 的 AndroidBridge、JWT 共享密钥、`womanSocialId` / `manSocialId` 字段或 `social-proxy-api-dev.sitin.ai` 写进 Onia。

## 假设

1. 个人项目 Onia，不复用、不迁入公司私有代码或内部接口。实现应对准已登录 `instagram.com` 页的同源会话，以及 `f31b49a77` 的**行为**（入参含义、是否需跳转、失败怎么分类），而不是拷贝文件。
2. 活代码在 `/Users/colna/WORK/onia` 的未提交工作区之上继续，不要回退已有登录/注入/实打改动，不要用 box 上的旧脚手架。
3. 角色形象与 loading Lottie 不在本轮。本轮不做脚本 OTA、Agent、`send_reply`、FCM 叫醒、官网、FFI。
4. 媒体文件仍不进 PostgreSQL；Range 回退去 Onia 服务端，URL 必须由宿主注入的 `api_base` 与设备 token 访问，构建物禁止写死环境地址。
5. Instagram 会话、Cookie、`sessionid` 不出原设备，不出现在后台表单，不上传到 Onia 服务端。
6. 不编造 QPS、延迟、容量或可用性数字。文中若提到现有客户端超时，只描述现状需要怎么改，不当成性能承诺。

## 命名拍板（三侧同一套）

协议（`crates/onia-protocol`）、策略、后台表单、`packages/scripts` 的 `execute` 分支全部使用下表 **Onia 动作名**。SP 的 camelCase 只作为来源对照，不进信封，也不在脚本入口再设一层别名转发（`getProfileBioLink` 并入 `get_profile`，不再单独登记）。

| SP 方法 | Onia 动作 | 能力 cap | Todo 类型 | 说明 |
|---------|-----------|----------|-----------|------|
| （已有） | `pull_inbox` | `inbox.pull.v1` | `PULL_INBOX` | 保持不变 |
| （已有） | `sync_profile` | `profile.sync.v1` | `SYNC_PROFILE` | 保持不变，同步**已绑定账号自己的**资料 |
| `getProfileInfo`（别名 `getProfileBioLink`） | `get_profile` | `profile.lookup.v1` | `GET_PROFILE` | 按用户名查询**任意**公开资料 |
| `followUserById` | `follow_user` | `social.follow.v1` | `FOLLOW_USER` | 目标是用户名，不是内部数字 ID |
| `getChatMessages` | `pull_thread` | `inbox.pull_thread.v1` | `PULL_THREAD` | 增量同步聊天记录 |
| `sendMessage` | `send_message` | `inbox.send_message.v1` | `SEND_MESSAGE` | 控制台主动发私信 |
| `sendGreet` | `send_greet` | `inbox.send_greet.v1` | `SEND_GREET` | 从对方主页打招呼 |
| `likePost` | `like_post` | `media.like.v1` | `LIKE_POST` | |
| `commentPost` | `comment_post` | `media.comment.v1` | `COMMENT_POST` | |
| `getMediaInsights` | `get_media_insights` | `media.insights.v1` | `GET_MEDIA_INSIGHTS` | |
| `createPost` | `create_post` | `media.create_post.v1` + `media.range_download.v1` | `CREATE_POST` | 发帖依赖分片下载能力 |
| `createStory` | `create_story` | `media.create_story.v1` + `media.range_download.v1` | `CREATE_STORY` | 发快拍同上 |

架构里已预留的 `inbox.send_reply.v1` **本轮不实现、不在控制台出现**，留给后续 Agent 先审后发。不要和 `send_message` 混成一个动作，以免回复草稿和运营发令抢同一套参数。

身份字段不用 SP 的 `manSocialId` / `womanSocialId` / `creatorId`：

- 已登录、被下发的那一方，用设备当前绑定的 `platform` + `account_id`（及展示用 `handle`），由服务端在创建 Signal / 填 params 时写入，表单不填、脚本不信客户端自报的「自己」。
- 对方用 `target_handle`（Instagram 用户名）。数字用户 ID 若脚本内部解析得到，可出现在结果里，不作为控制台必填项。
- 聊天用 `thread_id`。消息列表用 `texts`，不用 `messageTexts`。

## 和现有 Signal 类型的关系

**不改信封，不另开暗门。** 控制台发令新增一种已登记事实类型：

- Signal 类型：`MANUAL_ACTION`
- `source`：`admin`
- payload：`{ "action": "<上表动作名>", "params": { ... } }`
- 必须像现在一样先选已绑定设备及其账号；`device_id` 创建后不可改。

新增策略包「手动执行包」（playbook：`manual`）：

1. 只处理 `MANUAL_ACTION`。
2. `action` 必须落在上表白名单（含 `pull_inbox` / `sync_profile`，方便在发令页一并操作）。
3. 用协议里的 params 契约校验 `params`，失败则拒绝创建 Signal，不落一条无 Todo 的空事实充数。
4. 产出一条 Todo：类型与动作按上表 1:1，`params` 为校验后的对象，并补上 `platform`、`account_id`；`required_capability` 为对应 cap（发帖/快拍要同时具备媒体分片 cap，策略里把主 cap 写在 `required_capability`，服务端/调度在晋升时额外检查 `media.range_download.v1`，或把该 cap 列为该 Todo 的必备集合——选一种写进协议，不要只在客户端检查）。
5. 去重按 **Todo 类型**（即按动作），与现有 `already_open` 一致：同一设备上已有未完成的同类 Todo 时，不再新建一条。不同动作可以并存。
6. Scheduler、`behavior.execute` 形状不变。

现有 `INBOX_ACTIVITY` → `pull_inbox`、`PROFILE_SYNC_NEEDED` → `sync_profile` **保留**，仍表示「收件箱有活动 / 需要同步资料」这类事实，可供日后 cron 或运营使用。它们和 `MANUAL_ACTION` 可以产出相同动作，但语义不同，不要删掉旧类型，也不要把发令页改回只能选这两种类型。

后台因此有两类入口：

- 现有「创建 Signal」页：继续只创建 `INBOX_ACTIVITY` / `PROFILE_SYNC_NEEDED`，payload 可以仍是简单来源标记。
- 新建「发令」页：选动作 + 动态表单，只创建 `MANUAL_ACTION`。

`is_admin_creatable` 与 `adminCreatableTypes` 要加上 `MANUAL_ACTION`。创建接口必须在服务端校验动作和 params，不能只靠前端表单。

## 对照现状：已有什么，还缺什么

已有、必须复用：

- 统一信封；`behavior.execute` 仅四字段；错误分类 `session` / `platform_risk` / `network` / `script` / `host`。
- 单一 esbuild 产物与 Flutter `rootBundle` 注入；每次 `onPageFinished` 且处于 `instagram.com` 时会重新注入。
- live：`probeSession`、`pull_inbox`、`sync_profile`；`HostContext` 已有 `api_base`、`version`、`capabilities`、`trace_id`、`demo`、`platform`、`account_id`。
- 手机登录、握手、WebSocket、按 `behavior_id` 幂等回写。
- cap 常量里已有 `media.range_download.v1`，但还没有对应服务端接口和宿主桥。

还缺：

- 上表新动作的协议类型、策略、脚本 live 路径与 demo 桩。
- 脚本内 pipeline：当前 `execute.ts` 只是 `switch`，页面一跳 JS 上下文销毁，Flutter 侧现有执行等待会把整段当成一次 RPC（现状还有执行超时）。`send_message` / `send_greet` / `pull_thread` 以及发帖过程中的跳转，没有 pending / 恢复。
- 后台发令表单；创建 Signal 的 payload 仍写死 `{ source: "admin-ui" }`。
- 媒体分片下载与回退、线程增量游标、设备侧带 Cookie 的请求桥（发快拍若必须走移动端接口且网页 CORS 不可用时）。
- 不得把 SP 的 sessionStorage-only、AndroidBridge、脚本内 JWT 密钥原样搬来。

## Pipeline 与 execute（Onia 等价设计）

SP 的 `executeAction` 做了三件事：统一入口、sessionStorage 里的 pending/断点、跳转后再注入并续跑；另有 `SESSION_ONLY_ACTIONS`（发帖、关注、点赞、评论、Insights、查资料等只校验会话、不看当前页 DOM）。Onia 要对齐这些**能力**，但状态权威放在 Flutter 运行时，按 `behavior_id` 关联，并与已有注入、`trace_id` 衔接。

### 入口

保持 `SocialProxy.execute(action, params, host)`。内部先做前置检查，再进动作。新增 `SocialProxy.resume(checkpoint, host)`，供跳转并重新注入之后由宿主调用。不要让服务端下发「步骤数组」那种第二套 pipeline（SP 的 `executePipeline` 本轮不做）；多步骤只存在于**单个动作内部**。

前置检查：

- **只校验会话**的动作：`get_profile`、`follow_user`、`like_post`、`comment_post`、`get_media_insights`、`create_post`、`create_story`，以及已有的 `sync_profile`（若其 live 实现也不依赖当前 DOM）。使用现有 `probeSession` / 会话探测，不要只看页面 title。失败回写 `session` 或 `platform_risk`。
- **与页面相关**的动作：`pull_thread`、`send_message`、`send_greet`，以及如需停留在收件箱页的 `pull_inbox`。会话无效同样失败；页面不对则由动作自己导航，并通过 pending 挂起，而不是当成登录失败。

`host.demo === true` 时所有新动作返回模拟数据，**绝不访问 Instagram**（`apps/dev-device` 继续走这条，便于先测发令页和追踪）。手机验收路径必须 `demo: false`，失败不得回落演示数据。

### 状态存在哪、谁恢复

| 层 | 存什么 | 职责 |
|----|--------|------|
| 服务端 | Behavior 未终态 | 只在收到终态 `behavior.result` 后改状态；hello 后按原 `behavior_id` 重派。不要把脚本断点存进服务端。 |
| Flutter 运行时（权威） | 按 `behavior_id`：动作、params、host 快照、`trace_id`、脚本返回的 checkpoint、是否挂起 | 收到 `behavior.execute` 后调用 `execute`；若结果是 pending，**不**回写 result，清掉「整段 RPC」超时，等下一次 `onPageFinished` 注入同一份 `social_proxy.js` 后调用 `resume`。进程被杀则等服务端重派，再按 `behavior_id` 幂等（已有终态结果就原样回写，有 checkpoint 就 resume，不要无断点地重跑已产生副作用的步骤）。 |
| 脚本内 | 可选 sessionStorage，仅作同一次 WebView 会话的加速缓存 | 以宿主传入的 checkpoint 为准。校验 `validateCtx`（例如目标用户名、文案一致）失败则视为新执行，丢弃脏断点。 |

pending 的约定（写入协议的脚本/宿主约定即可，不新增 kind）：动作或某一步返回 `{ pending: true, checkpoint: { ... } }`。脚本在导航前通过现有 `OniaBridge` 把 pending 发给宿主，避免页面卸载时 Promise 被丢、宿主当成超时。`behavior.result` 仍只表示成功或失败终态。

不要发明一套「必须在多少毫秒内恢复」的承诺。断点只对当前 Behavior 有效；过期或校验失败就失败回写 `script` / `host`，由 Todo 策略决定是否重试。`session` / `platform_risk` 仍不按网络错误重试。

跳转后注入与现在一样：只注入这一份 bundle，且只在 `instagram.com` 源注入。`trace_id`、`behavior_id` 放进 `HostContext` 或 resume 参数，日志与回写必须带上它们。

## 依赖能力：本轮必须 vs 后置

**本轮必须进脚本包（同一份 bundle 内的模块，不是额外下发文件）：**

- 同源 GraphQL / 网页请求辅助（对标 `insGraphql` 的职责：在已登录页发同源请求、解析资料与媒体标识）。查询标识和页面结构属于脚本维护点，不要写进协议，也不要把 sitin 源码拷进来。
- 导航与页面辅助（对标 `navigateToInbox` / `navigateToChat` / `navigateToProfile`、`clickProfileMessageButton` 等）：只作为脚本内部函数，不登记为控制台动作。
- 脚本内 `pipeline`（步骤、pending、resume）。
- 会话探测：沿用现有实现。

**本轮必须进宿主 / 服务端：**

- Flutter：pending / resume、跳转后再注入再续跑；把 Instagram 的 httpOnly 会话查询留在设备上（宿主可读 Cookie 是否存在或配合脚本探测），**禁止**把 `sessionid` 做成发令参数，也禁止脚本把会话传到 Onia 服务端。
- 媒体分片：能力 `media.range_download.v1`。脚本向宿主要分片（`url` + `offset` + `size`），宿主优先本机直连 HTTPS 下载；失败再带 **Onia 设备 token** 请求服务端 Range 回退。服务端只允许 https、拒绝回环和链路本地地址，可再加主机白名单配置。返回字节给宿主，由宿主交给脚本。这是 SP `mediaChunk` + 写死 `https://social-proxy-api-dev.sitin.ai/script/media-chunk` 的替代。
- 设备侧 HTTP 桥（仅本机、使用 WebView Cookie 罐）：给 `create_story`（以及如网页 CORS 挡住的上传）在设备上发请求。Cookie 不出设备，Onia 服务端不代打 Instagram。
- `pull_thread` 增量：不要 Android 本地库。在 PostgreSQL 增加按 `platform + account_id + thread_id` 保存的游标（`last_message_id`、是否已完成一次全量）。`pull_thread` 成功回写后由服务端更新。策略在创建 Todo 时若表单未填 `after_message_id`，则自动带上游标。消息正文走 `behavior.result`，不要再发明脚本直传业务库的旁路。

**jwtAuth 的替代（不要搬密钥进客户端）：**

SP 脚本用与服务端共享的 HS256 密钥签发短效 JWT 去调媒体分片接口。这和 Onia「密钥不进构建物、会话不出设备」冲突。本轮禁止在 `social_proxy.js` 或 Flutter 资源里放入任何共享 HMAC/JWT 密钥。媒体回退鉴权只用已有的设备登录 token；脚本只调用宿主，不自己签 token。

**本轮后置：** 脚本 OTA、服务端下发脚本正文、Agent / `send_reply`、系统推送叫醒、把媒体 Range 做成通用任意 URL 代理、Story 的服务端洗视频或 `resultUrl` 回调、Snapchat。

## 各动作：params / result 契约

字段用 snake_case。服务端与 `packages/protocol-ts` 由 `crates/onia-protocol` 生成。下列「结果」是 `behavior.result.result` 的业务对象；外层仍是 `{ ok, result, error_class, error_message }`。

**共同：** 服务端写入的 `platform`、`account_id` 必须出现在下发 params 中。live 路径若页内会话身份与 `account_id` 不一致，失败并走重新登录或重新绑定，不得把结果写到错误账号上。

| 动作 | 表单 / params | 结果要点 |
|------|----------------|----------|
| `pull_inbox` | 无额外必填（沿用现契约：可选由服务端填身份） | 现有 `threads[]`，不要改坏 |
| `sync_profile` | 无额外必填 | 现有 handle / display_name / follower_count 等 |
| `get_profile` | **必填** `target_handle`（用户名） | 目标用户的稳定 ID、handle、显示名、简介、外链等已能稳定拿到的字段 |
| `follow_user` | **必填** `target_handle` | `target_handle`、是否已关注/已发出请求、内部 user id（若有） |
| `pull_thread` | **选填** `target_handle`、`thread_id`、`after_message_id`。两者都空则按收件箱未读/当前会话做增量（脚本可先导航收件箱）。有目标则进入对应线程 | `thread_id`、`target_handle`、`messages[]`（平台消息 id、方向、正文、时间）、`last_message_id`、`full_sync_done` |
| `send_message` | **必填** `target_handle`、`texts`（非空字符串数组，至少 1 条）。**选填** `thread_id`、`pre_send_anchor_mid` | 发送条数、`thread_id`、已发出的平台消息 id（若能读到） |
| `send_greet` | **必填** `target_handle`、`text`（非空）。SP 原实现假定已在主页且只有 `text`；控制台不能做这个假定，Onia 必须带目标用户并在需要时先导航主页 | `target_handle`、`sent: true` |
| `like_post` | **必填其一** `short_code` 或可解析的 `post_url`（`/p/` 或 `/reel/`）。**选填** `media_kind`（`reel` \| `post`，仅用于补全 URL）、`author_handle` | `short_code`、是否已赞 |
| `comment_post` | 同上，另 **必填** `content`。**选填** `reply_to_comment_id`、`tracking_token`（高级；一般不必填，脚本可自取） | 评论 id、正文 |
| `get_media_insights` | **必填** `media_id`（帖子 PK；兼容 insights URL，服务端或脚本解析出数字 ID） | `media_id` 与指标列表（名称 + 数值；缺指标不要用演示数字填） |
| `create_post` | **必填** `file_urls`（1–10 个 https URL，顺序即帖子顺序）。**选填** `caption`（最长 2200 字）、`client_post_key`（调用方标识，原样回传） | `media_id` / `short_code` / `post_url`（能拿到的）、`client_post_key` |
| `create_story` | **必填** `file_urls`，长度必须为 1。**选填** `link`（贴纸链接）、`link_x` / `link_y` / `link_width` / `link_height`（0–1，缺省由脚本用合理默认，0 视为缺省）、`caption`、`client_post_key`。**禁止**表单出现 `sessionid`、`chunk_base`、`result_url`、`result_token` | `media_id`（若有）、`client_post_key` |

校验要点：`file_urls` 必须是 https；`texts` / `text` / `content` 去空白后非空；`media_kind` 只能是枚举。不要接收 SP 的 `chunkBase`、`sessionid`、`resultUrl`。

## 后台发令页

新建 `apps/admin` 页面（例如 `/commands` 或 `/signals/dispatch`），入口放在现有布局导航里，标题用「发令」。不要把动态表单塞进现有「创建 Signal」把两种语义混在一页（旧页保留）。

页面结构：

1. 选择已绑定设备及其当前平台账号（复用现有设备列表；离线仍允许提交，Todo 保持 pending，文案写明离线不是失败）。
2. 选择动作（下拉，显示 Onia 动作名，旁注中文，例如「关注用户 `follow_user`」；可在说明里写 SP 旧名，但提交值必须是 Onia 名）。
3. 按动作切换的表单（Ant Design Form），字段见上表：必填/选填、类型（字符串、字符串数组、枚举、URL 列表）。`file_urls` / `texts` 用可增减的列表控件。
4. 提交：`POST /v1/admin/signals/create`，类型 `MANUAL_ACTION`，payload 为 `{ action, params }`，带新的 `idempotency_key`（同一次点击的重试沿用同一键；用户再次主动提交用新键，避免点赞/关注被静默吞掉）。
5. 成功后跳到现有追踪页，带上 `signal_id`，能看到 `signal → todo → behavior` 以及 `behavior.result`。

追踪页尽量直接展示 `action` 与结果摘要，避免运营只看到 UUID。本轮若改动很小即可，不要重写整页。

`apps/dev-device` 为本轮所有新动作提供 demo 桩，便于先验收发令与追踪，而不必打开 Instagram。

## 失败、兼容与风险

失败分类沿用现表，不新增 class。页面跳转中的 pending 不是失败。导航失败、选择器失效归 `script`；媒体 URL 不可达或 Range 回退失败归 `network` 或 `host`（视发生在脚本还是宿主）；会话失效 `session`；检查点 `platform_risk`。

协议只增可缺省或新增类型/cap/动作名，不删改现有 `pull_inbox` / `sync_profile` 字段，不升主版本 `v`。`just gen` 后提交生成物与协议一起给 Git 助手，本轮实现者不要手改 `packages/protocol-ts`。

风险（实现时按失败表处理，不要用 demo 交差）：Instagram 网页接口和页面结构会变；嵌入式 WebView 可能挑战登录；发帖/快拍/私信属自动化，可能触发平台限制，禁止失败 tight loop；手机必须同时到达 Onia 服务端和 `instagram.com`。参考 `f31b49a77` 时禁止把 sitin 的环境 URL、JWT 密钥或私有桥协议带进 Onia。

## 本地联调（实现完成后由 Coder 带着用户做）

1. 不要切 `sitin-next` 分支，不要改 sitin-next，不要代登 Instagram、不要代下发 Signal、不要装 APK（除非用户另行明确要求）。
2. 在 `/Users/colna/WORK/onia` 起服务端、后台、可选 `just dev-device`。先用模拟器 + demo 把发令页、每种表单校验、追踪回写跑通。
3. 真机再走 live：`api_base` 指向当时局域网地址（曾核对过 `192.168.3.24:8080`，以当时网卡地址为准），已登录真实 Instagram，`host.demo = false`。每种动作至少一次成功回包不是演示数据；对 `send_message` / `send_greet` / `pull_thread` 还要验证「跳转后仍能续跑并最终在追踪里看到结果」。
4. 回归：`INBOX_ACTIVITY` / `PROFILE_SYNC_NEEDED` 旧入口仍可用；登录 path 判断、`ONIA_WEBVIEW_INSPECT` 默认关闭不被改坏。

## Coder 需要改的目录和顺序

不要先改信封字段名，不要做 OTA，不要接 FFI，不要回退未提交的登录与注入改动。建议按此顺序，每步可验证再往下：

1. **`crates/onia-protocol`**：动作名、cap、`SIGNAL_MANUAL_ACTION`、Todo 类型、各动作 params/result 类型。`just gen`。
2. **`crates/onia-policy` + `crates/onia-signal` + `crates/onia-server`**：登记 `MANUAL_ACTION`、校验 params、补媒体 Range 回退接口与线程游标表（sqlx 迁移，向前兼容）。Scheduler 晋升时检查所需 cap。不改 `behavior.execute` 形状。
3. **`apps/admin`**：新发令页与表单；旧创建页保留。类型来自生成的 `protocol-ts`。
4. **`packages/scripts`**：pipeline / resume；同源请求与导航辅助；各动作 live + demo 分支；`execute` 的 switch 登记新动作。仍从 `src/browser.ts` 打**一份** IIFE。补测试：无会话时 live 抛 `session`、demo 不访问 Instagram、pending checkpoint 可 resume。
5. **`apps/client`**：`OniaBridge` 增加 pending / 媒体分片 /（如需要）设备 HTTP；`DeviceRuntime` 按 `behavior_id` 挂起和 resume；hello 在会话就绪后申报本轮已实现 cap；延长或取消「整段执行」超时，使其不误杀 pending。不要改登录 URL 与 inspect 默认值。
6. **`apps/dev-device`**：demo 桩覆盖新动作，便于先测后台。
7. 更新 `apps/client/README.md` 与根 README 中与发令、打包脚本相关的说明。

实现完成后把进度和如何复现直接告诉用户。不要 commit / push，仓库与 MR 交给 Git 助手。随后的代码评审交给 Code Reviewer，本方案不代替评审。

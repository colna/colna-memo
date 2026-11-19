---
title: Onia 官方 API 入站触发：可迁移事件与学习/设计清单
date: 2026-11-19
tags: project, onia, instagram, official-api, webhook, inbound, architecture, private-replies
---

# Onia 官方 API 入站触发：可迁移事件与学习/设计清单

更新于 2026-11-19（亚洲/上海）。

> **已拍板（2026-11-19）**
>
> - **永久禁止** cookie / `sessionid` / 私有 GraphQL·Direct·CDN。
> - **可选允许** OAuth 的 Graph / Page token，只打官方端点。
> - **一期按模式 A 推进**：Webhook 入站 → Signal → 会话视图 → Agent 草稿 → Policy；`send_reply` 仍由设备发送；默认 `review`。
> - **评论转私信**：官方 comments + Private Replies（模式 A 下的局部官方发送特例），**不做 WebView**。
> - **模式 B**（普通 DM 官方代发）为**显式二期开关，不默认开启**。

本文回答两件事：

1. **哪些入站事件**可以从本机 WebView / 私有接口监听，迁到 Meta **官方云端实时触发（Webhook）**；
2. 为实现「官方 API 获客触发」（尤其评论转私信），需要补齐哪些**技术学习与设计项**。

本文是技术架构成稿。**§8 硬约束改写已于 2026-11-19 拍板**，正文已按该决议修订。**不编造** QPS、延迟、容量或用量数字；本文不改 Onia 代码仓；不把竞品商业模式写成已定产品。

**真源**：同目录 [onia-status-overview.md](onia-status-overview.md)、[architecture.md](architecture.md)、[instagram-auto-reply.md](instagram-auto-reply.md)、[instagram-dm-sync.md](instagram-dm-sync.md)、[instagram-actions-pipeline.md](instagram-actions-pipeline.md)、[instagram-chat-agent-kotlin.md](instagram-chat-agent-kotlin.md)、[onia-business-model.md](onia-business-model.md)。平台事实以 Meta 公开文档为准（链接见文末「出处」）；不确定处标 **假设**。

**商业侧已对齐、本文必须遵守的产品判断**（不得写成相反结论；技术做不到则标明冲突）：

| # | 判断 |
|---|------|
| 1 | Instagram 官方消息 / 评论私信接口本身**一般不按条向 Meta 付费**（与 WhatsApp 不同）。服务端持 access token 并代发，曾与旧硬约束「凡打向 IG 的请求只出设备、服务端不持 cookie、不代发」冲突。**已拍板（2026-11-19）**：永久禁止 cookie / 私有面；可选允许 OAuth token 只打官方端点；一期模式 A（普通 DM 不默认官方代发）；评论 Private Reply 为局部特例。不得把 Graph token 混成 cookie，也不得静默取消「永不持 cookie」一条。 |
| 2 | 官方云端比本机 WebView / 私有接口更不易因「非官方脚本、异常登录态」被一刀切，但**滥用同样会被限**；不是零风险。官方路径通常需要**专业号 / 商业号**。 |
| 3 | **24 小时消息窗口**：对方先有动作才可继续发；评论转私信通常**一条评论只能先私回一条**，对方回复后才进入窗口。 |
| 4 | **一期不做**：营销广播 / 无限群发、WhatsApp 付费消息、粉丝积分 / 白标、用非官方脚本仿评论转私信或关注解锁。 |

---

## 0. 结论先行（已拍板，2026-11-19）

1. **最高优先迁移**：`DM_RECEIVED` 的触发源，从 WebView `start_dm_listener` 迁到 Instagram Messaging **`messages` Webhook**。设备离线仍可收到新私信事实。
2. **一期已拍板 = 模式 A「仅云端入站」**：Webhook 入站 → 仍走现有 Signal → 会话视图 → Agent 草稿 → Policy → **`send_reply` 仍由设备发送**。默认 **`review`**；`auto` 为显式开关 + 审计。官方 Conversation 读历史不纳入一期默认（二期或实验开关且默认关）。
3. **模式 B「云端入站 + 官方代发」**为**显式二期开关**，不默认打开。硬约束改写已拍板，但不等于一期默认开启模式 B。
4. **评论转私信、Story 回复/提及等本地尚未闭环的获客触发，不要再做成 WebView 监听**——应直接官方 Webhook + Private Replies。评论转私信是模式 A 下的**局部官方发送特例**。
5. **硬约束改写已拍板（2026-11-19）**：把「服务端不得碰 Instagram」细化为——**永久禁止**持 WebView `cookie` / `sessionid`、禁止打 **私有 GraphQL / Direct / CDN 反代**；**可选允许**经用户 OAuth 取得的 **Graph / Page access token**，仅调用 Meta **官方 Graph / Messenger 端点**。二者不可混为一谈。
6. **计费口径**：Messenger Platform 文档写明 *「The Messenger Platform is free to use」*；未见 Instagram Messaging 按条向 Meta 收费的公开价目（与 WhatsApp 不同）。产品侧仍可能向用户收 SaaS 费；滥用与窗口违规仍会被平台限制。

---

## 1. 现状对照

### 1.1 现行入站怎么来

| 路径 | 机制 | 依赖 | 状态（至 2026-11-19） |
|------|------|------|------------------------|
| 主路径 | WebView 登录后自动 `start_dm_listener` → `signal.report` → `DM_RECEIVED` | 设备在线、WebView 登录态 | 已闭环（阶段 A/B） |
| 弱路径 | `INBOX_ACTIVITY` → `pull_inbox` → 规范 `thread_fbid` → `THREAD_PULL_NEEDED` → `pull_thread` | 设备执行脚本 | 已有；补漏用 |
| 拉会话 | `pull_thread`（GraphQL 全量/增量 + DOM 回退） | 设备 | 已有；不是实时触发 |
| 自动回复 | 视图更新 → Agent 写草稿 → Policy → `send_reply` Behavior → 设备发 | 默认 `review` | Mock 真机闭环；Kotlin Agent 已接入 |

### 1.2 硬约束（2026-11-19 已拍板；与 status overview §4.1 对齐）

拍板前「凡打向 IG 的请求只出设备、服务端不持 cookie、不代发」的笼统表述，已按下列文案改写：

1. 凡打向 Instagram **私有域 / CDN / GraphQL / Direct 私有接口**的请求，**只能由设备端发出**；服务端只调度与收结果。
2. 服务端**永久不得**持 WebView cookie / `sessionid` 等会话材料，不得反代 IG，不得用服务端出口拉 IG **私有**媒体 URL。
3. **可选允许**：经用户 OAuth 取得的 Graph / Page access token，仅调用 Meta **官方 Graph / Messenger 端点**（Official Channel）。不得用该 token 打私有接口。
4. 一期默认**不代发**普通 DM：`send_reply` 仍走设备（模式 A）。评论转私信走官方 Private Replies（局部官方发送特例）。模式 B 全量官方代发为显式二期开关，不默认开。官方发送失败不得静默回落私有接口。
5. Agent **只写草稿**，不发消息、不持社媒会话、不操作 WebView、不直接创建发送 Todo。
6. 禁止在 Behavior 里写死「拉完再回复」；必须事实 / 视图更新后再进 Policy。
7. 不手造 `message_id`；敏感值不进日志 / 测试 / memo。
8. 不迁入公司私有仓源码与内部接口。

### 1.3 与「学官方 API 获客」的张力

- 竞品簇（ManyChat / DripDM / Kwikzy / ReplyKaro 等）主打 **评论转私信、关键词、Story 触发**，走 **Meta 官方 Messaging / Graph**，云端收发。
- Onia 今日是 **Social Proxy**：本机登录 + 私有接口执行 + 服务端不持 cookie。
- 若要「官方获客触发」又不静默取消硬约束，必须 **拆通道**，而不是把 Graph token 当成 cookie 混进现有 Device Channel。

---

## 2. 推荐架构：双通道共存

### 2.1 拆 Official Channel 与 Device Channel

同一套领域管线不变：

```
Signal → Policy → Todo → Scheduler → OutboundAdapter → 结果回写
                              ↑
                         Agent 只写草稿
```

新增**出站适配**与**入站适配**分叉，禁止混成「服务端打私有接口」。

```
                    ┌─────────────────────────────────────┐
  Meta Webhook ───►│ Official Channel                     │
  (messages /      │ · 验签 / 订阅校验                     │
   comments / …)   │ · 持 Graph / Page access token       │
                    │ · 映射为平台中立 Signal               │
                    │ · 可选：Conversation API 拉历史       │
                    │ · 可选（模式 B）：Send / Private Reply │
                    └──────────────┬──────────────────────┘
                                   │ ingest Signal
                                   ▼
                    ┌─────────────────────────────────────┐
                    │ onia-server 领域层（不变）            │
                    │ Signal · 会话视图 · Policy · Todo    │
                    │ Agent(chat-agent) · Scheduler         │
                    └──────────────┬──────────────────────┘
                                   │ Todo 晋升
              ┌────────────────────┴────────────────────┐
              ▼                                         ▼
   Device OutboundAdapter                    Official OutboundAdapter
   (现有 Behavior → WebView 脚本)            (仅模式 B：Graph Send API)
   send_reply / send_message / …             /PAGE_ID/messages 等
              │                                         │
              ▼                                         ▼
     本机 cookie / sessionid                      Graph access token
     （永不上传服务端）                            （OAuth，加密存放）
```

### 2.2 两种模式（已拍板）

| 模式 | 入站 | 出站 | 服务端持有物 | 与硬约束关系 | 推荐阶段 |
|------|------|------|--------------|--------------|----------|
| **A：仅云端触发** | Webhook → Signal | **仍走设备** `send_reply` 等 | Webhook 验签密钥；官方 Conversation 读历史**不纳入一期默认**（二期或实验开关且默认关） | **不**持 cookie；**不**打私有接口。评论 Private Reply 为一期局部官方发送特例 | **一期已拍板默认** |
| **B：云端触发 + 官方代发** | 同 A | Official Send / Private Replies | Graph / Page token（读写） | 硬约束已拍板改写：允许官方端点代发，但**不得默认打开**；失败不得静默回落私有接口 | **二期显式开关** |

**一期已拍板**：模式 A。理由：

- 解决「设备离线收不到新私信 / 评论」的最大痛点；
- Agent、Policy、`review` 默认、审计链可原样复用；
- 发送权仍在用户设备，与现有安全叙事最接近；
- 评论转私信若一期就要做：**Private Reply 本身是一次官方发送**——见 §2.3 特例。

### 2.3 评论转私信的特例（模式 A 下的局部模式 B）

Private Replies 是 `POST .../messages`，`recipient: { comment_id }`，**只能由持 token 的服务端（或经官方授权的后端）发出**，无法用「设备 WebView 仿一条」且一期明确禁止非官方仿制。

因此：

- **评论 → 首条私信**：若产品要做，**必须**走 Official Outbound（局部模式 B），即使 DM 普通回复仍走设备。
- **对方回复之后的续聊**：可继续模式 A（设备发）或模式 B（官方发）；需在账号设置里写清，避免同一会话双通道乱序。

**已拍板默认（2026-11-19）**：

1. 账号未开通官方通道 → 仅 Device Channel（现状）。
2. 开通官方通道（一期默认，模式 A）→ Webhook 入站 + 普通 DM 仍设备发送；**评论转私信直接走官方 Private Replies**（局部官方发送特例），不做 WebView。
3. 显式开启「允许官方代发」（模式 B，普通 DM 也走官方 Send）→ **二期显式开关，不默认开启**。

### 2.4 Graph access token ≠ WebView cookie

| | Graph / Page access token | WebView cookie / sessionid |
|--|---------------------------|----------------------------|
| 来源 | 用户 OAuth / Facebook Login for Business / Instagram Login | 浏览器登录态，私有 |
| 调用面 | `graph.facebook.com` / `graph.instagram.com` 官方端点 | Instagram Web GraphQL、Direct、CDN 等私有面 |
| 硬约束（已拍板） | **可选允许**，且只打官方端点 | **永久禁止**上传与服务端持有 |
| 风险 | App Review、权限、窗口与速率限制、token 泄露 | 非官方自动化、异常登录态、封号叙事更敏感 |
| 媒体 | 官方 CDN / 附件上传 API（有平台规则） | 设备本机拉再上传（现行） |

**并存禁令**：同一条出站 Todo 不得「官方失败再静默改打私有接口」；失败应回写 Todo / Event，由 Policy 或人工决定是否改走另一通道。

---

## 3. 一、可挪到云端的事件

### 3.1 已有本机闭环 → 建议迁云端触发

#### 3.1.1 `DM_RECEIVED`（最高优先）

| 项 | 内容 |
|----|------|
| 今日 | WebView `start_dm_listener` → `signal.report` |
| 官方对应 | Instagram Messaging Webhook 字段 **`messages`**（客户发来文本/媒体/分享等；己方发出带 `is_echo`） |
| 迁完收益 | 设备离线仍可 ingest 新私信事实；弱化对常开 WebView 的依赖 |
| 映射 | Webhook `message.mid` → 幂等键 `platform + message_id`（与现行一致，不手造 id） |
| 注意 | GIF/贴纸等文档标明部分场景不触发 webhook；消失媒体无 URL——会话视图需容忍「有 mid 无正文」并可选再派设备 `pull_thread` 补全 |
| 主路径建议 | **云端入站 →（必要时仍 pull_thread 补全）→ 会话视图 → Agent 草稿 → Policy**；模式 A 下 `send_reply` 仍设备 |

#### 3.1.2 `INBOX_ACTIVITY`

| 项 | 内容 |
|----|------|
| 今日 | 弱路径：本地发现未读 → 再 `pull_inbox` |
| 迁完后 | 有可靠 `messages` Webhook 后，**应收敛**：不再作为主触发；可保留为「webhook 漏报 / 个人号无官方通道」时的兜底 |
| 建议 | 官方通道账号：降级为可选巡检，不默认高频轮询 |

#### 3.1.3 `THREAD_PULL_NEEDED` / `pull_thread`

| 项 | 内容 |
|----|------|
| 性质 | **不是**实时触发，是补历史 / 补正文 |
| 官方替代（若接受服务端持 token） | Messenger / Instagram **Conversation API** 等官方会话读取（具体端点以实现时文档版本为准） |
| 模式 A 默认 | 仍可派设备 `pull_thread`，零硬约束冲突 |
| 模式 A+（可选） | 云端官方拉历史，减少派设备翻会话；**需 token**，属硬约束改写范围 |
| 禁止 | 服务端用 WebView cookie 打私有 GraphQL 拉线程 |

### 3.2 迁完后建议主路径

```
官方 Webhook（或设备兜底 signal.report）
  → 幂等 Signal（DM_RECEIVED / COMMENT_RECEIVED / …）
  → Policy（收件箱包 / 回复包 / 新「获客触发包」）
  → 必要时 Todo：补拉会话（设备或 Official Read）
  → 会话视图更新
  → Agent 写 reply_drafts（仍不投递）
  → REPLY_DRAFT_READY
  → Policy：review → 待审；auto → SEND_REPLY；suggest → 仅草稿
  → OutboundAdapter：
       模式 A → Device Behavior send_reply
       模式 B → Official Send API
```

默认 **`review`**；`auto` 必须账号级显式开启 + 审计日志（与现架构一致）。

### 3.3 本地尚未闭环 → 不要做 WebView 监听，应直接云端触发

| 产品能力 | 官方机制（公开文档） | Onia Signal 建议 | 优先级 |
|----------|----------------------|------------------|--------|
| 帖子 / Reels / 广告新评论 | Instagram Webhooks **`comments`**（广告评论可能带 `ad_id` / `ad_title`，可能重复通知） | `COMMENT_RECEIVED`（`channel=comment`） | 获客一期核心 |
| Live 评论 | **`live_comments`**（与 `comments` 区分；Live 期间负载更高） | `LIVE_COMMENT_RECEIVED` 或同类型加 `live=true` | 可二期 |
| 评论转私信 | **Private Replies**：用 `comment_id` 调 Send；**每评论仅一条**；帖/Reels/广告评论创建后 **7 天内**可发；Live 仅直播中可发；对方回复后才进 24h 窗口 | Policy「获客包」→ Official Private Reply Todo | 获客一期核心 |
| Story 回复 | `messages` 内 story reply 形态 | `DM_RECEIVED` + `source=story_reply` 或独立类型 | 一期可收事件，完整产品化可后置 |
| Story 提及 | `messages`：文档写明 **仅 story mention 触发**；普通帖 @ 不走 messaging webhook。另有 Graph **`mentions`**（评论/文案 @） | `STORY_MENTION` / `MENTION_RECEIVED` | 一期可收，产品化后置 |
| 私信关键词 | 非独立 webhook；在 `messages` 入站后由 **规则引擎 / Policy** 匹配 | 仍 `DM_RECEIVED` + 规则 | 一期（规则简单版） |
| 按钮 / 快捷回复 / Icebreaker | **`messaging_postbacks`**；`messages` 亦可能含 quick reply 选择 | `POSTBACK_RECEIVED` | 一期（配合 Opening DM） |
| 消息已读 | **`messaging_seen`** | `MESSAGE_SEEN`（审计或弱策略，默认不自动发） | 次优先 |
| 表情反应 | **`message_reactions`** | `MESSAGE_REACTION` | 次优先 |
| 公开评论回复 | Instagram Graph 评论回复接口（权限 `instagram_manage_comments` 等） | Todo → Official 或（过渡）设备；推荐官方 | 与「已私信」话术轮换一起学 |

### 3.4 不要包装成「云端实时触发」的

| 能力 | 原因 |
|------|------|
| `send_reply` / `send_message` / `send_greet` | **执行**动作，不是入站 Webhook；换通道是 OutboundAdapter 选择 |
| `follow_user` | 执行；且一期不把「关注解锁」当灰产主卖点 |
| `sync_profile`、定时 Signal、发帖 / 快拍 | 可用官方 Content Publishing 等，但是**出站/定时**，不是入站实时触发 |
| 个人号、官方 API 不覆盖的收件箱细节 | **必须留本机 Device Channel** |
| GIF/贴纸等文档标明不触发 webhook 的消息 | 只能设备兜底或接受不可见 |

---

## 4. 事件映射表（摘要）

| Onia 事实 / 动作 | 今日来源 | 官方字段 / API | 迁云端？ | 出站通道 |
|------------------|----------|----------------|----------|----------|
| `DM_RECEIVED` | `start_dm_listener` | `messages` | **是（优先）** | A: 设备 / B: Send API |
| `INBOX_ACTIVITY` | 本地未读探测 | （无直接等价；由 messages 收敛） | 收敛为兜底 | — |
| `THREAD_PULL_NEEDED` | Policy | Conversation API（可选） | 可选云端读 | 读：Official Read 或设备 |
| `COMMENT_RECEIVED` | 无闭环 | `comments` / `live_comments` | **直接云端** | 公开回复：Official；私信：Private Reply |
| Story 回复 / 提及 | 无 | `messages`（story）/ `mentions` | **直接云端** | 视产品 |
| Postback / 快捷回复 | 无 | `messaging_postbacks` | **直接云端** | 规则 / Agent |
| `MESSAGE_SEEN` | 无 | `messaging_seen` | 可云端 | 默认不自动发 |
| `MESSAGE_REACTION` | 无 | `message_reactions` | 可云端 | 次优先 |
| `send_reply` 等 | 设备脚本 | Send API（模式 B） | 否（执行） | A 或 B |
| `follow_user` | 设备 | 非「入站触发」 | 否 | 设备；勿灰产主卖 |

幂等：官方 `mid` / `comment_id` 进入 Signal 幂等键；广告评论重复 webhook 需去重（文档提示可能重复）。

---

## 5. 二、需要学习 / 设计的能力清单（按优先级）

### 5.1 先学（做评论转私信就必须会）

1. **Meta 登录与授权**
   - Instagram **Professional**（Business / Creator）；通常需关联 Facebook Page（Facebook Login 路径）或 Instagram Login 路径（以实现时文档为准）。
   - 权限示例（Messaging）：`instagram_basic`、`instagram_manage_messages`、`pages_manage_metadata` 等；评论：`instagram_manage_comments`、`pages_read_engagement` 等。
   - **App Review**、**Advanced Access**（服务非 Role 用户时）、**Business Verification**（文档在「非自己 Page」场景要求）。
   - Token：**存放加密、刷新、吊销、最小权限**；与 WebView session **物理隔离**（不同表、不同密钥、禁止日志打印）。

2. **Webhook 入站**
   - 订阅 `messages`、`comments`、`live_comments`、`messaging_postbacks`、`messaging_seen`、`message_reactions` 等按阶段开通。
   - 验签、挑战握手、重放保护、按 `mid`/`comment_id` 幂等。
   - 文档要求：应用需 **published** 才能收 webhook；未过审时往往只有 Role 用户有数据。

3. **评论转私信（Private Replies）**
   - 范围：指定帖 / 全部帖 / 下一篇（产品配置）；关键词匹配（自建规则）。
   - **一条评论先私回一条**；7 天窗口（Live 仅直播中）；对方回复后进入标准消息窗口。
   - 未关注用户可能进 Request 文件夹（文档说明）——产品文案需告知。

4. **24 小时窗口与 Opening DM / 按钮确认**
   - 标准窗口多由用户动作打开，通常 **24 小时**；Click-to-Direct 等广告入口文档写明**最长可至 7 天**（自用户点击广告后首条消息起算）——实现时按当时官方「Standard Messaging Window」页核对。
   - Icebreaker / 快捷回复 / 按钮 → `messaging_postbacks` 续上合规对话。
   - 窗口外**禁止**当普通营销续发；一期不做广播。

5. **公开评论回复**
   - 可轮换「已私信请查看收件箱」类话术；与 Private Reply 编排顺序（先公开短回 / 只私信 / 双发）要可配置。

6. **关注检查**
   - 可作为能力设计（发链接前是否已关注；未关注先引导）。
   - **不要**做成灰产主卖点；一期禁止用非官方脚本仿「关注解锁」。

7. **双通道并存规则**
   - Official Channel 与 Device Channel 配置、默认出站、失败是否允许切换（默认：**不允许静默切换到私有接口**）。
   - 身份映射：IGSID（官方）↔ 现有 `account_id` / 线程 id（设备私有面）——需设计对照表；对不上时降级人工或只记事实。

### 5.2 接着学

- 可视化规则 / 多步 Flow（延迟、分支、按钮）；落库为策略包，**仍经 Policy**，不在 Webhook handler 里写死业务。
- 链接点击追踪与「一次未点击 / 未回复」跟进（须遵守窗口与反垃圾）。
- 私信采集邮箱 / 电话（同意与存储合规另案）。
- 商品卡或固定话术模板（官方 Generic / Product template 等，按文档权限开通）。
- 默认 **`review`**；**`auto` 显式开关 + 审计日志**（草稿、通道、token 主体、操作者）。

### 5.3 可后置

- Story 完整产品化（今日可先收 webhook）。
- 历史评论补发（注意 7 天 Private Reply 限制）。
- 浅层 link-in-bio / 外链点开预填私信（`ig.me` / referral 等）。
- 规则与内容维度的转化数据看板。

---

## 6. 权限、审核门槛与平台限制（有来源才写）

### 6.1 账号与应用门槛

- Instagram Messaging 面向 **Instagram Professional account**（business 或 creator）。（[Messenger Platform Overview](https://developers.facebook.com/docs/messenger-platform/overview)）
- 收 Messaging webhooks 需相应权限；对非 Role 用户数据通常要 **App Review / Advanced Access**；应用需 **published**。（[Webhooks for Instagram Messaging](https://developers.facebook.com/documentation/business-messaging/instagram-messaging/webhooks)）
- Private Replies 文档列有 `instagram_manage_comments`、`pages_messaging`、Page token、Advanced Access 等前置。（[Private Replies](https://developers.facebook.com/docs/messenger-platform/instagram/features/private-replies)）

### 6.2 窗口与条数

- Private Reply：**每名评论用户仅一条**；帖/广告/Reels 评论后 **7 天内**；Live **仅直播中**；对方回复后才可在 **24h 窗口**内继续。（同上 Private Replies）
- 标准消息窗口：多数用户动作 **24 小时**；部分 Click-to-Messenger / Click-to-Direct 广告场景文档写明窗口**可至 7 天**。（[Messenger Platform Overview](https://developers.facebook.com/docs/messenger-platform/overview)、changelog 叙述）
- Send 文本长度等：官方 Send 文档写明文本限制（如小于约 1000 字符量级）与媒体格式/大小上限——实现时按 [Send a Message](https://developers.facebook.com/docs/messenger-platform/instagram/features/send-message/) 核对，本文不抄成不变常量表以外的「容量承诺」。

### 6.3 速率

- Meta 对 Instagram Messaging 有 **按专业账号的调用速率限制**（公开 Rate Limiting 文档提及每秒调用量级上限）。**本文不把具体数字当成 Onia 容量承诺**；实现与告警以当时 [Rate Limits](https://developers.facebook.com/docs/graph-api/overview/rate-limiting/) / Instagram Messaging 专页为准。
- 高并发 Live `live_comments` 需可区分事件类型并做好队列，避免拖垮入站服务。（Private Replies 文档提醒）

### 6.4 计费（Meta 侧）

- 官方表述：*「The Messenger Platform is free to use.»*（[Overview](https://developers.facebook.com/docs/messenger-platform/overview)）
- 公开开发者文档中**未见** Instagram Messaging / Private Replies **按条向 Meta 付费**的价目；这与 **WhatsApp** 商业消息计费不同。
- **假设（需产品与法务再确认）**：第三方工具向用户收的是 SaaS 费，不是 Meta 的 IG 按条费；Meta 政策、配额、审核要求仍可能变化。

### 6.5 封号与合规口径（产品约束）

- 官方路径相对本机私有脚本：**更不易**因「非官方客户端 / 异常登录态」被一刀切。
- **不是零风险**：窗口外群发、骚扰、违规关注引导等同样会受限或处罚。
- 一期明确不做广播与非官方仿获客。

---

## 7. 一期 / 二期范围

### 7.1 一期（建议）

- Meta App + Professional 账号授权闭环（自有测试号 → 再 App Review）。
- Webhook：`messages`、`comments`；（可选）`messaging_postbacks`。
- Signal 映射 + 幂等 + 与现有回复包衔接。
- **模式 A** 为默认：云端入站，**普通 DM 回复仍设备发送**；默认 `review`。
- 评论转私信：**仅官方 Private Reply**；关键词 + 指定帖配置；公开评论短回可选。
- 双通道配置与「禁止静默打私有接口」的开关与审计。
- 文档 / 控制台标明：个人号无官方通道时仍走 Device。

### 7.2 一期明确不做

- 营销广播 / 无限群发  
- WhatsApp 付费消息  
- 粉丝积分 / 白标  
- 非官方脚本仿评论转私信、仿关注解锁  

### 7.3 二期（建议）

- 模式 B：普通 DM **官方代发**（显式开关）。
- Official Conversation 读历史替代大部分 `pull_thread`。
- `messaging_seen` / `message_reactions` 策略化。
- Story 产品化、历史评论补发、Flow 可视化、转化分析、`ig.me` referral。
- Handover Protocol（多 App 共用收件箱时）。

---

## 8. 硬约束改写（已拍板，2026-11-19）

**已拍板日期：2026-11-19。** 下列改写取代拍板前「凡打向 IG 的请求只出设备、服务端不持 cookie、不代发」的笼统表述。

| ID | 拍板前约束 | 与官方路径的冲突 | 已拍板处置（2026-11-19） |
|----|----------|------------------|--------------------|
| C1 | 打向 IG 的请求只能由设备发出 | 模式 B 的 Send / Private Reply；模式 A+ 的 Conversation 读 | **已改写**：永久禁止**私有** GraphQL / Direct / CDN；**可选允许** Official Channel 调官方 Graph 端点 |
| C2 | 服务端不持 cookie / sessionid、不代发 | 持 **Graph token** 并代发 ≠ 持 cookie；「不代发」字面冲突模式 B 与 Private Reply | **已拆分**：永久不持 cookie / sessionid；一期默认不代发普通 DM（模式 A）；评论 Private Reply 为一期局部官方发送特例；模式 B 全量代发为显式二期开关，不默认开 |
| C3 | 服务端不反代 IG、不拉 IG 媒体 URL | 官方附件 / CDN 规则不同 | 官方媒体走官方方式；私有 CDN URL **仍永久禁止**服务端直拉 |
| C4 | Agent 只写草稿 | 无冲突 | **保持** |
| C5 | 不手造 message_id | 无冲突（用官方 mid / comment_id） | **保持** |
| C6 | 商业材料曾写「不是官方 Messaging 替代品」 | 若上官方通道，定位变为「双通道」 | 更新 [onia-business-model.md](onia-business-model.md) 叙事：**可选 Official 获客 + Device 执行**，而非否认官方 |

**已拍板结论**：

1. 永久禁止 cookie / `sessionid` / 私有 GraphQL·Direct·CDN。
2. 可选允许 OAuth 的 Graph / Page token，只打官方端点。
3. 一期按模式 A 推进：Webhook → Signal → 会话视图 → Agent 草稿 → Policy；`send_reply` 仍设备；默认 `review`。
4. 评论转私信按官方 comments + Private Replies，作为模式 A 下的局部官方发送特例；不做 WebView。
5. 模式 B 保持显式二期开关，不默认开启。

### 8.1 一期工程启动边界

一期工程**可以启动**的范围（与已拍板模式 A 对齐；现码进度见 §14 / §15，不以现码提前把模式 B 当成一期默认）：

1. 按已拍板文案同步协议与文档：Official Channel 与 Device Channel 分叉，禁止复用 DeviceSession 的 cookie / `sessionid` 路径。
2. Official Channel 模块：Webhook 验签与订阅校验、token 保险库（仅 OAuth Graph / Page token，加密存放、不回显、不进日志）、入站事件映射为平台中立 Signal。
3. 官方 Graph 调用面只允许 `graph.facebook.com` / `graph.instagram.com` 等官方端点；禁止用该 token 打私有 GraphQL / Direct / CDN。
4. 接通 `messages` Webhook → `DM_RECEIVED`（幂等键与设备监听一致）→ 会话视图 → Agent 只出草稿 → Policy；默认 `review`，`auto` 须显式开关 + 审计。
5. 普通 DM 的 `send_reply` / `send_message` / `send_greet` 仍走设备 Behavior，不在一期默认改为官方代发。
6. 评论转私信：官方 `comments`（及可选公开评论回复）+ Private Replies；一条评论先私回一条、窗口校验；**不要**再做 WebView 监听或非官方脚本仿制。
7. 控制台：官方通道开关与状态、评论规则、审核模式、Webhook / 官方发送审计；官方失败不得静默回落私有接口，也不得自动改走另一通道。
8. 未开通官方通道的个人号 / 设备账号，继续只走 Device Channel。
9. 设备 listener / `pull_inbox` 可作为官方 Webhook 漏报时的兜底，不得重新成为官方通道账号的主触发（收敛策略可配置，默认不高频轮询）。

一期**明确不启动或不得当作默认交付**的：

1. 模式 B：普通 DM 官方代发。可在协议与控制台预留显式开关，但默认关闭，属二期能力；不得在一期默认打开，也不得在官方发送失败时静默回落设备或私有接口。现码若已预留该开关（见 §15），仍按本条约束，不改变分期。
2. 用 Official Conversation API 云端拉历史替代大部分 `pull_thread`：属二期（或实验开关且默认关）。
3. 营销广播 / 无限群发、WhatsApp 付费消息、粉丝积分 / 白标。
4. 非官方脚本仿评论转私信、仿关注解锁。
5. Story 完整产品化、历史评论补发、Flow 可视化、转化看板、Handover Protocol：二期或更后。

拍板范围只授权上述一期边界，不构成「已完成 Meta App Review / 生产 token 已配置」的承诺。

---

## 9. 风险

| 风险 | 说明 | 缓解 |
|------|------|------|
| 审核与权限拖延 | App Review / 商业验证周期不可预测 | 先自有 Role 号打通；并行准备录屏与用例说明 |
| Webhook 漏报 / 延迟 | 平台侧或网络问题 | 设备 listener 作兜底；幂等去重 |
| 身份对不齐 | IGSID vs 私有 thread_fbid | 对照表；对不上则 review 人工 |
| 双通道双发 | 官方与设备各发一条 | 单会话绑定唯一 OutboundAdapter；发送锁 |
| Token 泄露 | 等价于官方发信权 | KMS/加密列、轮换、最小权限、审计 |
| 窗口违规 | 自动规则在窗口外发送 | Policy 查窗口状态；默认 review |
| 政策误读 | 计费/窗口细节变更 | 实现前再打开官方文档核对；本文标假设处不写死 |
| 产品预期 | 「上了官方 = 零封号」 | 对外口径：降低非官方脚本风险，**不是**零风险 |

---

## 10. 假设（单独列出）

1. 用户愿意为「官方获客」接受 **Professional 账号** 与 Meta 审核流程；个人号继续只走 Device Channel。  
2. 一期可接受「评论首条私信走官方、普通回复仍设备」的混合出站。  
3. Messenger Platform「free to use」在本文写作时适用于 Instagram Messaging 的收发与 Private Replies；**若 Meta 日后改计费，以新文档为准**。  
4. IGSID 与现有设备侧线程身份可以建立稳定映射；若不能，官方入站会话与设备发送会话可能暂时无法自动粘合，只能人审或只记录。  
5. 不把第三方博客的费率/配额数字当作 Meta 官方承诺；速率具体数字实现时再钉死到配置，不写入产品 SLA。  
6. 本文不假设 Onia 已成为 Meta Tech Provider；若走「代众多客户持 token」路径，审核与合同要求可能更高（需另案）。

---

## 11. 建议落地顺序（工程）

§8 已于 2026-11-19 拍板。一期启动边界以 §8.1 为准。

1. 文档与硬约束改写提案评审（本文 §8）→ **已完成**：用户拍板模式 A / Private Reply 特例，模式 B 留作二期显式开关。
2. `onia-protocol` 增加官方入站 Signal 类型与 `outbound_channel` 字段（设计评审）。
3. 独立 `Official Channel` 模块：Webhook 入口、token 保险库、映射器——**禁止**复用 DeviceSession 的 cookie 路径。
4. 先接通 `messages` → `DM_RECEIVED` 与现有回复包（模式 A）。
5. 再接通 `comments` + Private Reply 获客包（不做 WebView）。
6. 控制台：通道状态、审核模式、审计、失败不静默降级私有接口。
7. 模式 B 全量代发另立里程碑（显式开关，默认关）。

---

## 12. 出处（Meta / 公开文档）

- Messenger Platform Overview（含 *free to use*、专业号、窗口概述）：https://developers.facebook.com/docs/messenger-platform/overview  
- Webhooks for Instagram Messaging：https://developers.facebook.com/documentation/business-messaging/instagram-messaging/webhooks  
- Instagram Webhooks 字段参考（comments / messages / mentions / messaging_seen 等）：https://developers.facebook.com/docs/graph-api/webhooks/reference/instagram/  
- Webhooks for Instagram（comments / mentions / story_insights 等）：https://developers.facebook.com/docs/graph-api/webhooks/getting-started/webhooks-for-instagram/  
- Private Replies：https://developers.facebook.com/docs/messenger-platform/instagram/features/private-replies  
- Send a Message（Instagram）：https://developers.facebook.com/docs/messenger-platform/instagram/features/send-message/  
- Graph API Rate Limiting：https://developers.facebook.com/docs/graph-api/overview/rate-limiting/  
- App Review（Instagram Messaging）：https://developers.facebook.com/documentation/business-messaging/instagram-messaging/app-review  

（权限名以 https://developers.facebook.com/docs/permissions/ 当时列表为准。）

---

## 13. 相关 Onia 文档

- [onia-status-overview.md](onia-status-overview.md) — 功能与硬约束快照  
- [architecture.md](architecture.md) — Signal → Policy → Todo → Behavior  
- [instagram-dm-sync.md](instagram-dm-sync.md) — 监听与拉会话  
- [instagram-auto-reply.md](instagram-auto-reply.md) — 阶段 C 与 review/auto  
- [instagram-chat-agent-kotlin.md](instagram-chat-agent-kotlin.md) — Agent 只出草稿  
- [onia-business-model.md](onia-business-model.md) — 商业叙事（上官方通道后需改「非官方替代品」表述）


## 14. 落地状态（2026-11-19）

已按本文 §11 顺序在 `/Users/colna/WORK/onia` 落地一期（模式 A + 评论 Private Reply）：

- **协议**：新增 `COMMENT_RECEIVED` / `LIVE_COMMENT_RECEIVED` / `STORY_MENTION_RECEIVED` / `MENTION_RECEIVED` / `POSTBACK_RECEIVED` / `MESSAGE_SEEN` / `MESSAGE_REACTION`；`SOURCE_WEBHOOK`；`SEND_PRIVATE_REPLY` / `REPLY_COMMENT` 动作与 Todo；`outbound_channel` / `official_channel_id`。
- **迁移 0016**：`official_channels`（Graph token AES-256-GCM 加密、token hint、模式/开关、绑定设备）、`official_comment_rules`、`official_outbound_messages`（channel+kind+comment 唯一，硬保证一条评论一次私回）、`official_webhook_events`（不含正文的审计）；`todos` / `behaviors` 增加官方通道字段。
- **Official Channel 模块** `crates/onia-server/src/official/`：独立于 DeviceSession；Webhook HMAC-SHA256 验签 + hub challenge；messages/comments/live_comments/postback/reaction/seen/story mention 映射；IGSID/入口 id → 通道路由；评论规则匹配（关键词 + media_ids）；7 天窗口；分页审计。
- **链路**：Webhook → Signal（`instagram:{mid}` 与设备监听同一幂等键）→ Policy → Todo。普通 DM 在模式 A 下仍 `pull_thread` + 设备 `send_reply`；评论产出官方 `SEND_PRIVATE_REPLY` / `REPLY_COMMENT`，由服务端 `OutboundAdapter` 直接调 Graph，失败不回落私有接口、不自动重试。
- **控制台**：`/official` 页；通道 CRUD、token 轮换（不回显）、模式/入站/Private Reply 开关、评论规则编辑、最近 Webhook 审计。
- **环境**：`ONIA_META_APP_SECRET`、`ONIA_META_WEBHOOK_VERIFY_TOKEN`、`ONIA_OFFICIAL_TOKEN_KEY`（base64 32B）、`ONIA_GRAPH_BASE_URL`、`ONIA_OFFICIAL_TIMEOUT_MS`。

明确未做（与本文一致的部分）：模式 B 普通 DM 官方代发（接口存在但显式拒绝启用）；Official Conversation API 读历史；`INBOX_ACTIVITY` 收敛开关；Story/mention 产品化；Flow 可视化；多步跟进与转化看板。另：当前实现要求官方通道绑定一个已有设备账号，DM 仍靠设备补全会话；IGSID ↔ thread_fbid 映射仍是“对不上就人审”的降级路径。

## 15. 模式 B 与收敛补齐（2026-11-19 追加）

说明：下列现码已预留模式 B 显式开关与部分收敛能力，**仍属二期**，默认关闭，**不改变** §8 已拍板的一期边界。

在 §14 的一期之上补齐：

- **Mode B 普通 DM 官方代发**：`outbound_mode=official` 从“拒绝启用”改为显式可用。Webhook 入站 DM 会写入合成一对一会话 `igsid:<peer>`，服务端 Agent 生成草稿；Policy 在 channel mode B 且 `official_reply_peer_igsid` 已知时产出 `SEND_OFFICIAL_REPLY`，由 Official OutboundAdapter 调 `/{page_id or ig_user_id}/messages`。无 peer IGSID 时只记录事实，**不静默回落设备**。
- **发送语义**：`official_outbound_messages` 对 DM 以 `inbound_message_id` 为唯一键（同一入站消息一次回复）；私回仍按 `comment_id` 唯一。官方发送失败不自动重试，草稿在成功后标记 `sent`。
- **INBOX_ACTIVITY 收敛**：通道新增 `inbox_activity_fallback_enabled`，新建默认 false；Policy 在官方通道 active 且未开兜底时不再 `INBOX_ACTIVITY → pull_inbox`。需要补漏时可显式打开。
- **草稿审核**：`igsid:` 会话的草稿审批使用通道绑定设备做身份校验，但跳过实时 account binding 检查，设备离线也可审批/发送。
- **控制台**：Mode A/B 选择、INBOX 兜底开关、实验性官方会话历史读取、模式列、Webhook + 官方发送审计。
- **窗口**：Private Reply 服务端二次校验 7 天；Mode B 普通 DM 发送前校验最近入站时间不超过 24 小时，过期直接失败并写 Todo 错误。
- **仍未做**：Flow 可视化、Handover Protocol、Story/mention 完整产品化、转化看板。Conversation 历史读取已提供实验开关（`conversation_read_enabled`），失败时退回 Webhook 单条消息。

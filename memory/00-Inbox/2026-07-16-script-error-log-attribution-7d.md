---
title: Script 错误日志归因（7 天 41720 条）
date: 2026-07-16
tags:
  - instagram
  - script-error
  - signal-processor
  - app-ins-scripts
  - attribution
  - noise-reduction
---

# Script 错误日志归因（7 天 41720 条）

## 总量分布

- 41720 条 / 7 天 / 6 个 action
- 时间跨度：2026-07-10 00:00 → 07-16 11:22

| Action | 总数 | 占比 |
|---|---:|---:|
| getProfilePostImage | 21876 | 52% |
| getChatMessages | 15049 | 36% |
| followUserById | 4347 | 10% |
| sendMessage | 275 | 0.7% |
| clickMute | 96 | 0.2% |
| navigateToInbox | 77 | 0.2% |

## 一句话结论

**~73% 的"错误"其实是伪错误 + 脚本没做空态早退。真正的技术问题不到 5000 条。**

---

## 归因（按噪声大小排序）

### 1. 空 profile 判定慢 + 误报错误（17413 条，42%，最大噪声源）

- **哪儿**：`getProfilePostImage` / `ELEMENT_NOT_FOUND` / 消息："页面未发现帖子链接，可能为空 profile 或页面未加载完成"
- **selector**：`a[href*="/p/"], a[href*="/reel/"]`
- **要命的一点**：平均耗时 32.7s，p50 30.9s，p90 36.5s — 每次都跑满 timeout 才失败，脚本一直死等
- **归因**：空 profile（IG 用户没帖子）是正常业务状态，不是错误；当前脚本既没提前判定，也上报为 `ELEMENT_NOT_FOUND`
- **解决**：
  - script 层：先检测 profile 页 header 显示的 "posts" 计数，0 即返回 `{ok: true, images: []}`，秒返回；或者检测到 "No posts yet" / "暂无帖子" 语句直接早退
  - SP 侧：即使脚本仍报 `ELEMENT_NOT_FOUND`，`FetchProfileImagesDef` 收到空图列表算 completed，不当失败重试

### 2. NO_UNREAD 当错误上报（12787 条，31%）

- **哪儿**：`getChatMessages` / `NO_UNREAD` / "收件箱没有未读消息"
- **归因**：收件箱空是正常轮询结果，不该进 error 表
- **解决**：script 层 `NO_UNREAD` 走 success + `{unread: []}`；SP 侧过滤 `error_code IN ('NO_UNREAD','NEW_MESSAGE_DETECTED')` 不入 `script_error_log`

### 3. userIgHandle 数据质量事故（2941 条，7%）

`PROFILE_NOT_FOUND` 6546 条里挖出：

| Handle 形态 | 条数 | 样例 |
|---|---:|---|
| ✅ 干净 handle 但 IG 侧不存在 | 3440 | `ftbskee36` — 正常业务（用户注销/改名） |
| 🚨 handle 含空格/URL 编码 | 1949 | `4147211398%202201%20s%20allis%20st%20Milwaukee%20Wisconsin`（把地址塞进 handle）|
| 🚨 邮箱当 handle | 966 | `patrickwhartenby@gmail.com` |
| 🚨 URL 当 handle | 176 | `https:`（把整段 URL 截断成 handle）|
| 🚨 URL 编码 UTF-8 | 15 | `Jos%C3%A9mendozamtz` = `Josémendozamtz` |

- **归因**：上游（CE Kafka / admin 后台入口）让脏 handle 混进 `userIgHandle` 字段。`normalizeIgHandle` 只 strip 前导 `@`，不校验其他垃圾数据
- **解决**：
  - `ig-handle.util.ts::normalizeIgHandle` 增加白名单校验：`/^[a-zA-Z0-9._]{1,30}$/`，不匹配 → SP 侧 signal 层直接标 `signalError='invalid_handle'`，不产生 todo，不下发到设备
  - `CeKafkaAdapter.onCEEvent` 校验失败告警，回溯上游为什么会有邮箱/地址进 `userIgHandle`

### 4. Handle 遗漏 normalize（2000+ 条 followUserById 命中）

- **证据**：`followUserById` / `ELEMENT_NOT_FOUND` 消息："在 主页上找不到关注/已关注按钮"、"在 **@**agustinfostersr117@gmail.com 主页上找不到关注/已关注按钮"
- **归因**：handle 带前导 `@` 的仍然穿透到 APK，说明不是所有入口都跑了 `normalizeIgHandle`（现有 util 已经处理 `@`，但这里显然没生效）。同时上面 handle=邮箱的问题也叠加
- **解决**：审计所有会构造 `userIgHandle` payload 的位置（`FollowBackDef` / `ReplyDmDef` / `SendDmDef` / `FetchProfileImagesDef`），统一在 `makeStep` 输出前跑一次 normalize + 校验

### 5. INVALID_PARAMS: SP 侧 payload 缺 handle（579 条）

- **哪儿**：`followUserById` / `INVALID_PARAMS` / "请提供 userIgHandle 参数"
- **归因**：SP 层往 APK 发指令时 `userIgHandle` 为空字符串。可能是：
  - CE 事件里 `userIgHandle` 缺失 → 应该在 signal 层就拦掉，不该走到 behavior
  - 或 payload 展开时没读到正确字段
- **解决**：`FollowBackDef.fromSignal` 里 `if (!userIgHandle) return null;` 返回 null 使得 `todoCount=0`，不进 todo 层；并加告警

### 6. SESSION_LOST 集中在 5 个 creator（1227 条，3%）

- **哪儿**：`SESSION_LOST` / "登录态已失效" 或 "页面为空白"
- **分布高度集中**：

| Creator | 条数 |
|---|---:|
| 7860038 | 499（占 40%）|
| 7119364 | 119 |
| 6691148 | 92 |
| 7837083 | 60 |
| 7240892 | 44 |

- **归因**：少数 creator 设备 IG 号掉线 / cookie 过期；SP 侧仍在按调度器不断投递 signal → 每次都撞墙
- **解决**：
  - Signal → Todo 层加熔断：统计 `v3:session-lost:count:{creatorId}` 15min 内 ≥5 次，置 `v3:creator:disabled:{creatorId}` 1h，`InboxPollService.tick` 跳过 disabled creator
  - 触发运维告警，推设备重登

### 7. ACCOUNT_BANNED（136 条，主要 2 个号）

| Creator | 条数 |
|---|---:|
| 5613093 | 86 |
| 6882768 | 29 |

- **归因**：账号已封。继续投递 = 白白撞墙
- **解决**：
  - `AccountBannedRule` — 收到 `ACCOUNT_BANNED` feedback signal 立即 `IG_ACCOUNT_RESTRICTED` signal（SignalType 已存在），将 creator 从 `v3:inbox:poll-schedule` 直接 `zrem`；取消该 creator 全部 PENDING todo（status → `CANCELLED`）
  - 告警运营侧下发新号

### 8. clickMute 中文 selector 硬编码（27 条，小但危险）

- **selector**：`svg[aria-label*="对话信息"]`、`input[role="switch"][aria-label*="关闭通知"]`
- **归因**：APK 里其他脚本（`getChatMessages` 等）已经支持 zh/en/es（见本次 PR #628 的 `499b5300d feat(app-ins-scripts): support zh/en/es text matching in snapchat scripts`），`clickMute` 漏了
- **解决**：`clickMute.js` selector 改多语言：`aria-label*="Conversation information" i` OR `aria-label*="对话信息"` OR `aria-label*="Información de la conversación"` 三选一

### 9. EXCEPTION: undefined.length（177 条）

- `Cannot read properties of undefined (reading 'length')` × 177
- `Cannot read properties of undefined (reading 'alreadyFollowing')` × 5
- **归因**：脚本代码某处对可能为 undefined 的对象直接读 `.length` / `.alreadyFollowing`
- **解决**：grep 脚本源码里 `.length` 前置对象没做空判定的位置；根据 URL 分布（`/direct/inbox/` 和 `/direct/t/*`），优先看 `getChatMessages` 相关代码

### 10. CLICK_FAILED 无法进入聊天（1243 条）

- **哪儿**：全在 `getChatMessages`，消息："无法进入聊天: <displayName>"
- **归因**：inbox 列表能看到会话但点不进去。可能是列表 DOM 变化 / 会话已被删 / 页面卡顿。样本里 `displayName = "Instagram User"`（默认名），说明是已删除或匿名会话
- **解决**：script 层遇到 `displayName == "Instagram User"` 直接跳过下一条；正常会话点击失败重试 1 次后返回 `success=true, skipped=1`，不当错误

### 11. navigateToInbox / UNKNOWN 消息为空（77 条）

- **归因**：错误码 `UNKNOWN` 且 message 空 → 日志基础设施本身有 bug，错误路径没写入信息
- **解决**：在 SP 或脚本上报入口：拒绝 `error_code=UNKNOWN && !message`，直接分类 `INTERNAL_ERROR`

---

## 优先级 & 收益预估

| 优先级 | 动作 | 预计砍掉的噪声 |
|---|---|---:|
| P0 | 空 profile 早退 + 不当错误（§1） | -17413（-42%） |
| P0 | NO_UNREAD/NEW_MESSAGE_DETECTED 不入错误表（§2） | -12988（-31%） |
| P1 | Handle 白名单校验 + normalize 全入口（§3+§4+§5） | ~-3500 |
| P1 | SESSION_LOST/ACCOUNT_BANNED 熔断（§6+§7） | ~-1360 |
| P2 | clickMute 多语言（§8）、EXCEPTION 修 bug（§9）、CLICK_FAILED 匿名会话跳过（§10） | ~-1450 |
| P3 | 日志基础设施补 message 字段（§11） | ~-77（纯质量） |

**P0 两项做完 → 4.2 万降到 ~1.1 万，信噪比翻 4 倍，更容易看到真正的技术回归。**

---

## 落地建议

1. **P0 两项属于 script 层单文件改动，可以立刻 PR 单独出**：
   - `packages/app-ins-scripts/src/instagram/methods/getProfilePostImage.js` — 加空 profile 早退
   - `packages/app-ins-scripts/src/instagram/methods/getChatMessages.js` — `NO_UNREAD` 走 success
   - 或者在 SP `script-error-log-api.md` 定义的入库入口过滤 `NO_UNREAD` / `NEW_MESSAGE_DETECTED` / 空 profile 三个 code（改动更集中）
2. **P1 handle 治理是跨层改动**，建议先做 `normalizeIgHandle` 加白名单校验，再在 `CeKafkaAdapter` 落 metric 定位上游污染源
3. **P1 熔断**改 `InboxPollService.tick()` 和加个 `SessionHealthRule`，是本 sprint 内可以做的

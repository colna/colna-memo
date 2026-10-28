---
title: CE 交换后未自动关注 / 未自动回复（IG handle 被存成邮箱）
date: 2026-10-28
tags: [troubleshooting, sitin-next, social-proxy, ce, instagram, follow-back, dm-listener]
---

# CE 交换后未自动关注 / 未自动回复

## 现象

- 环境：**dev**（`dev-presence-service-log / dora-dev-stdout`）
- 男：`2100071159`（Liam，IG 实际 handle `lilybrown4783`）
- 女：`2100069412`（Eva，`leosmith0909`）
- CE 订单 `60354` 已完成（`status=COMPLETED`）
- 问题：女方没有自动关注男方；男方在 IG 私信女方后没有自动回复

## 结论

**根因在男方名片数据：`user_contact_card` 里 `account_id` 被存成了邮箱 `lilybrown21000@gmail.com`，不是 IG handle `lilybrown4783`。** SP 用这张名片解析 handle，follow 脚本拿邮箱去 IG 找人，返回 `USER_NOT_FOUND`，重试 4 代后彻底失败。

follow 失败又触发 `[Todo:cancel-sleeping] cancelled=2`，把同一个 CE signal 下的 **SEND_DM（女方主动打招呼）** 和 MUTE_USER 一起取消，所以女方连第一条 DM 也没发。

自动回复则完全没有触发：**dev 环境 7 天内 0 条 `SCRIPT_EVENT`**（prod 同期正常有 `NEW_MESSAGE`/`IG_DM_RECEIVED`）。服务端从未收到女方的私信事件，就没有 `IG_DM_RECEIVED` signal / `REPLY_DM` todo。并且 follow 失败意味着男方未被女方关注，他的 DM 很可能落在 IG 的 **Requests** 里，而 DM listener 只扫 inbox/底栏，不扫 requests。

## 证据链（dev，UTC / 北京时间）

| UTC | 北京 | 日志 |
|---|---|---|
| 07:55:39 | 15:55:39 | CE 订单 60354 创建（`orderCreateUserId=2100069412`） |
| 07:56:06 | 15:56:06 | `saveContactCard resp: cardId=4547, cardType=CARD_TYPE_IG, accountId=lilybrown21000@gmail.com` |
| 07:56:07 | 15:56:07 | `[CE] creator=2100069412 user=2100071159 platform=IG` → signal `01M60R4R08C18JD12DSQAP9P7B` |
| 07:56:07 | 15:56:07 | `[Identity] DB fallback hit: userId=2100071159 platform=instagram → handle=lilybrown21000@gmail.com` |
| 07:56:07 | 15:56:07 | `[Signal→Todo] produced=3 todos=[FOLLOW_BACK, MUTE_USER, SEND_DM]`（后两个 `SLEEPING`） |
| 07:56:08 | 15:56:08 | `[APK] method=followUserById params={"userIgHandle":"lilybrown21000@gmail.com"}` |
| 07:57:20 | 15:57:20 | `[Step:fail] USER_NOT_FOUND 未找到用户: lilybrown21000@gmail.com` |
| 07:58:22 | 15:58:22 | 第一代 `[Todo:fail] attempt=3` → `[Todo:cancel-sleeping] signalId=... cancelled=2` |
| 08:16:40 | 16:16:40 | 第 4 代 FOLLOW_BACK 最终失败（共约 20 分钟、16 次 follow 尝试） |

自动回复缺失：

- dev：`type=SCRIPT_EVENT` 近 7 天 **0 条**；`IG_DM_RECEIVED` / `REPLY_DM` 同期也为 0。
- prod：同期有正常 `SCRIPT_EVENT`（如 `type=NEW_MESSAGE`、`IG_DM_RECEIVED`）。
- 女方的 WebView 在 10-26 已更新过 `instagram/automation/startDMListener`，但 10-28 全天没有任何 `NEW_MESSAGE` 上报。

## 为什么会这样

1. **UE 输入无格式校验**：`EnterSocialUsernameModal` 只做 `trim + 去 @`，邮箱能直接提交。
2. **落库无校验**：`saveContactCard` 原样写入 `user_contact_card`，成为 `IdentityService` 的权威 handle 来源。
3. **失败级联**：`FOLLOW_BACK` 失败后 `Todo:cancel-sleeping` 把同 signal 的 `SEND_DM`/`MUTE_USER` 取消。
4. **DM 监听范围**：`startDMListener` 只监听 `/direct/inbox/` 或底栏红点；Requests 不在扫描范围。follow 失败 → 男方 DM 进 Requests → 无事件。
5. **附加数据问题**：多个测试男号共用 `lilybrown4783`，`resolveUserIdByHandle` 是 `findFirst orderBy createdAt desc`，handle → userId 存在歧义（当前 `lilybrown4783 → 2100071147`，不是 `2100071159`）。

## 修复建议

### 数据修复（先恢复这对）

- 更新 `user_contact_card`：card `4547`（userId `2100071159`, cardType `IG`）的 `account_id` 改为 `lilybrown4783`（确认该账号真实 handle 后）。
- 清 Redis：`v3:user:2100071159` 的 `instagram_id` 字段（或等 7 天 TTL），否则 SP 仍会读到旧邮箱。
- 手动触发一次 follow（或重新产生 CE signal；注意 dedupeKey `CE_EXCHANGED:2100069412:2100071159` 会挡住重复 signal）。

### 代码修复

- `EnterSocialUsernameModal.tsx` / `normalizeIgHandle`：拒绝邮箱、空格、`@ + / ?` 等非法字符；限制 1–30 位 `[A-Za-z0-9._]`。
- `saveContactCard`（user-service / CE 侧）也做一次服务端校验，避免绕过前端。
- `TodoSchedulerService`：`FOLLOW_BACK` 失败时不要取消 `SEND_DM`，或把 SEND_DM 改成独立可重试任务。
- DM listener：补扫 `/direct/requests/`（或 CE 交换后主动 OPEN_CHAT），否则未关注/单向关注场景永远不回。
- `IdentityService.resolveUserIdByHandle`：handle 应保证平台内唯一，或结合 chatId/CE 关系消歧。

## 复现/排查命令

```bash
# dev SLS
SK=/Users/colna/WORK/social-proxy-agent/skills/sls-logs/scripts/sls_query.py
source <(python3 -c '...从 ~/.aliyun/config.json 导出 STS...')
SLS_PROJECT=dev-presence-service-log SLS_LOGSTORE=dora-dev-stdout /usr/bin/python3 $SK \
  --query '"2100071159" and "CE"' --minutes 1440 --reverse

# 跟 follow todo
--query '"01M60R4R0BAXYDTTMHJH0CKAZD"'

# 看 DM listener 是否上报
--query '"type=SCRIPT_EVENT"'
```

## 关联文件

- `packages/app-pwa/src/components/EnterSocialUsernameModal.tsx`
- `packages/app-social-proxy-server/src/v3/ig-handle.util.ts`
- `packages/app-social-proxy-server/src/v3/identity.ts`
- `packages/app-social-proxy-server/src/v3/signal/adapters/ce-kafka.adapter.ts`
- `packages/app-social-proxy-server/src/v3/todo/todo.service.ts`（cancelSleepingSiblings）
- `packages/app-ins-scripts/src/instagram/actions/startDMListener.js`
- `packages/app-ins-scripts/src/instagram/actions/scanInboxThreads.js`

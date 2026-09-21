---
title: 登出重登后资料字段变回「add yours」——先查两个读模型的响应，再决定改哪边
date: 2026-09-21
tags: [troubleshooting, luka, sitin-rn, profile, proto, stale-read-model]
---

# 登出重登后资料字段变回「add yours」

来源：飞书《IOS Luka Buglist》T0052（2026-09-21，P1）。现象：退出登录再次登录后，
Occupation / Education / Interests / Bio 全变回 `add yours`，而**昵称、年龄、身高、
地点还在**。

## 先看事实：客户端的两条读路径

luka 的 Edit profile hub 所有行值来自 `hooks/use-me.ts` 的 `profile`，它有两个来源：

| 来源 | 覆盖字段 | 代码 |
| --- | --- | --- |
| `GetUserBasicInfosV2` 的 self 行 | 全部（nickname / age / height / profession / education / interestTabs / bio / …） | `services/me.ts` `getMyProfile()` |
| 本地会话快照（`UserInfo`） | **只有** nickname / age / geoLocation（+ avatar） | `hooks/use-me.ts` `profileFromSnapshot()` |

登出会 `clearSession()` + `clearMeOverview()`，快照与 store 缓存都没了 —— 所以
**跨登出只剩后端读模型**能证明这些字段存在。Iris 那套「编辑后本地缓存 + 读模型缺字段
时保留旧值」（`141e269c1`）救不了这条复现，因为缓存本身被登出清掉了。

截图里 Height、Location 仍在 → 写入是通的（或 onboarding 阶段写的），丢的只是
`profession / education / interestTabs / bio` 这四个。

## 排查手法：只记字段存在性（PII-free）

`apps/luka/src/services/me.ts` 在两个读路径上各加一行日志（仅 `isDevToolsAvailable()`
时写，落在 Export Log 里）：

- `[profile] self nickname:y age:y height:y profession:n education:n interestTabs:n bio:n geoLocation:y`
- `[profile] page code=0 profession:n interestTabs:n bio:n …`（`GetUserProfilePageInfo`）

```ts
function fieldPresence(user: UserApi.UserInfo): string {
  const absent = (v: unknown) => v === undefined || v === null || v === "" || v === 0 || (Array.isArray(v) && v.length === 0);
  const fields = ["nickname", "age", "height", "profession", "education", "interestTabs", "bio", "geoLocation"] as const;
  return fields.map((f) => `${f}:${absent(user[f]) ? "n" : "y"}`).join(" ");
}
```

**为什么不用现成的 `[接口] resp …`**：`packages/rn-net` 默认 `payloadChars = 400`，
self 的 `UserInfo` 很长，排在后半段的 `profession / education / bio` 会被截断 —— 看到的
「空」可能只是没打印。要么临时调 `payloadChars`，要么用上面这条定点日志。

## 结论分支（拿到日志后）

| 现象 | 结论 | 修法 |
| --- | --- | --- |
| `self` 行四个字段都是 `n`，`page` 行有值 | 主读模型缺字段，资料页读模型有 | 客户端读 `GetUserProfilePageInfo` 合并（注意：**它没有 education / height**；`code=0` 是 `UserServiceCommonCodeNone`，不代表失败，别按 `=== Success` 丢） |
| 两个模型都 `n` | 后端没持久化 `EditProfile` 的这些字段（或其投影不返回） | 后端修；客户端缓存救不了登出重登 |
| `self` 行有值、UI 仍空 | 客户端 mapper / 合并丢值 | 查 `use-me` 的 merge 与 hub 取值 |

## 可复用点

- **跨登出的字段丢失，本地缓存类修复一律无效** —— 先确认哪些数据源活过了登出。
- 日志截断会让「空字段」和「没打印」长得一模一样；定位时优先加**定点**的字段存在性日志。
- `code=0`（`UserServiceCommonCodeNone`）在 Luka 后端是常态，客户端若按 `code === 1`
  判定成功，会把有数据的响应整条丢掉（`GetUserProfilePageInfo` 就踩过）。

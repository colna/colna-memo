---
title: sitin-pwa Social Connect 一次性任务卡不显示(cardType 归类)
date: 2026-08-17
tags: [troubleshooting, sitin-next, app-pwa, ce, social-connect]
---

# Social Connect 一次性任务卡「未登录+有订单却不显示」

## 现象
首页 Task 栏「Social Connect on Instagram / Snapchat」卡(`OnboardingTaskList` 里的
`SocialConnectCard`,无 taskId),在「对应社媒未登录且有积压 CE 交换订单」时不显示,
尤其 Snapchat。

## 卡的显隐逻辑
- 门槛:`pendingByPlatform[platform] > 0 && loggedInByPlatform[platform] !== true`。
- `amount = pendingByPlatform[p] = Σ pwaFollowReward`,由 `checkPendingOrders` 每次拉单后写。
- 「金额 0 不展示」是产品确认的规则 → 保留 `amount>0` 门槛,不改成 count-based。

## 根因
`checkPendingOrders` 汇总金额时用 `cardTypeToPlatform(item.order?.cardType)` 归类,而
```js
cardTypeToPlatform = (ct) => ct === CardType.SNAPCHAT ? "snapchat" : "instagram";
```
对**缺省/未知 cardType 默认返回 instagram**。→ 无 cardType 的订单钱全记到 IG,
SC 永远累计不到 → SC 卡不显示,且 IG 卡显示了 SC 的钱。

## 修法(commit 0cc813786)
`checkPendingOrders` 严格判:`ct===2→snapchat / ct===1→instagram / 其它 continue 跳过不猜`,
不再用 `cardTypeToPlatform` 的默认兜底。门槛 `amount>0` 不变。

## 依赖后端(关键前提)
SC 卡真正显示,前提是后端在 `ListUserInsExchangeOrderResponse.orders` 里下发 `card_type`
(gen 已含字段 13,proto 源一度没有)。列表接口若没下发,SC 订单会被跳过 → SC 卡仍不显示,
需后端补 `card_type`。真机 vConsole 看 `checkPendingOrders` 的
`console.log("333333接受ins交换", ...)` 里 `response.orders[].order.cardType` 是否有值即可验证。

## 通用经验
`cardTypeToPlatform(undefined)` 默认 instagram 是个隐雷:凡是「按 cardType 分平台汇总/判断」
的地方,缺省值都要显式处理(跳过或报错),不能让它默默落到 IG。

---
title: 订阅成功后赠送金币为 0 —— 客户端没有「赠币数」这个字段，先对账再改
date: 2026-09-21
tags: [troubleshooting, luka, sitin-rn, subscription, iap, coins]
---

# 订阅赠币为 0

来源：飞书《IOS Luka Buglist》T0061（2026-09-21，P0）。截图：Sparks history 里
`Subscription coins +0`，而同屏签到是 `+3` / `+2`。

## 关键事实：金额完全来自后端

流水的金额与文案是分开的：`packages/business-payments/src/payments.ts` 的
`GetUserCoinChangeHistory` → `mapCoinHistory` 里 `amount = |coinValue|` 取自响应，
label（`USER_SUB` → "Subscription coins"）才是客户端本地映射。

**客户端在任何一步都没有「赠币数量」可以传**：DTC 建单只有 `productId/provider/bizType`；
iOS 验单 `VerifyIOS` 只有 `transactionId + subscriptionId`（周期码 100/101/102）。

## 客户端唯一可能指错的东西：周期码

`VerifyIOS` 的 `subscriptionId` 此前由商品 ID 后缀猜（`luka_vip_*`），猜错的话后端可能
对到一个「没有赠币配置」的计划。已在 `packages/business-payments` 0.7.2 修掉：周期优先取
目录的 `name`，并记住售卖时的周期供验单使用（见该包 CHANGELOG）。

## 界面侧的坑：宣传与配置脱钩

luka paywall 的权益格原先把赠币写死成 `750/1,500/3,500 Coins instantly added`，
而实际赠币取决于 Matrix 里商品的 `subCoins`。于是「界面照喊、后端记 0」看起来像客户端
bug。已改成读 `GetSubscriptionsV3` 的 `subCoins`（映射成 `Plan.credits`），一个档都没配
就不显示那一行。

## 后端对账清单（客户端已确认无能为力的部分）

1. `GetSubscriptionsV3` 返回的 `normalSubscriptions[].subCoins` 是否为非 0（三个档
   750/1500/3500 对应的值）。
2. Matrix 后台：`luka_vip_*` 商品的赠币是否配置；`docs/external-platform-onboarding.md`
   说权益最后一条必须是 `%d` 赠币条目（官方标注的坑）。
3. 真实买一单后查 `GetUserPaymentOrderDetail`（proto id 5030）的
   `status / coinExpected / coinDelivered / coinBase / coinBonus`：
   - `status=ACTIVE` 才算订阅生效；DTC 单的赠币走 `activateThirdPartySubscription`
     另一条履约路径。
4. `GetUserCoinChangeHistory` 那条 `USER_SUB` 的 `coinValue` 是 0 还是缺字段；对比
   `changeTime` 是否等于购买时刻（自动 restore 会重复验单，看后端是否幂等）。
5. 用 dev 的 fake 通路对照：`FakeVerifyIOS` 也不发币 → 指向后端配置；只有真实验单不发
   → 指向 `VerifyIOS` 的字段/计划匹配。

---
title: RN 订阅页「没有任何套餐」——不是客户端 bug，是后端商品目录为空
date: 2026-09-21
tags: [troubleshooting, react-native, payments, subscription, sitin-rn, luka, koda]
---

# 症状

付费墙只剩 `CHOOSE YOUR PASS` + 一个**点不动的 CTA**，**既没有转圈、也没有
`Couldn't load plans`**。用户口径通常是「订阅页未获取到任何套餐，无法订阅」，看起来像接口挂了。

出处：sitin-rn 各派生包（luka 2026-09-21；koda R00218/R00219 2026-09-08 同一病灶）。

# 为什么界面看不出病因

`packages/business-payments/src/subscriptions.ts` 的 `getSubscriptions()`：

```ts
const res = await client.call(UserApi.GetSubscriptionsV3Request, {}, UserApi.GetSubscriptionsV3Response);
if (res.code !== UserApi.UserServiceCommonCode.Success) return [];
return (res.normalSubscriptions ?? []).map(mapSubscription);
```

- **非 Success 业务码被吞成 `[]`**，不抛错；`useAsyncData` 的 `error` 只在网络层真抛异常时才置真。
- 于是「后端报错」「后端没东西卖」在屏上完全一样：`plans=[]` 且 `error=false` →
  `activeId=""` → `useSubscriptionPurchase.start()` 第一行 `if (!activeId || !activePlan) return;`
  → CTA 点了没反应。
- 同一模式也在 `packages/business-vibes`（like / 访客列表）。

# 怎么坐实（取真实证据）

1. 拿到设备上的 `session.log`（Debug Tools → Export Log；路径
   `<App container>/Library/Caches/logs/session.log`，`@heyhru/rn-log/file` 写）。
2. grep 原始 RPC：`grep "GetSubscriptionsV3" session.log`。

luka 本次的实锤：

```
req  GetSubscriptionsV3Request  POST https://api-dev.lukasoc.com protoId=4538 {}
resp GetSubscriptionsV3Request  POST https://api-dev.lukasoc.com protoId=4538
     code=1 {"code":1,"message":"success","normalSubscriptions":[],"visitorSubscriptions":[]}
```

`code=1`（`UserServiceCommonCode.Success`）+ 两个数组都空 → **客户端原样收到了空目录**，
不是解析/映射问题。

3. 同时对照同租户的**其它商业化接口**，证明身份与鉴权没问题：
   - `GetCoinProductInfos` 返回真实币档位（`coins_tier8..11`）；
   - `GetSupportedCardTypes` 返回 `cardTypes:[1,2,3]`；
   - `SwipeCard` / `GetUserBasicInfosV2` 都 `code=1` 且有数据。
   即：同一个 `app_name` / `package_name` / token 下**只有订阅商品为空**。

# 结论

- **客户端没有可修的地方**：它已经正确地把后端目录渲染出来了。
- 需要后端在**该 app 的租户**下配置会员 / 访客订阅商品（`normalSubscriptions` /
  `visitorSubscriptions`），dev 与 prod 两套后端各配各的。luka 的 dev 后端只有币档位、
  没有订阅商品。
- 客户端商品 ID 的归属由 `app.config.ts` 的 `extra.iapSkus`
  （luka：`luka_membership` / `luka_visitor` / `luka_credits`）与 `services/config.ts` 的
  `iapKindForSku()` 决定；后端目录里的 `iosProductId` / `androidProductId` 必须包含对应关键字，
  否则即使有目录也会被判成 `unknown` 而拒绝。

# 可选的客户端改进（未做）

区分「目录为空」与「加载失败」：非 Success 时抛错（或在 `Plan[]` 之外带回 `code`/`message`），
让 UI 落到 error/empty 分支而不是一个灰 CTA。这是共享包行为，动它会影响所有消费方，
与后端补商品是两件事，不要混在一个 PR 里。

# 后续（2026-09-21 下午）：后端补了目录，但商品 ID 与客户端关键字不一致

后端补完后，直接对 `api-dev.lukasoc.com` 做带鉴权调用核实（`FastLogin` 拿 token →
`GetSubscriptionsV3`），目录已非空：

- 会员：`luka_vip_yearly` / `luka_vip_monthly` / `luka_vip_yweekly`
- 访客：`luka_guest_yearly` / `luka_guest_monthly` / `luka_guest_weekly`
- 币：`luka_bronze_1..7`

**新问题**：这些 ID 与包内配置/解析全对不上 ——

| 位置 | 期望 | 后端实际 |
| --- | --- | --- |
| `apps/luka/app.config.ts` `iapSkus.membershipKeyword` | `luka_membership` | `luka_vip_*` |
| `iapSkus.visitorKeyword` | `luka_visitor` | `luka_guest_*` |
| `iapSkus.coinsKeyword` | `luka_credits` | `luka_bronze_*` |
| `periodFromProductId` | 后缀 `_w`/`_week`/`_y`/`_year` | 后缀 `weekly`/`monthly`/`yearly` |

后果（都是静默的）：

- `iapKindForSku()` 对全部 SKU 返回 `unknown` → `iap-verifier.ts:54` 直接 `return false`，
  **真实 IAP 永远不发货**；`use-subscription-purchase.ts:138` / `use-coin-purchase.ts:76`
  的购买结果监听也会把它过滤掉。
- `periodFromProductId()` 对 `yearly`/`weekly` 都落到默认 `monthly` → 套餐卡标题全部显示
  `1 month`，DTC/埋点的 `billingPeriod` 也错。
- `entitlements.ts` 的 `visitorActive()` 用 `productId.includes(visitorSkuKeyword)`，访客订阅
  `luka_guest_*` 不包含 `luka_visitor` → **买了也读成未解锁**。

另注意 `luka_vip_yweekly` 疑为 `weekly` 的拼写错误（`yearly` / `monthly` 都是正常词），
要先跟后端确认，再决定解析器怎么写。

**修法（已采用 ②，2026-09-21）**：App Store 商品 ID 一旦创建不可改，所以是客户端对齐后端：

- `apps/luka/app.config.ts` `iapSkus` → `luka_vip` / `luka_guest` / `luka_bronze`。
- `packages/business-payments/src/subscriptions.ts` 的 `periodFromProductId` 加认无分隔符的
  word 后缀 `weekly$` / `yearly$`（保留 `_week`/`_w`/`_year`/`_y`），因为 `luka_vip_yweekly`
  的 `y` 不是分隔符。
- 测试：`packages/business-payments/tests/subscriptions.test.ts` 补真实 ID 用例。
- 验证：6 个真实 ID 的 period/label/kind 全对；luka typecheck 0、business-payments
  typecheck 0、biome 0、boundaries OK；luka 911 passed（4 红为既有 shape-language 等）。

教训：**共享包里的「商品 ID 解析」和 app 里的「关键字」是一对，任何一边改名字都要同步改另一边**，
且两边都不报错 —— 症状是静默的（不发货 / 不解锁 / 标签错）。后端换商品目录时优先确认这两处。

# 相关

- `docs/payments.md`《App 身份与发布门禁》——「每个 App 发布前必须拥有独立的 StoreKit / Play 商品」。
- koda 同病灶：`50-Daily/2026-09-08.md`（R00218/R00219）。

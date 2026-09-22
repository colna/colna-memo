---
title: Luka 资料页地址显示「0, California」——后端 UserGeoLocation 原文，不是展示问题
date: 2026-09-22
tags: [troubleshooting, luka, sitin-rn, location, backend-data, protobuf]
---

# 症状

部分用户从 feed 进资料页，地址胶囊显示 `0, California`（`apps/luka/src/app/profile/[id].tsx`
→ `ProfileSummaryCard` 的 location 事实）。

# 结论

**接口数据问题**：后端该账号的 `UserGeoLocation` 原文里城市位就是 `0`。客户端只是原文拼接。

展示链（无任何加工）：

- `apps/luka/src/lib/candidate-mapper.ts` `buildLocation()` =
  `[geo?.city, geo?.province || geo?.region].filter(Boolean).join(", ")`
  → 只有 wire 上是 `city="0"` + `province="California"`，或 `region="0, California"`，
  才可能渲染出 `0, California`。客户端没有任何把数字/占位符写进地址的路径。

# 实证（dev 后端扫码，2026-09-22）

FastLogin 新设备号 → `GetMatchFeedsV3` 扫 ~2000 个女用户 + 对 400 个用户逐个对比
`GetUserProfilePageInfo`：

- 三个接口（feed / `GetUserBasicInfosV2` / `GetUserProfilePageInfo`）返回的 `geoLocation`
  **400/400 完全一致** → 资料页地址不可能与卡片不同。
- 后端地址的存法是 **`region` 放整串 `"City, State"`**，`city` / `province` 常为空；
  客户端靠 `province || region` 兜底显示。
- 样本里没扫到 `0`，但同类脏值不少：`" Ohio"`（只有州、带前导空格）、
  `"South African"` 当州、`"Strand, Georgia"`（Strand 在南非）→ 这批账号地址是
  批量灌入/反解析失败留下的，`0` 是同一来源的城市位占位。

# 复现/取证手法（可复用）

无 token 时直连 dev 后端做只读核实（不依赖模拟器）：

1. `esbuild` 打包脚本，alias 指向共享包源码：
   `--alias:@heyhru/common-util-pb=./packages/common-util-pb/src/index.ts --alias:@heyhru/business-pwa-proto=./packages/business-pwa-proto/src/index.ts`
2. 直接用 `PbClient`（无 RN 依赖），`getParams` 带
   `{ app_name: "luka", package_name: "com.presence.gracechat", appver, os }`。
3. `FastLogin { deviceId: 随机串, platform: iOS }`（**skipAuth**，会 mint 一个新 dev 账号）拿 token。
4. 再调 `GetMatchFeedsV3` / `GetUserBasicInfosV2` / `GetUserProfilePageInfo` 打印 `geoLocation`。

# 后续

- 定位到具体账号需 uid：给我 uid 即可按上面方式查 dev 原始值（或从设备
  `session.log` grep `resp GetUserBasicInfosV2` 的 payload 预览）。
- 若要前端兜底（把纯数字/`0` 当空值过滤），`buildLocation` 在 iris / luka / koda / lumi
  各有一份，需一起改；属防御性展示，根因仍在写这条数据的一端（女端/PWA 或灌库脚本）。

---
title: sitin-rn 项目结构与架构速查
date: 2026-09-02
tags: [sitin-rn, react-native, expo, monorepo, koda]
---

# sitin-rn 结构速查

`/Users/max/Dev2/zhangzheng/sitin-rn`(main,remote `presence-io/sitin-rn`)。**Expo/React Native 多应用 monorepo**,pnpm workspace,源码消费共享包(无构建步骤)。RN 0.86 / React 19.2 / Expo SDK 57 / TS 6 / uniwind(Tailwind-for-RN)/ vitest / biome。

## 顶层
- `apps/*` — 可部署应用:**demo, iris, koda, lumi, naya**。`apps.json` 是应用清单单一真源(CI 读它)。
- `packages/*` — `@heyhru/*` 共享包,`business-*`(领域引擎)+ `rn-*`/`common-*`(基建)。
- `docs/` — 权威文档:`architecture.md`(分层/依赖/抽包)、`shared-packages.md`、`new-app.md`、`ci-pipeline.md`、`dependency-management.md`、`testing.md`、`payments.md` 等。
- `pnpm-workspace.yaml` — **catalog** 集中管版本(`"react":"catalog:"`),`overrides` 钉 react/react-dom 单实例。

## 架构三层(应用内,决定能不能抽包)
- **基建 Infra**(~100%可抽→packages):`services/client.ts`、native SDK 封装、传输、日志。
- **领域 Domain**(~70%,引擎可抽、屏幕留 app):chat/call/payments/discovery/match 的 service+store。
- **应用胶水 Glue**(0%,永远在 app):`src/app`(屏幕)、hooks、app 专属 components。
- 依赖规则(强制,`pnpm boundaries` 每个进 main 的 PR 卡):app→packages 可;app 不得依赖另一 app;packages 不得依赖 app;**带原生面的包不能被另一个包依赖**。
- 每 app 只自己写四样:屏幕交互 / 主题 token(`src/theme`)/ `app.config.ts`(域名 scheme SDK key 等身份值,构建期按 EAS profile 选)/ fixtures。**共享基建绝不 import 某 app 的 config,由 app 注入**。
- 起新 app 从 `apps/demo` 复制骨架,**不要复制 iris 再删**(会带出品牌串/存储键前缀/埋点名血缘)。

## Koda app(`apps/koda/`,张峥 buglist 全在这)
暖色浅色视觉,复用既有业务引擎+后端协议,视觉走 Figma Koda。**必须 Development Build**(Expo Go 报 SDK 不兼容)。
- `src/app/` — expo-router 路由。tabs:`(tabs)/index(首页)|chat|feed|vibes|me`。其余:`chat/[id]`、`settings/*`(index/blocklist/help/payment-account)、`edit-profile/*`、`contact/*`(CE:exchange/chemistry/spark)、`onboarding/*`、`profile/[id]`、`paywall`、`credits`、`legal`(privacy/terms)。
- `src/components/` — 按域分:auth chat contact discovery edit-profile feed match notifications onboarding payment profile settings tabs ui vibes …
- `src/stores/` — zustand:chat / discovery / session / entitlements / onboarding / notifications-unread / koda-exchange-ui …
- `src/services/` — 网络+领域:chat-*、call-*、discovery*、contact-exchange、settings、payments/iap、analytics …
- 常用脚本:`pnpm koda:start|ios|android|typecheck|test`;根级 `pnpm lint|typecheck|test|boundaries`。

## 常用命令
`pnpm install` → `pnpm <app>:start`(metro)→ `pnpm <app>:ios`。lint=biome。校验三件套 `pnpm lint && pnpm typecheck && pnpm test`。

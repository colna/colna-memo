---
title: PWA 分级卡片预览 mock(sitin-next app-pwa)
date: 2026-08-20
tags: [sitin-next, app-pwa, pwa-tier, grade, mock, preview, 权益池, 小美]
---

# PWA 分级机制(权益池 / 小美)卡片预览 mock

无后端 XIAOMEI/权益数据时,在浏览器预览 Me 页各分级卡片样式。

## 位置

- **mock 文件**:`packages/app-pwa/src/mocks/pwaTierMock.ts`
- **接线**:`packages/app-pwa/src/pages/Me/index.tsx` 进页 useEffect —— 带 `?gradeMock=<key>` 时动态 import mock 灌进 `pwaTierStore`,否则走真实 `getPwaTierMe`。
- **分支**:`feature/pwa-nationwide`(commit `bea2d5e99`);数据结构对齐接口 gen 类型(basis points / micros / 枚举)。

## 用法(URL 门控,动态 import 不进主 bundle)

| URL | 预览 |
|---|---|
| `/me?gradeMock=member` | 常驻权益卡 `GradeBenefitPanel`(Video +50% / Chat +100%) |
| `/me?gradeMock=xiaomei` | 小美 3K Club 卡 · 进行中(Go Live ✓ / Video Earn. 44% / Eligible 未达,药丸 In Progress 黄) |
| `/me?gradeMock=xiaomei-done` | 小美 3K Club 卡 · 已完成(三环全 ✓,药丸 Achieved 绿) |
| `/me?gradeMock=invitation` | 定向邀请入口卡 → 底部 Boost Sheet |
| `/me`(无参) | 真实后端 |

小美两态差异只在 `guaranteeProgress` 的 `postBenefitVideoIncomeMicros`(35→80)/ `eligibilityReached`(false→true)。

## ⚠️ 前置:必须重建 proto dist

预览前 `pnpm --filter @heyhru/business-pwa-proto build` —— 否则本地旧 dist 没有前端先行加的 `videoIncomeTargetMicros`,Video Earn. 环显 0%。(dist gitignore,拉 proto/gen 变更后本地都要重建,见 [[sitin-pwa-proto-surgical-gen]] 里的 dist 坑。)

## 注意

- mock 在共享 `feature/pwa-nationwide` 上(URL 门控 + 动态 import,线上无害);**上 `feature/pwa` 主线前是否删由产品/用户定**(权益池 mock 曾删过一次)。
- 接口口径见项目内 `sitin-next3/docs/pwa-tier-grade-frontend.md` 与飞书 RFC docx `HEPjdg9B4oPHWlxmRbQcfRHNndq`。

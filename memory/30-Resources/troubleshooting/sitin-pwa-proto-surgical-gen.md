---
title: sitin-next proto 增量加接口(surgical gen,不整包 bump)
date: 2026-08-17
tags: [troubleshooting, sitin-next, business-pwa-proto, proto]
---

# 给 business-pwa-proto 只补一个接口的 gen(不污染其它 gen)

## 背景
要给前端加某个新 protoId(如 21022 GetUnfinishedOrderAmount),但该接口只在
proto 的 `origin/release/test` 有、`online` 还没。直接 `pnpm proto:gen` 会出问题。

## 坑
`pnpm --filter @heyhru/business-pwa-proto proto:gen` 会从当前 submodule 基线**全量**重生成,
而仓库里 committed 的 `src/gen` 往往和当前 submodule checkout **不一致**(如 committed gen 有
`live_api.ts`,但近期 proto commit 没有 `live_api.proto`)→ 全量 regen 会**大范围乱改**:
删掉 live_api.ts、`protoIdMapping.json` 删掉 Live/PwaMatchFeeds/HoldingAutoForward 等一堆现有 mapping。
直接提交会破坏其它模块。

## 正确做法(surgical)
1. submodule 里只取目标 proto 文件:
   `cd packages/business-pwa-proto/proto && git checkout origin/release/test -- archat_api/xxx.proto archat_api/ProtoConfig_*.json`
2. `pnpm --filter @heyhru/business-pwa-proto proto:gen`
3. **只保留目标 gen 文件的 diff**,其余全还原:
   `for f in $(git diff --name-only .../src/gen | grep -v 'xxx.ts'); do git checkout HEAD -- "$f"; done`
4. `protoIdMapping.json` **手动**加目标两条(regen 版会删掉别的 mapping,不能用),
   在对应区段(如 21021 后)插入 `"XxxRequest": 21022, "XxxResponse": 21023,`。
5. submodule 还原、**不 bump 指针**:`git -C proto checkout <原commit> -- <那些文件>`。
6. `pnpm --filter @heyhru/business-pwa-proto build`(tsup + 拷 protoIdMapping 到 dist)。
7. 校验目标 gen:`grep GetUnfinishedOrderAmount dist/gen/.../xxx.d.ts`、`grep 21022 dist/gen/protoIdMapping.json`。

结果:只动 1 个 gen 文件 + protoIdMapping 2 行,dist 未入库(gitignore),submodule 指针不变。

## 类型坑
`utils/bridge.ts` 的 `CardType` 与 proto gen 的 `CardType` 是**两个 TS 类型**(值相同 1=IG/2=SC)。
`platformToCardType()` 返回 bridge 的,传给 proto 请求会报 TS2345。仿 `createBlurredCardOrder`:
API 封装参数用 `number`,内部 `cardType as CardType`(proto)。

## 全新克隆跑 tsc
app-pwa tsc 前需先 `pnpm build` 内部 workspace 包(common-util-format / web-util-media 等),
否则报 TS2307 找不到模块(与业务无关)。

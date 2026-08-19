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

---

## 追加(2026-08-19)整包更新到 release/test + 手写字段对齐

**场景**:要把某分支 proto **整体**更新到 `release/test`(不是只补一个接口),用 `proto:test`
(`cd proto && git checkout origin/release/test && generate.sh && build`)。

### 何时整包 regen 是安全的
- 先 `git checkout origin/release/test` 提 submodule,`generate.sh` 全量重生成,
  **然后 `app-pwa tsc -b` 必须 0 error**——若 release/test 删/改了 app 在用的字段,tsc 会炸,
  那就得连带改 app(范围变大)。今天 tsc 0 → 整包 regen 不连带 app 改动,可直接提交。
- 只提交**有真实变化**的 gen 文件。

### protoc 版本噪声(重要)
committed `src/gen` 头部有 `//   protoc  v6.32.0`;本机 brew 是 **v7.35.1**。全量 regen 会给
**每个** gen 文件那一行改成 v7.35.1(代码字节其余一致)→ 未变的 proto 也“被改”。
筛掉噪声:对每个改动 gen 文件,若 diff 去掉 `^[+-]//   protoc +v` 行后为空,就 `git checkout` 还原。
今天 23 个改动里 17 个是纯版本噪声、只留 6 个真变化。

### 手写字段“先行”会和后端对不上(踩过)
我先行手写 `ReplyReward`/`reward` 到 gen;后端 release/test 实际叫 **`BonusReward`/`bonusReward`**
(`baseCents/bonusCents/totalCents=int64`、`multiplier=float`,字段号 1/2/3/4)。
**教训**:动手手写字段前,先 `git show origin/release/test:archat_api/xxx.proto | grep -i <关键词>`
看后端有没有已经落地、叫什么。对齐时:分支 merge 带真 gen 的 nationwide,`messaging_api.ts` 冲突
`git checkout --theirs` 取真生成版;app 里 `ReplyReward→BonusReward`、`.reward→.bonusReward`
(内部 ref/函数名 `lastReplyRewardRef`/`consumeLastReplyReward` 可保留,语义仍准)。

### commitlint 拒自定义 merge 首行
merge commit 首行若写 `Merge feature/x + 说明…` 会被 commitlint 判 `type-empty` 拒。
必须用标准 `Merge branch 'feature/x' into <cur>`(commitlint 默认忽略此前缀),说明放正文。

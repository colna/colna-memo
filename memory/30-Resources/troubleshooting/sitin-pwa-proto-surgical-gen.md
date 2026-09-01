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

---

## 追加(2026-08-25)只加一个 Response 接口 + committed gen 含「手写占位字段」时的干净 regen

**场景**:nationwide 分支加 `getPwaTierInvitationPopup`(proto PR#453,只改 user_api.proto)。

### 先判断:整包同步能不能过 tsc
- 直接 `git checkout origin/release/test -- archat_api/user_api.proto` 整文件替换后 regen → `app-pwa tsc -b` **炸 7 处**:release/test 把 app 在用字段改/删了(`benefit_config_version`/`terms_version` 移除;`video_income_target_micros` 改名 `required_video_income_micros` + 加 service_quality_passed 等)。→ 整包不安全,退回 surgical。

### committed gen 有「前端先行手写占位字段」时的坑
- committed `user_api.ts` 的 `PwaTierGuaranteeProgress.videoIncomeTargetMicros`(注释「前端先行占位字段」)**不在任何单一 proto commit**。
- 从记录指针 `3884fef0` 直接 regen → 会 **-23 行**(丢占位字段),diff 变脏、且 app 用它会编译错。
- **手法(干净 diff)**:submodule 里 `git checkout 3884fef04 -- archat_api/user_api.proto` 为底,手补占位字段到对应 message(注释/字段号/类型要和 committed gen 完全一致,ts-proto 会把 `//` 注释转成 `/** */`),再追加目标 message,然后 regen。这样 `user_api.ts` 的 diff **只剩目标 message**(+ 偶发一处相邻字段换行的格式化噪声,无害)。
- 目标是 Response-only 接口:gen 出 `GetPwaTierInvitationPopupResponse`(interface + MessageFns)即可,自足(引用已存在的 PwaTierResponseMeta/GetPwaTierMeResponse)。

### Response-only 接口前端调不通的两个前置(后端侧)
- httpClient `requestPost2(ReqMsg, payload, ResMsg)` 靠 **Request message 的 `$type` → protoIdMapping** 推 protoId 和 url。
- 只有 Response、没有 `XxxRequest` message → 无法发起;且 `protoIdMapping.json` 由 `ProtoConfig_*.json` 的 `id` 生成,PR 只加 .proto message 没登记 ProtoConfig id → mapping 里没有该接口。两者都要后端补齐才能真正调。

### 环境坑(复用「shared checkout」)
- `sitin-next3` 被并发会话反复横跳分支。改 gen 前后锁定分支,未提交的 gen diff 先 `git diff > scratchpad/xxx.patch` 备份防横跳丢失。

---

## 追加(2026-09-01)整包对齐 release/online:何时是「删除型 regen」+ pre-push 被无关包拖累

**场景**:feature/pwa 把 proto 整体对齐 `release/online` 并 regen(PR#1077)。

### 现版工具链变化(好消息)
- `generate.sh` 头部已改用 `// @ts-nocheck`,**不再写 protoc 版本行** → 8/19 记的「protoc 版本噪声」坑消失,diff 干净很多。
- 包内已有现成脚本:`proto:online`(fetch+checkout origin/release/online+generate.sh+build)、`proto:test` 同理。想整包同步直接用,不必手敲。

### 判断「删除型 regen」是否安全的套路
- committed `src/gen` 常**领先** online(混入 live_api + test/nationwide 才有的接口)。整包 regen 到 online 的净效果往往是**删除**(本次删 live_api.ts 9749行 + Live*/GetPwaMatchFeeds/PwaSwipeCard/GetFaceVerifyConfig/GetClinkPrewarmSession/GetConversationChatMode 等一批 mapping)。
- 安全门槛两步:① 从 `git diff protoIdMapping.json` 抽出被删接口名,`grep -rl '\bXxxRequest\b\|\bXxxResponse\b' packages --include=*.ts(x)` 排除 proto 自身,确认**全仓库零引用**；② `app-pwa tsc -b` 必须 0 error(唯一消费 proto 的 app;app-cashier 不引用 proto)。两者都过 → 整包删除安全。
- 判断新增:`git diff protoIdMapping.json | grep '^+' ...` 为空 = online 相比 committed 无新增接口,本次纯删除+字段对齐。

### app 走 dist 不走 src(重要)
- `business-pwa-proto` package.json exports `./gen/*` → **./dist/gen/*.d.ts**。改 src/gen 后 app-pwa tsc 看不到,**必须先** `pnpm --filter @heyhru/business-pwa-proto build` 更新 dist；再 build app-pwa 依赖(web-util-http/common-util-format/web-util-media/common-util-event-bus/common-util-timezone,见 app-pwa `vercel:build` filter)才能 `pnpm --filter @heyhru/app-pwa exec tsc -b`。

### pre-push 会被无关包全量拖累
- `.husky/pre-push`=`pnpm test --affected && pnpm build --affected`。改 proto 让一堆下游 minerva 包变 affected,它们因刚 pull 大量 commit **未 install/build**(新包如 server-ext-onerway 无 dist)报 TS2307,且 business-minerva-payhub 自身有 TS7006/TS2347 真错 → pre-push 挂。**与 proto 改动无关**。按全局约定「无关包全量 hook 拖累」凭用户逐次授权 `git push --no-verify`。pre-commit(lint+circular)不受此影响,正常过。

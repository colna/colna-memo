---
title: Luka（sitin-rn 派生 app）
date: 2026-09-10
tags: [project, sitin-rn, luka, react-native]
---

# Luka（sitin-rn 的 `apps/luka`）

从 `apps/blueprint` 派生的 RN app（Expo SDK 57 / RN 0.86 / React 19 / Uniwind）。
当前阶段：**按 UI 稿逐屏把模板的蓝灰视觉换成 Luka 的暖色圆语言**，用户逐轮发设计图 /
实机截图驱动。

## 工作约定

- 分支：`feat|fix|chore/<事>`，PR **合入 `feature/luka-ios`**（不是 `main`）。
  因此 `pr-gate.yml` 的 `checks` 不跑，只跑 `ai-code-review`。
- 提交与 PR 作者统一 **colna**；gh 操作前先 `gh api user -q .login` 确认。
- **Luka 的改动改完直接 commit + push**（用户 2026-09-10 授权），开 PR / 合 PR / 删分支
  仍要问。
- 绝不 `--no-verify`：`pre-commit` = lint，`pre-push` = boundaries + typecheck。
  **解冲突后的合并提交也走钩子，没有例外。**

## 设计语言

- 暖色 token 在 `src/theme/colors.ts`（`background` #FDF9EE / `foreground` #3A2317 /
  `inkSoft` #6B4D38 / `tagBg` #F5EADA / `honey` / `caramel`）。
- 发现页卡组另有一套 `DeckColor`（`components/discovery/deck-theme.ts`），比全局浅一档，
  **刻意分开**。主动作色 `DeckColor.brand` #9A6547 —— Say hi 胶囊、选中的分段、空态 CTA、
  发帖页的竖杠都用它。稿子上采样常是 #AD6637（更橙一档），**统一用 token，不逐屏贴稿**；
  真要改就改这一个值，三处一起变。
- `tests/design/shape-language.test.ts` 是**迁移进度条**：按文件列白名单，一屏改完把文件
  加进去。三组条目插在同一个锚点，**多分支并行时这里必冲突**，解法是三组都留。
- **My feeds（Me 页第二段）已按稿子卡片化**：发帖条 `feed-composer-bar`、帖子卡
  `my-feed-post-card`、骨架 `my-feed-loading-skeleton` 三处都进了白名单。帖子卡是
  「一张白卡装整条帖子」（`rounded-[20px] border bg-surface`），列表不再画行分隔线；
  点赞用 `HeartGlyph`（稿子 post ActionRow 2271:182），**不是** Feed 卡片的拇指
  `FeedLikeGlyph` —— 两处不是同一个动作。

## 仓鼠素材（`src/assets/pet/`）

| 文件 | 用在哪 | 类型 |
| --- | --- | --- |
| `pet.mov` / `panda.mov` | Nearby / Chats 顶部横幅 | HEVC+alpha，**仅 iOS** |
| `tilt-head.mov` | 划卡页大图模式 | 同上 |
| `tilt-head-circle.mov` | Profile 大头像下沿（`PET_TILT_HEAD_CIRCLE_SOURCE`） | 同上；同一姿势的**圆版**：底边平切、内容顶满画布。母版 124.5 MB ProRes 4444 → 556 KB |
| `stay-all.mov` | Profile · My feeds 引导行那只站姿小崽（`PET_MY_FEED_SOURCE`） | 同上；母版 245 MB ProRes 已压到 714 KB |
| `stay.mov` | 他人资料页大图左下 | 同上 |
| `luka.mov` | Feed 发帖按钮上方、发帖页输入区右侧 | 同上 |
| `play-phone.mp4` | People·Connections 页头横卡 | **普通 MP4，两端都能播** |
| `pet-flat.mp4` | 亲密度主页 | 底色编码在视频里 |

带 alpha 的 MOV 在 `index.ts`（非 iOS）返回 `null`，`index.ios.ts` 才 `require` ——
所以 Android 上那些崽**整块不挂载**，写布局时要保证「没有它排版也不动」。
普通 MP4 装在有边界的卡里时，Android 必须 `surfaceType="textureView"`，否则圆角裁不到。

## 已完成（合入 `feature/luka-ios`）

| PR | 内容 |
| --- | --- |
| #465 #473 #480 | 划卡页：Figma 重做、大图模式、列表模式 |
| #467 | 他人资料页整屏重做 |
| #478 #481 #482 | Feed 列表、资料页底部毛玻璃、Feed 紧凑化 |
| #484 | 删号两处修复（请求缺 `user_id` + 登出不清导航栈） |
| #487 | 换成 Luka 自有域名（`*.lukasoc.com`） |
| #488 | Profile（我的）页重做 + 仓鼠扒头像 |
| #489 | 发帖页换皮 + 仓鼠从输入区右边探进来 |
| #483 | People 两页重做（Connections 仓鼠页头 / Friends A–Z 索引）+ review 五条 |

## 未定 / 残留

- **`tim.officialAccountId` 两档都不对**：非 production 那个是 Intro 测试环境的官号，
  production 是占位 `0000000`。要向后端要 Luka 自己的官号。**填错不报错**，表现是
  「系统消息那条会话永远是空的」。
- **同一个 `user_id` 缺失的 bug 在 iris / intro / lumi / koda / naya 上都在**（共用
  `packages/business-settings` 的空请求体）。是否下沉修复未定。
- 「Global」tab 内容未定：稿子写 `Nearby / Global`，代码是 `Nearby / Feed`，后端没有
  global 概念。
- 底部标签栏 IA 未定（稿子 vs 已合并的 Chats/Calls/People/Discover/Me）。
- `discoveryCardLayout` / `discovery-card-fit.ts`、`components/discover-nearby/wave-button.tsx`
  都是僵尸代码，删不删未定。
- Android `experimentalBlurMethod` 未定（关=没有真模糊，开=掉帧）。
- 远端约 11 条已合并/已关闭的 luka 分支未清。
- **全程没有在模拟器上逐项验证过**（本机无 booted 模拟器，实机在用户那边）：字母索引条
  的拖动手感、SE 上倒推的行高、发帖页仓鼠探出屏幕的量、仓鼠视频底部那条硬边。

## 相关沉淀

- [[expo-router-protected-stack-leftovers]]、[[react-async-hook-loading-stuck]]、
  [[sitin-proto-request-missing-id]]、[[design-mock-measure-and-sample]]
- [[rn-card-deck-recycling]]、[[mov-alpha-compression]]、[[rn-derived-app-theme-swap]]

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

## 审核账号（App Store / Meta 开白）

- 邮箱：`review@lukasoc.com`（商店）/ `metareview@lukasoc.com`（Meta 开白）—— 2026-09-14 从占位换成品牌域名，见 `docs/review-mode.md` §3.1。
- 两号 × 两平台的四个钉死 deviceId 在 `src/lib/review-mode.ts`（`luka[_meta]_{iOS,Android}_deviceID`）。
- ⚠️ 两个账号需在 Luka 后端真实存在且预置好资料，**后端稍后就绪**（尚未联调）。
- `lib/legal.ts` 的客服邮箱 `support@luka.example.com` 仍是占位，未换。

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
- **那只小崽的气泡是全 App 唯一的投影**（`my-feed-pet.tsx` 的 `BUBBLE_SHADOW`：10% `Colors.foreground`、
  半径 8、下移 2、Android `elevation: 3`）。这套设计讲「层级靠线与留白」，`design/theme.js` 形状语言 §4、
  `src/theme/shadows.ts`、守门测试三处都写着「没有阴影」—— 这一处是**显式例外**（用户拍板，2026-09-11），
  **规则本身没放宽**，只在测试白名单里给这一个文件开口子，并在两处文档留了注记。气泡底色 `#F7CFD5`，
  比 `Colors.peach` #F2B5BC（心形、点赞那种点状元素用）浅一档；尾巴与气泡同色，**尾巴不单吃投影**。

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
| `play-mac.mov` | Edit profile hub 右上角（抱 MacBook 站起来那只大仓鼠） | HEVC+alpha，**仅 iOS** |
| `pet-flat.mp4` | 亲密度主页 | 底色编码在视频里 |

带 alpha 的 MOV 在 `index.ts`（非 iOS）返回 `null`，`index.ios.ts` 才 `require` ——
所以 Android 上那些崽**整块不挂载**，写布局时要保证「没有它排版也不动」。
普通 MP4 装在有边界的卡里时，Android 必须 `surfaceType="textureView"`，否则圆角裁不到。

**跟着素材走的常数必须随素材重算。** `profile-portrait.tsx` 的 `PET_DROP`（圆裁小崽的下沿探出量）
由素材包围盒决定：`tilt-head-circle.mov` 并集包围盒 320×306、横向顶满、**底边空 14px** → `PET_DROP = 9`
（模拟器上平切底边正好落在圆盘切点，爪子只探出约 2pt）。换素材只换 `source` 不重算这个数，
**git 上看不出问题，只有实机 / 截屏能发现** —— 2026-09-11 就栽过一次（见 [[rn-simulator-pixel-measurement]]）。

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
| #497 | 第二轮 UI 微调（10 笔）：People 页头放大、仓鼠卡正方形、`play-phone.mov` 不再被 Android 引用、发帖等后端再退、My feeds 换白卡 + 小崽引导行、Profile 头像换圆裁小崽、两颗心归位、`PET_DROP` 重算、骨架屏统一 `SkeletonBlock`、气泡投影加一档 |
| #506（分支 `feat/luka-edit-profile-ui`） | Edit profile hub 按 2026-09-14 的 Settings 版 mockup 重做：蜜色身份卡 + 右上角抱 MacBook 的大仓鼠（`play-mac.mov`，483 MB 母版 → 2.1 MB）+ 三张 ✦ 分组卡（八行一个不少，Age/Education 特意保留）+ Done 胶囊取代底部 Save；目录抽成 `lib/edit-profile-hub.ts` 并用测试钉住八行路由；大仓鼠右缘离屏幕 28pt（稿里被屏切掉像 bug，按反馈往左收了两轮）；PR #506 已开，base `feature/luka-ios` |
| #498 | Edit profile 前两屏：Name 照 `78 · Edit name` 重做（左缘探头 `hamster-peek`、尺寸按稿 214、标题写死断行、placeholder `Alex`、不画进度条）+ Age 换 `AgeRuler` 滑尺（撤掉数字输入框与 AI tip）；`OnboardingScaffold` 加 `mascotPeek` / `backFallback`，新增 `useGoBack` 修深链冷启动 GO_BACK；squash 合并 |
| `ef63a141` `1ebfc405` `0133747f` `d68f67a0` `13578314` | 09-14 第二批：pet 素材整理（`luka.mov` 按动作更名 `point-down.mov`，新增 like/clap）、恋爱铃横幅按稿重做 + Debug Tools 预览入口（小崽 88 / 下压 35 / 对准接听键 / 上层渲染，与发帖 FAB 同档）、Edit profile 六子页重做（bio / education / height / interests / location / occupation，探头小崽 + 浮起卡片）、Me 页 tab 横滑 + 头部随滚动收起、Settings 分组名加粉底 |
| `eedeea12` | iOS 桌面小组件白屏修复：`rn-home-widget` 0.2.0 → 0.3.0，帧图改走 asset catalog（见 [[rn-home-widget-ios-blank]]） |
| `d34b92bf` | Discover / Nearby 改瀑布流：`MasonryList`（两列 + 曝光 + 滑窗口）+ 浮起毛玻璃页头（`HeaderBlurFade`）+ 视觉统一到蜜色新稿（sparks / credits / onboarding / edit-profile）+ Debug Tools PAYMENTS 分组；见 [[rn-scroll-perf-and-blur-header]] |
| `51b8029a` `2fdbcc88` `b19d4d8e` `66c2149d` `3d300cfb` | 09-15 大批量（三个会话同推）：Chats 列表按新稿重做（白卡 + 小崽招呼行置顶 + video match 条 + 爪印分割）、聊天页 B 版（蜜色发出气泡 + 毛玻璃动作浮层 + 小崽活动范围收进消息区、拖拽切 `fly.mov`）、Calls 页重做（独立白卡行 + 页头 point-down 小崽 + 加密页脚）、通知双 tab B 版 + Visitor 付费页 social-proof 版、Bond 契约三屏、全局 toast 重做、桌面组件 0.4.0 states 模式（数据驱动四态 + Debug 预览 + 冷启动误报修复）、Hellos You Sent 重设计、`eat.mov` 压缩入库（474MB→1.4MB） |
| `2fdbcc88` | 通话充值面板重做：Luka 自建原生模块 `apps/luka/modules/luka-call-recharge`（iOS Swift + Android Kotlin，`configureCallRechargePresenter("LukaCallRechargePresenter")` 指过去）——奶油底 + 可可胶囊 CTA + 蜜色选中的套餐卡 + `hamster-lean` 扒弹窗上沿；去掉双头像；主题多 `selection`/`selectionSurface` 两槽位；素材走 pod `resource_bundles`（iOS）/ `drawable-xxhdpi`（Android）；Dev Tools PAYMENTS 加预览入口。见 `apps/luka/docs/call-recharge.md` |

## 未定 / 残留

- **template-migrations 落后 26 条**：`.template-rename.json` 的 `appliedMigration` 停在 **0009**（09-09 派生），blueprint 已到 **0035**。09-15 已 merge `origin/feature/blueprint`（67 提交）并把共享包契约适配掉（rn-net 身份字段 / mock handlers / 测试期望），但**逐条迁移还没做** —— 支付三件（0015 收银台锁 / 0027 支付失败原话透传 / 0030 支付回跳不是屏）与 review-mode 新设计（0024 / 0026 邮箱由后端下发）是其中动到实现的几条。按 `template-migrate` skill 一次一条来。
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
- **edit profile 只剩 `photos` 一屏**（mockup `82`，hub 上的入口现在是那张蜜色身份卡；其余六屏 09-14 已在 `0133747f` 落完）。
  未决：photos 是单图换头像还是多图相册、interests 平铺还是分组、hub 那颗 Save 的逻辑。稿子（Figma）是**扁平图片**，量不到样式值，只能按同屏同类量比比例 —— 见 [[design-mock-measure-and-sample]]。
- **`appleTeamId` 09-15 已还原**：公司 Team（AI FANTASY，`39CFYH6W55`）生效，`apps/luka/app.config.ts` 换回真实 ID、`usesAppleSignIn` 一并恢复、entitlements 复原（**改动未提交**）。免费号绕过的坑与完整流程见 [[ios-device-free-team-debug]]。
- **Nearby 列表模式没有仓鼠**（w3 稿上有；只有卡组有）—— 要加说一声。
- **AI Code Review 在 sitin-rn 上是假绿**：#498 上 3 条 `401 Unauthorized 失败` 评论，而 check 与 workflow 都报 success（凭证失效或 workflow 吞了错误）。见 [[ai-code-review-triage]]。
- **这些还没在实机逐项验证过**（实机在用户那边）：字母索引条的拖动手感、SE 上倒推的行高、
  发帖页仓鼠探出屏幕的量、仓鼠视频底部那条硬边、气泡投影的轻重、Android 的 `elevation`。
  （#497 那批已在**模拟器截图**上逐项量过，手法见 [[rn-simulator-pixel-measurement]]。）
- **`AGENTS.md` 规则 3 与本题实际做法冲突**：规则写「合入目标唯一是 `main`，⛔ 不再有
  `feature/<App名>` 这类常驻集成分支」，而本 app 一直合入 `feature/luka-ios`。待裁决（给 luka
  写明确例外，还是调整规则）。

## 2026-09-16 全 app 代码审查（只读，未修）

基线全绿（biome 1 warning / typecheck 0 / 710 测试过 / boundaries OK），但发现按优先级：

1. **`/dev` 路由未门控**：`dev/_layout` 不查 gate、未进 `Stack.Protected`，`devFlags` 生产可用 → 深链可开 unlimitedChat/Swipes/Boosts（客户端跳扣费），同页可真删号。修复：布局内 Redirect + 进守卫组 + `__DEV__` 双条件。
2. **路由守卫缺口**：`Stack.Protected` 只覆盖 6 组 ~14 屏，其余 ~60 条深链可绕阶段门（settings/chat/profile/paywall/credits/dev…）。
3. **审核账号可冒用**：`sign-in` 手输邮箱即作 `googleEmail` 送 FastLogin，`review@`/`metareview@` 硬编码可触发（meta 登录前删号）；需后端校验或改走真实 Google 授权。
4. **DTC**：`purchasing` 锁可卡死且跨登出；成功路径无 orderId 去重、`order===null` 也 `recordPurchase` → 回跳深链可重放刷次数并上行。
5. **通话**：`onError` 不在 per-call 终态订阅（计费/心跳不停）；扣费超时重试无幂等键。
6. **跨账号状态**：discovery 缓存（明文真人资料）、`chat.matchedPeers`（URL 参数永久解锁）、chat-rounds 等不在登出清理、无 userId 前缀。
7. **`storage.ts` userInfo 解析无容错** → 脏数据冷启动永久黑屏。
8. 其余 Medium：swipe 计数覆盖、entitlements 并发、`deductForCall` 缺 `isFree`、feed 非虚拟化、PII 日志、gift 成功码、pay-times 重试、死代码/多组件文件/测试缺口。
9. 发布就绪：pre-submit 4 处身份重复（AppsFlyer/BytePlus 占位同 blueprint）、缺 Android release signing、4 项未配置。

完整报告见 2026-09-16 Daily 工作日志；路由门禁机制沉淀进 [[expo-router-protected-stack-leftovers]]。

### 修复批（分支 `fix/luka-code-review`，独立 worktree，未提交）

17:15 完成上面 1–8 的代码修复 + 死代码清理 + 9 的文档记录（`apps/luka/docs/security-hardening.md`）；
死代码删除 71 文件（45 个零引用组件 + 6 个 test-only 遗产模块 + 其测试）。验证：typecheck 0、
122 文件 / 706 tests、biome 1 既存 warning、boundaries OK。已拆 10 个提交（`7e413bca`..`84b2f568`）
在 `fix/luka-code-review`，**17:38 已合入 `feature/luka-ios` @ `f79a069d`（快进，未 push）**：先
`merge 726237eb` 解 4 处冲突（路由守卫/rate 删除、dev 三层+阶段化 Redirect、devFlag 双闸、contacts
接受删除），再把另一会话 9 个在途文件以快照 patch 纳入（`6f7cac53`）。合并态在主工作区复跑
typecheck + 706 tests 全过。**未做**：24 个多组件文件拆分（对方已拆 dev 页那批）、review-mode 0020
（Meta 开白拆墙）迁移、template-migrations 其余条目。

## 性能待办（2026-09-16 三路静态审计，P0 已修）

**2026-09-17 更新**：按 `vercel-react-native-skills` 的完整清单又修 8 个提交（列表 memo/回调/分隔线、RN Image×6 + 预取管线、Pressable/borderCurve、键盘动画转 transform、`totalBadge`/`boost.active` 派生、Intl hoist、Button 死分支、`expo-font` 声明、credits 流水 SectionList、onboarding 草稿 fallback、字体瘦身、`docs/performance-conventions.md`）。同日续批（6 commit）：与 feature/luka-ios 合流（对方独立做了预取/写盘合并）、**瀑布流自绘滚动窗口虚拟化**（`lib/masonry-window` + 7 测）、内存缓存 LRU+登出清理、轮播失焦暂停、voice-transcript 内存封顶、删死 hook、卡组震动改 worklet 闩。全在 `fix/luka-code-review` worktree（129 文件 / 738 tests）待合。**仍未做**：JS sheet→原生 formModal、SafeAreaView 54 文件 sweep、字体 config plugin（跨 app/不可 OTA）、列表缩略图（后端）、Galeria、Android 后台 Alive 心跳、Sentry DSN/OTA 配置。

P0 已修（4 commit 在 `fix/luka-code-review`：chat handlers identity / Chats 行 memo + badge identity / discovery 缓存节流+上限 / vector-icons 与 protoMap 子路径）。

**2026-09-20 更新（`feature/luka-ios` 工作区，未提交）**：用 `vercel-react-native-skills` 再过一遍 chat 页，修 9 处不与 fix 分支重复的点 —— 返回键角标不再订阅整个 `conversations` 数组（改订阅 `messagesBadge` + 当前会话未读两个数字，并删除只此一用的 `lib/chat-list.otherConversationsUnread`）、`usePeerPresence` 打字每秒重渲改到期 timeout、`useSendGift` 去掉 `coinBalance` 订阅、`ChatHeader`/`PetChatCompanion` 改 `memo`、`renderItem`/`keyExtractor`/`incomingCount` 稳定化、`bubble-audio` 改布尔选择器、`chat-time` Intl hoist；并**恢复 `useChatPrefetch` 断线**（09-17 `b8d686232` 加过，Chats 玩法层重写时丢了接线，hook 与测试此前 dead），同时把 `items` 改 ref 读取避免预取链被会话更新打断。验证：luka typecheck 0 / biome 本次文件 0 / 838 passed（4 红为 good-review + candidate-card-fields + shape-language 既有红）。

**2026-09-20 第二批（同工作区，未提交）**：用户「修复全部」→ 把上一轮列的剩余项全做掉，并**等价补齐 fix 分支的 handlers identity 修复**（`useGatedMediaGate` 改 `useCallback` + `onBlocked` ref；handlers 的 `chemistry.condition` 改 `conditionRef`）—— 这是最大的一处（此前每次屏幕渲染整条 transcript 重渲）。其余：① 入站 chemistry 刷新改 **600ms 尾部去抖 + focus 门控**（连发消息 1 趟 RPC，屏幕在别的屏上面时不刷、回来补一次）；② `lib/chat-rounds` 每消息 3 次 SecureStore 读 → **内存 hydrate**（写仍逐条经串行队列落盘），`recordTurn`/`getRounds` 后续 0 读，新增 2 条单测；③ chat store 消息缓存**上限 12 会话 + LRU 淘汰**（新 `lib/chat-cache.trimConversationCache` + 4 条单测；`retainConversation`/`releaseConversation` 钉住屏上会话，淘汰按对象键顺序、写入即触摸）；④ `peerBasicInfoCache` 上限 64；⑤ rn-image 内联 `source={{uri}}`/style 改稳定（bubble-feed/bubble-video/gift-artwork/match-card-message/greeting-gif-panel）+ social-icon/official-message-card/exchange-links-panel hoist + `video-match-card` 子路径导入。验证：typecheck 0 / biome 0 / **844 passed**（+6 新测；4 红为既有）。**跳过**：gift 网格虚拟化（fix 分支已用原生 formSheet 重写，不在待删文件上投入）、exchange 倒计时每卡 1s 定时器（倒计时本就需要秒级刷新，通常 1 张卡且到期自清）。
**P1 已清**：~~每条入站消息 3 个 RPC~~（chat 侧已 600ms 去抖；Chats 列表的批量本就是 1 趟）、~~3–6 次 SecureStore~~（已内存 hydrate）、~~chat store 消息缓存无界~~（已 LRU + 屏上会话钉住）。**P1 待排**：BlurView 6/8 层常驻（`header-blur-fade` / `bottom-blur-fade`）→ 压到 1–2 层（**视觉取舍，需真机过目**）；~~图片预取管线错配~~ **已修（09-16 晚，`prefetchImages`）**；启动侧：3 个零引用字重（`_layout.tsx:17,18,27`）、Splash 固定 1400ms（`splash.tsx:16`）、`bootstrapAfterLogin` 串行 await（`bootstrap.ts:150`）、heroui-native 改 `provider` 子路径、Android R8 未开。
**P2 待排**：整屏被无关状态拖重渲一批（`contact/inbox` 整店订阅、scene/waves presence 表、notifications rows/sections、into-you quietIds、`profile/[id]` conversations selector、`my-feed-post-card` 未 memo）；无界缓存（`senderProfileCache`、voice-transcript 内存、narrative 全量加密写；~~`peerBasicInfoCache`~~ 2026-09-20 已加 64 上限）；credits/history 非虚拟化；card-stack 每帧 runOnJS；hero-carousel 失焦不停；Android 后台 5s Alive 心跳；`use-into-you-badge` 30s 轮询 + 全量访客拉取；Sentry 占位 DSN；OTA 未配置。
审计方法：静态证据 + file:line，未做真机 profiling；量化前建议先用 Instruments/Profiler 打一枪。

## 瀑布流分页改版（2026-09-16 晚，未提交）

用户口径：nearby + feed 加载要无感，每页多加载一些；露过 **2/3** 就预取下一页；数据到位只追加、不影响已渲染；请求中菊花（不是文字提示）。

- feed 每页 40 → **60**；触发线从「距底一半」改成「露过 2/3 的卡」（按张数、视口底沿历史水位判，快速甩动不漏判）；底部文字 → `ActivityIndicator`。
- nearby 列表**滑窗换人（10 人窗 / 换 5）改追加式**：每批 20 人、池上限 120、到顶与空页都停；滑过的人不再从共享 feed 移除（切回卡片模式会再遇到）——用户拍板的行为变化。
- 卡片全部 `memo` + 回调改「把 item 作为参数传入」；`useSceneFeed` 内存上限语义修正为「到顶即停」（原 `slice` 会把新页整页丢掉、游标继续走）。
- **性能收尾（09-16 晚）**：曝光回调里的写盘合并成 trailing debounce（O(n²) IO → O(1)，追加式放大了这条债）；图片预取统一走 `prefetchImages`（原 RN `Image.prefetch` 与卡片渲染是两套缓存，同一张图下两遍）；2/3 触发线改预计算阈值（滚动每帧 O(1) 比较）。
- **纯图帖白条（09-17）**：卡片高度写死预留「正文两行」（78）导致无正文的帖子中间空一条两行白条；改为正文 `trim` 为空时只留作者行（28）。第二步：给正文 `Text` 挂 `onTextLayout` 量实际行数，1 行时把富余还给封面（`feedRenderedTextBlock`），缓存键 =「帖子 + 列宽」；**装箱仍按两行预算，不做重排**（测量后改高度会推动已滚内容）。高度纯逻辑在 `lib/feed-card-layout.ts` + 10 条单测。
- **首次切换卡（09-16 晚）**：定位到 PagerView 8（SwiftUI `TabView(.page)` → UICollectionView）**cell 首次可见才建** + 瀑布流 `onLayout` 后才挂卡起图 → 首帧成本全落在第一次切换上。修法：`MasonryList.renderLimit` 首帧只挂 16 张（约 3 屏），首次切到 Feed、翻页动画结束后放开；装箱与 2/3 触发线不受影响。**切回 Nearby（卡片模式）仍卡**，二次定位：`FeedFab` 仓鼠 `pause + 回第 0 帧` 的 seek 只在第一次暂停时真跑，恰好落在切回那一帧 —— `PetAnimation` 加 `rewindOnPause`（默认 true 保拖拽语义），FAB 传 false。两处均待真机确认。
- 验证：biome 0 error / luka typecheck 0 / 121 文件 701 tests（`good-review` 测试因并行会话的 TEMP 改动暂时挂，与本次无关）；见 [[rn-infinite-scroll-pagination]]。

## 亲密度主页重做（2026-09-16 晚，未提交）

用户要求「使用 luka 的形象 + image2 出背景图」，拍板：整屏通栏场景 / 温馨房间角落 / 默认 `stay-all.mov` 且三个按钮各播一段动作。

- **场景**：`assets/pet/room-bg.jpg`（image2 `gpt-image-2` 1024×1536 一次过，参考图 = 站立仓鼠、prompt 声明 REFERENCE ONLY；sips 转 JPEG q85，2.16MB → 359KB）。`cover` 裁切下重要物件都在中间竖带；小崽 **270pt** 时脚正好踩在地毯上（先用 ffmpeg 合成 1:1 模拟图量的，见当日日记）。
- **动作映射**（`components/pet/pet-action-clip.ts`，加动作缺键编译不过）：待机 `stay-all.mov`；feed → `eat.mov`（新增 `PET_EAT_SOURCE` 导出，此前这段素材没接线）、play → `play-phone.mov`、hug → `heard.mov`；Android 退 `hamster-happy` / `hamster-waving` / `hamster-heart` 静态图（1.6s 回站立）。
- **播放**：`pet-stage.ios.tsx` 两播放器常驻 + 按按钮 `replaceAsync` + `playToEnd` 交叉淡回待机（各段 10~11s，不能用定时器）；`pet-stage.tsx`（Android 转发）/`pet-stage-poster.tsx`（静态图版）。
- **按钮无额度**（09-17 用户拍板「点完还能一直点，不要点完就置灰」）：`stores/pet.ts` 的 `interact` 直接记录、返回 void；store 里两个 `usePetInteraction*` hook 删除；每日 3 次 / 60s 冷却仍在 `@heyhru/business-pet`（blueprint 用）。
- **摇一摇**（09-17）：`hooks/use-shake.ts`（`expo-sensors` 加速度计，> 2.2G + 1.2s 冷却，聚焦时订阅）+ `pickPetAction`（避开上一次那段，可注入 random，有单测）。`expo-sensors ~57.0.2` 进 catalog；**旧基包降级必须用 `requireOptionalNativeModule("ExponentAccelerometer")` 先探测再 require** —— `try/catch` 拦不住（Expo dev Metro 的 `guardedLoadModule` 先报红屏再吞异常），踩过，见 [[expo-missing-native-module-guard]]。
- **dev 视频落盘**（09-17）：`expo-video` 对数字 asset 会解析成 Metro 的 http uri → 边下边播、网络一抖就冻（release 无此问题）。新增 `components/pet/pet-video-source.ts`：先下进 cache（按 asset hash 命名、同 asset 去重）再交给播放器，待机与动作都走它，准备期间画静态图。见 [[expo-video-dev-asset-streaming]]。
- **尺寸归一**（09-17）：七段素材的仓鼠大小不一（jump 只有待机的 82%，并集要框住腾空那一跃）。新增 `scripts/measure-pet-alpha.swift`（AVFoundation 逐帧 alpha 包围盒中位数，ffmpeg 解不了 HEVC alpha）+ `PET_CLIP_LAYOUTS`（scale/bottom），`pet-stage.ios.tsx` 用 transform 缩放到同一大小并把脚钉在待机那条线（允许溢出方框）。换素材要重量（README 有命令）。
- **六颗按钮 / 六段动作**（09-17）：新增 `daydream`（boring.mov）/ `jump` / `run` 三段素材，**只播动画、不写数据**（`isPetInteraction` 分界）；按钮改 `PET_ACTION_ROWS` 两排三颗（上排 feed/play/hug）。三段母版（ProRes 4444，604/104/207 MB）压成 320² HEVC-with-alpha（1.4/0.9/1.2 MB），boring 必须 `--alpha-floor 24`；**母版移出仓库** → `/Users/colna/WORK/luka-pet-masters/*-master.mov`。Android 三段用近似静图（curious / pointing / coach）。摇一摇池变 6 段。
- **删煎蛋**（用户拍板，此后已无人引用）：`pet.mov` / `pet-flat.mp4` / `pet-flat.json` / `source-color+matte.mp4` / `PetFlatVideo` / `pet-flat-background.test.ts`；`PetVisual` 的 `animationSource` 改必填。`render-pet-flat` 脚本与 blueprint 那套不动。
- **验证**：biome 0 error（仅既存 `use-swipe-actions` warning）/ typecheck 0 / boundaries OK / 702 tests passed（`good-review` 挂 = 并行会话 TEMP 改动）。
- **待办**：真机确认三段动作播完回待机、连按同一按钮重播、Android 静态图换姿势；模拟器 app 停在未登录 onboarding，deeplink 到不了 `/pet`。本页 `BottomBlurFade` 8 层 BlurView，与「BlurView 压到 1–2 层」的 P1 债同向，嫌重可改纯渐变。

## 聊天键盘跟随 & Chats 两条小修（2026-09-16 晚）

- **聊天输入条「不跟手」**（`087d00e3`，已 push）：消息区 + 输入条放进同一个 `KeyboardStickyView` **整块 transform 平移**（UI 线程、零逐帧重排），删掉 iOS 的 `KeyboardAvoidingView` 逐帧 `paddingBottom` 与 Android 的 `keyboardDidShow` 一次性 inset；输入条不再收起 resting inset（多余部分藏到键盘后，只留 8pt），消息区 `bottomPadding` 变常数。模拟器 120fps 逐帧量过：上升期输入条与键盘同步（无滞后尖峰）。用户原问「要不要原生实现」——不需要，原问题是动的是布局属性。详见 [[mobile-keyboard-and-viewport]] §11 与 [[ios-simulator-keyboard-and-taps]]。
- **Chats 白卡底部被标签栏遮挡**（未提交）：`(tabs)/index.tsx` 原写 `insets.bottom + 8`（误以为 iOS 26 的 bottom 含系统标签栏），Luka 标签栏是 JS 自绘浮层 —— 改用 `useTabBarClearance()`。
- **爪印分割线**（未提交）：`chat-row-divider.tsx` 容器 `ml-[76px] mr-4` → `mx-4`，线铺到头像下方、爪印回屏幕中线。

## 会话页进场延迟（2026-09-17，真机专测）

用户报「真机点会话行 1–2s 才进聊天页」（模拟器复现不出）。真机账单（iPhone X + Metro dev 包；五点埋点 + 触摸原生时间戳，方法见 [[rn-device-perf-measurement-via-metro-logs]]）：

| 段 | 改前 | 改后 |
|---|---|---|
| 手指 → push（JS 处理） | 3–139ms（不忙，排除排队） | 同 |
| push → 首屏 commit | 87–338ms | 同 |
| commit → 消息进 store | 291–486ms（等 `getMessageList`） | **10–15ms** |
| 合计（按下 → transitionEnd） | 1.05–1.7s | ~0.8–1.0s |

改法（`b8d68623`，已 push `feature/luka-ios`）：① **消息预取** —— Chats 空闲时给前 10 个会话 `loadMessages`（`lib/chat-prefetch` 纯函数 + `hooks/use-chat-prefetch`，失焦即停、500/300ms 一笔）；② **本地快照直渲** —— `loading` 不再只认本轮 `page[id]`，`messages[id]` 有快照即视为已就绪；③ **首屏减负** —— FlatList `6 / 6 / 11` + 悬浮小崽（两段 alpha-HEVC 播放器）挪到 `settled` 后。`pnpm luka:test` 716 passed（新增 6 条单测）。

**试过无效**：仓鼠 perch 视频 A/B（去掉无变化，非瓶颈）；`animation: "none"` 页面瞬现（用户体感仍慢，已按用户要求还原）。
**残留**：余下 ~0.8–1.0s 是 iPhone X + dev 包本身（React dev 模式 / 未压缩 / 每条网络日志走 WS 到 Metro）；**luka 没有 nightly**（`apps.json` `nightly:false`），要验 release 体感需手动发 EAS preview 包。
**顺带**：P1 里「消息缓存无界」等未动；本次只做进场路径。

## 低余额充值抽屉（low-sparks）重设计（2026-09-17，未提交）

聊天里余额不足时升起的充值抽屉，从 blueprint 继承的 3px 直角语言迁到 Luka 语言：
白面板 28 圆角 + 抓手、标题组居中、选中包卡换焦糖描边 + 奶油底 + 对勾、CTA 与
POPULAR 都是整圆（稿子：image2 B 版 `20260917-luka-lowsparks-b.png`）。

抽屉本体改用 app 通用 `components/ui/bottom-sheet`（HeroUI/gorhom）。两个实测坑见
[[rn-heroui-bottom-sheet-clipping-and-mount-open]]：① 面板外内容被裁，稿子里「小崽
扒上沿探头」做不到 → 换成面板内的抱心小崽；② 挂载即 `visible=true` 不展开，debug
预览要走点击。Debug Tools 的 PREVIEWS 卡新增「Low sparks sheet」预览行（`previewOnly`）。

## 欢迎页换静态 hero & 开场片挪到注册成功后（2026-09-17，未提交）

用户口径：登录页不再播视频，改用 Figma「ios 新包规范 / 03 · Pet hatch」（节点 `321:899`）的图与文案
（`Poo` → `Luka`），其余入口保持现状；开场片改为**首次注册成功后**播放、可跳过。

- **欢迎页**：`components/login/welcome-hero.tsx`（标题 `Say hi to {BRAND}.` + 仓鼠 + 两行说明）。
  仓鼠图 = 节点 `321:911` 的 `@3x` 导出，按 alpha 收紧成 `assets/mascot/hamster-hello.png`
  （718×911，808 KB）。稿里的 `Name him` 主按钮不采用 —— 本屏的动作是登录。
  三个零件为奶油底换了色（白 → `foreground` / `muted` / `tintColor: inkSoft`）；
  Apple 官方白按钮与 Android Google 胶囊保持不动（前者是 `guest-path.test.ts` 钉死的）。
  旧视频底板 `welcome-video-backdrop.tsx` 删除。
- **开场片**：抽 `components/intro/intro-film.tsx`（片 + 跳过键 + AUTO PLAYING），
  `how-it-works`（游客 / 审核员）与新增 `app/intro-video.tsx`（注册成功后）共用；
  结果页落库后 `replace("/intro-video")`，出口仍是 sign-in → Into You / 其余 → 首页。
  **「首次」不落盘**：onboarding 一生只走一次，`profileComplete` 就是闸门；中途杀进程不补播。
- **口径**：登录页的静态 hero 与 `Poo` 的替换都写在这个 app 自己的组件里，派生包改 `copy/brand.ts`
  的 `BRAND` 即可跟着变。
- **预加载 + 立刻弹起（用户追加要求，`c76b8e98`）**：`onboarding/_layout.tsx` 一进来就
  `preloadIntroVideo()`（`components/intro/intro-video-preload.ts`，复用 `pet-video-source` 的落盘）；
  `intro-video` 路由待在**独立的 `authed && !pendingDeletion` 组**（横跨 onboarding → 产品两阶段），
  `summary.tsx` **先 `replace("/intro-video")`、再 `await setUserInfo(snapshot)`** ——
  否则 `profileComplete` 一翻、onboarding 组退场，人会先被甩到 tabs 闪一帧。
  `complete-onboarding.ts` 因此改成只回传 `UserInfo | null`，应用是调用方的事。
- **验证**：biome / typecheck / 728 tests / boundaries 全过；新建临时模拟器截图核对（与稿偏差 ≤ 屏高
  多出的 30pt 的均分）。`/intro-video` 的实际播放入口未在模拟器复现（要走完 onboarding + 真后端）。

## My feeds 空态迁移（2026-09-17，随 `9072a06b` 入库）

`components/my-feed/my-feed-status.tsx`：旧的 3px 直角白方块 + 蓝灰 inbox 图标换成
**不套卡、直接画在背景上**：抱相机的小崽（`hamster-camera.png`，132×170）+ 标题/说明，
**不铺白底、整体比整屏空态小一号**（用户口径：不要白背景；白卡版被他否了）。卡内不放 CTA
（Me 页那段头顶就是 composer）。同文件的出错态一并换新语言（`pill-outline`）。
`shape-language` 白名单与 `assets/README.md` 同步。验证：typecheck 0 / 全量测试 / biome clean；
iPad 模拟器（feed 为空的账号）深链 `/my-feed` 实拍确认。

## Me 页标题栏固定 + 「漏底」修复（2026-09-17，`549778be`）

- **标题被内容盖住**：`components/me/profile-nav-bar.tsx` 加 `bg-background` + `zIndex: 20`
  —— 收起中的页头（绝对定位、后画的兄弟）原本会把「Profile」整条盖掉。
- **漏底**（用户「漏底了」+ 截图）：页头在裁剪层缺 `overflow-hidden` 时整幅漏出，头像那条图
  涌进标题栏**上方**的状态栏带；那条带子比标题栏还高、`zIndex` 盖不到。两步修：
  ① `app/(tabs)/me.tsx` 的裁剪层补 `overflow-hidden`（页头在标题栏下沿被平切）；
  ② `ProfileNavBar` 往上贴一块 `insets.top` 高的同色底（absolute、不参与布局、不吃触摸）兜底。
- **验证**：iPhone 17 模拟器「拖动进行中」连拍：页头在标题栏下沿平切、状态栏带全奶油、at-rest 无变化；
  typecheck 0 / 126 files · 746 tests / biome clean。iPad（窗口化）上 Me 页内容不够长（滚不动也不回弹），
  「收起中」那一帧复现不了 —— 复现要靠可滚动页面（iPhone / My feeds 有帖时）。

## 相关沉淀

- [[expo-router-protected-stack-leftovers]]、[[react-async-hook-loading-stuck]]、
  [[sitin-proto-request-missing-id]]、[[design-mock-measure-and-sample]]
- [[rn-card-deck-recycling]]、[[mov-alpha-compression]]（新增「坑六：母版的 alpha 1–15 灰洗」）、[[rn-derived-app-theme-swap]]
- [[rn-simulator-pixel-measurement]]（截屏量 UI 几何）、[[colna-fastembed-cache]]（模型缓存落在 cwd）、[[simulator-fresh-device-signed-in]]（新建模拟器拷 keychain/data 拿已登录会话）
- [[ios-device-free-team-debug]]（免费 Personal Team + iOS 16 设备真机调试）、[[rn-scroll-perf-and-blur-header]]（滚动卡顿 + BlurView 页头）、[[rn-home-widget-ios-blank]]（小组件白屏）
- [[clash-tun-blocks-github-ssh]]（Clash TUN 挡 git over SSH，改一次性 HTTPS URL）

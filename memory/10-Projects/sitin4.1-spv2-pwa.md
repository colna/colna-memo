---
title: sitin4.1 SP V2 · PWA 前端进度
date: 2026-08-03
tags: sitin4.1, spv2, ce, snapchat, app-pwa, 进度
---

# sitin4.1 SP V2 · PWA 前端进度

把 PWA 从「IG 单类型」参数化为「IG + Snapchat 多社媒」。真源 RFC:飞书 wiki [V33Zwbua](https://presence.feishu.cn/wiki/V33ZwbuaKiqtVHkr4YCc29vWnZK)(含第三部分前端任务拆解 + 第四部分服务端 CE 接口)。

- **代码分支**:`sitin-next2` → `personal/zz/sitin4.1`(基于 `feature/pwa`)
- **PR**:[#824](https://github.com/presence-io/sitin-next/pull/824)
- **dev 预览页**:`/dev/spv2`(pages/dev/Spv2ModalsPreview)
- **服务端 CE**:contact-exchange-service PR #104(已 merge 大部分:platform_online / card_type / OSS 头像)

## 状态总览

| Phase | 任务 | 状态 | 备注 |
|---|---|---|---|
| **0 基础层** | FE-0.1 社媒类型/配置表(SOCIAL_CONFIG/CardType) | ✅ 完成 | 并行 agent 又按端上契约拆了 robotUrl/checkUrl/loginUrl + SC 桌面 UA |
| | FE-0.2 insStore 多社媒化(map + selector) | ✅ 完成 | authByPlatform/loggedInByPlatform/abnormalByPlatform + selectPrimaryPlatform |
| | FE-0.3 Bridge platform 透传 | ✅ 完成 | open/start/check 三方法可选 platform |
| | FE-0.4 授权回调多社媒写回 | ✅ 完成 | finishPWAInsTask 补 platform;handleOpenSocialLogin 补 snapchat |
| **1 授权登录 UI** | FE-1.1 授权卡按社媒 | ✅ 完成 | AuthorizeSocialCard(platform),已接 needsInsAuth 链 |
| | FE-1.2 登录弹窗按社媒 | ✅ 完成 | showInsModal 严格对齐 Figma 2956/3045,IG/SC 真素材 |
| | FE-1.3 授权抽屉多社媒三态 | ✅ 完成(金额占位) | 断连触发已接(handleOpenSocialLogin/checkInsAbnormal);金额仍占位待取数 |
| | FE-1.4 单登录态授权另一社媒卡 | ✅ 完成 | PausedCard authorizeOther,已接真实渲染链(scAuthed 判据) |
| **2 任务系统** | FE-2.1 一次性任务 Authorize snapchat(id 137) | ✅ 前端完成(待后端确认) | 08-03:enum/registry/标题/点击(showInsModal snapchat)+ finishTask(137)+$0.5 奖励弹窗。**待后端确认返回任务 137 及奖励金额**;Social Connect 金额取数另计 |
| | FE-2.2 双登录态强制 CE 交换任务 | ⬜ 未做(前端可做) | 数据源=listUserInsExchangeOrder(cardType)+platform_online;需新增阻塞卡进渲染链。**当前最大前端缺口** |
| | (增)登录弹窗授权后按平台收回+完成 | ✅ 完成 | 08-03 fix:showInsModal 用 justAuthed(存续期 false→true)驱动成功→完成→关闭;按平台选任务;IG 保留 insState 重置、SC 不重置 |
| | (增)Chat 空状态放行 | ✅ 完成 | 08-03:授权任一社媒(igUsable‖scAuthed)即进聊天,IG 原口径不回归 |
| **3 CE 链路+接口** | FE-3.1 女主动发卡 cardType 动态决策 | ✅ 完成 | 优先服务端 card_type,兜底 selectPrimaryPlatform(IG>SP) |
| | FE-3.2 InsExchangeBubble 按 cardType 渲染 | ✅ 完成 | 平台名 Instagram/Snapchat 按 payload.cardType |
| | FE-3.3 CE 接口补 cardType | ✅ 完成 | createBlurredCardOrder 传 cardType;**proto 已 regen 解除阻塞** |
| | FE-3.4 CE 机器人调度按社媒分支 | ✅ 完成 | startRobot 按订单 cardType 拉对应平台小崽 |

## 已完成的「实质逻辑」vs「仅 UI」区分

- **真逻辑(Phase 0)**:状态模型(insStore map/selector)、bridge platform 透传、授权回调按平台写回、类型/配置表。
- **UI + 平台化触发(Phase 1)**:各卡/弹窗/抽屉点击真的调 `openSocialProxyWebView(platform)` / `showInsModal({platform})`;FE-1.1/1.4 已接进 AiGoldCashoutCard 真实渲染链。

## 未接线 / 占位(follow-up)

1. ~~授权抽屉生产触发~~ **已接**:断连(handleOpenSocialLogin/checkInsAbnormal)时弹,仅 expired 才弹。剩:金额取数(见 2)。
2. **奖励金额写死** —— Claimable $X / Rewards $X / 单登录卡金额都是占位;需 `listUserInsExchangeOrder` 带 cardType 分组求和(后端已可返回 cardType,前端未接)。
3. **Snapchat 真实授权闭环** 依赖 social-proxy-server 官网 SC 登录 + APK 回调传 platform(PWA 侧已就绪;Android p/ljb/snpachat 三端能力已支持 platform)。
4. **InsRobotUserInfo.insId/insAvatar 未改名**(语义已泛化,彻底改名留 follow-up)。
5. Snapchat 弹窗 SC 素材已从 Figma 导入(ic_logo_snapchat.svg / bg_snapchat_login.png);其余 SC 图后续按需补。
6. **埋点事件名仍是 IG 名**(showInsModal 里 pwa_ins_login_button_click / pwa_earning_ins_task_page_one|two_click / pwa_perm_ig_authorization 等)——08-03 只补了 `platform` 字段区分,**彻底重命名待与数据侧对齐**;⚠️ 影响 INS 成本漏斗口径(全局记忆按内容名取数)。
7. **登录弹窗成功页 showcase 图仍 IG**(bgInsAuth),缺 SC 素材。
8. **taskId 137 完成依赖后端**:前端已 finishTask(137),但需后端返回该任务+奖励金额,否则奖励取 0、不弹窗。

## 关键约定 / 踩坑(本项目)

- sitin-next2 有 husky **commitlint**:commit subject 必须**小写开头**(subject-case)。
- app-pwa 是**面向英文用户产品**,UI 文案用英文(CLAUDE.md「中文文案」不适用于此包),注释中文。
- **在 sitin-next2 干活**(主树 sitin-next 有脏 proto submodule + 并行 agent worktree,避开);同一 origin。
- rebase 换了底层 bridge.ts 后**必须重跑 tsc/lint 再 push**(曾漏跑)。
- Figma download_figma_images 的 image dir 是**工作区根**,素材要 mv 进 sitin-next2。
- **合 main 遇 proto submodule 冲突**(08-03):用 `git merge-base --is-ancestor A B` 判断祖先关系,谁包含对方取谁(本次 ours 19259794 ⊇ main dad3cb70);`src/gen` 是生成代码不信 auto-merge,`git checkout HEAD -- src/gen` 取超集侧再 `pnpm --filter business-pwa-proto build` 重建 dist。详见 troubleshooting/colna 无关,记在 daily 08-03。

## 08-03 推送记录(PR #824)

- `5bf52bcb4` feat: add snapchat authorize one-time task (id 137)
- `293ad1e98` fix: close & complete social login modal per platform
- `8507a8f22` feat: allow chat access when any social authorized
- ~~`b9674ae22` Merge origin/main~~ **已撤销**(见下)
- `731bc2bfc` feat: ai code review(他人推:CI 流式 workflow,但 reasoning_effort 误留 max)
- `c480df8e2` 交换请求弹窗多社媒化 Social Requests(figma 7268:969)
- `3da1760d2` fix(ci): reasoning_effort max→low(修 0 字空评审)
- `bce6f89a1` 统一多社媒可用口径 selectPlatformUsable,修状态一致性(CR 批1:App 过期放行/InsExchangeBubble/startRobot 返回值/bridge fallback)
- `fc282c3dc` snapchat 授权 finish 流程对齐 IG(bindSocial/finishSocialBind)
- `78c6de3b6` chore: 更新 proto 到 release/test b23ea527 + regen gen
- `bfe22d104` 结合 CE PR#104 修 CR 批2:setInsState 删/发卡按平台取名片/批量按平台校验 ← **当前 head**
- PR#824 标题已改为「SP V2 PWA 多社媒化(IG + Snapchat)· Phase 0/1/3 全链路」

## CR 已修 / 待办(2 轮 AI Review)

- **已修**:App.tsx snapchat 过期放行(selectPlatformUsable 排除 abnormal)、InsExchangeBubble 按平台判登录、startRobot 接返回值失败不置登录态、bridge legacy 降级加 `platform!=="instagram"` 守卫、showInsModal 删多余 setInsState(false)(修 IG 重连抽屉失效)、发卡 TIM payload 按 cardType 取 getContactCard(不再写死 IG 账号)、handleAcceptExchange 校验批量所有涉及平台(不只 messages[0])、getContactCard 参数 CardType→number。
- **未改(follow-up)**:#3 bindSocial 失败继续(旧 IG best-effort 设计)、showInsModal 联合参数 overload(仓内无旧调用)、/dev/* DEV 门禁(既有惯例统一改)、CardType 本地/proto 彻底统一、insStore/bridge 单测、去 222222/333333 调试日志。
- **卡后端(FE-2.2)**:listUserInsExchangeOrder 订单仍无 cardType、CE 交换 TaskType 未加(b23ea527 仍无)。CE PR#104 只覆盖 blurred-card/free-exchange 侧(cardType/platform_online/saveCard/getCard by type 已就绪)。

## ⚠️ 08-03 教训:不要把 main 合进 base≠main 的 PR

- PR #824 的 **base 是 `feat/sitin4.1`**(不是 main)。我按用户「pull 一下 main」把 origin/main 合进 head,结果 **main 相对 feat/sitin4.1 超前 72 提交 / 152 文件**全灌进 PR diff → 文件数从 **44 炸到 194**(首页 30 全是 minerva 无关代码,连累 AI review 审错对象)。
- **修复**:`git reset --hard 8507a8f22`(合并前)+ `git push --force-with-lease` 撤销 merge,PR 回到 44 文件。
- **规则**:给 PR「合基线」时,合的必须是**该 PR 的 base 分支**(这里是 feat/sitin4.1),**不是 main**。base≠main 时合 main 会把两者全部差异塞进 diff。下次先 `gh api .../pulls/<n> --jq .base.ref` 看清 base 再动手。

## 08-06 多社媒全链路对齐(6 commit,进 PR #824)

| commit | 内容 |
|---|---|
| `094b8382c` | 冷启动补 SC 掉线探测(checkInsAbnormal 并行探 IG+SC)+ checkSocialProxyPageAbnormal 降级非 IG 隔离 |
| `64b925f90` | **授权判据统一到后端名片 hasContactCard**(Proto 21008):冷启动回填 authByPlatform,tryStartInsRobot 去 insId/insState |
| `be1b79d8d` | 完单按订单平台判登录(靠后端新加 `InsExchangeOrderInfo.card_type=13`):checkPendingOrders 填 cardType、handlePeerAccepted 加平台 gate、入口去 isInsLoggedIn 一刀切;方案 B 保留 Social Requests 弹窗 |
| `9f535cc09` | **未授权/登录态失效两状态解耦**:resetInsState 改回登出全清(只被 useUserInit 登出调)、SC 补 setPlatformLoggedIn、showInsModal 授权判据统一 authByPlatform |
| `1f173b776` | 授权入口按状态分流 `authorizeOrLogin`:未授权→大抽屉 showInsModal / 授权过→直接 openSocialProxyWebView;四入口(任务/workspace/胶囊 PausedCard/chat)全覆盖 |
| (未提交) | review 修 handleAcceptExchange 回执 accountID/avatarUrl 按 cardType 取名片(女被动漏了女主动的处理)+ platformLoggedIn 复用模块级 |

**三态解耦最终口径**:授权=`authByPlatform`(hasContactCard 回填/finishSocialBind 置/登出清)、登录=`loggedInByPlatform`(startRobot/探测/openSocialLogin,IG+SC 都写)、异常=`abnormalByPlatform`;三独立字段独立数据源,判"授权过"全局统一 authByPlatform。**小抽屉 showSocialAuthDrawer 只在系统检测(checkSocialProxyPageAbnormal/openSocialLogin)登录态丢失时弹**;入口点击按状态分流。

**观察(未改,产品定)**:App.tsx authBlock 已多社媒但枚举名 ig-unbound/ig-login-lost 遗留;AiGoldCashoutCard:66/Settings:31 判绑定仍用 insId(IG),SC 管理入口待定。

## FE-2.2 强制 CE 交换任务 · 前端方案(后端已给 TaskType,待实现)

RFC 需求 6 / 图 3.2(wiki `V33ZwbuaKiqtVHkr4YCc29vWnZK`)。这是多社媒最后一块没做的。

**触发场景**:双登录态用户、当前仅单社媒在线、有待完成男方 CE 订单属**另一(未登录)社媒** → 强制任务(优先级最高、红黄绿可见、阻塞流程、完成后解除、点接受跳官网登录)。男主动订单倒计时同男端、结束消失不可关闭;系统代发无倒计时可关闭。**后端判触发并下发**,前端只渲染。

**后端数据结构**(挂 convState.tasks,与 probe 同源):
```ts
{ id, taskType: TaskType.SOCIAL_ACCOUNT_AUTHORIZATION /*=4*/, ceType: IG|SC, reward, timeoutSeconds }
```

**前端方案(复用 probe 那套 anomaly 机制)**:
1. proto:`TaskType.SOCIAL_ACCOUNT_AUTHORIZATION=4` + `Task.ceType`(建议复用 CardType 值);
2. `useChatConv.getSocialAuthTask`(对齐 getProbeTask);
3. `chatAlertStore` 加 socialAuth info + 置**最高优先级**;
4. `GlobalAnomalyBanner` 渲染 + 红黄绿跳转 + 新阻塞任务卡(参考 ProbeTaskDrawers);
5. 倒计时复用 `useTaskRemaining`(锚 createdAt+timeoutSeconds);男主动(timeoutSeconds>0)不可关、系统代发可关;
6. 点 Accept → `cardTypeToPlatform(ceType)` → `openSocialProxyWebView(platform)`;
7. 完成解除以后端下发的 convState 移除 task 为准(前端登录成功后 refresh ListConversationStates)。

**待用户拍板 3 点**:① ceType 是否=CardType(IG=1/SC=2);② "可关闭"用 `timeoutSeconds===0` 还是单独字段;③ social-auth 优先级是否绝对高于 probe。

## 三个「一次性/社媒授权」任务(RFC KpwEw8wU / V33Zwbua · 2026-08-12 起,待 2 个 id)

RFC 原文(「新增一次性任务」格):Authorize snapchat 💲0.5;未登录对应社媒且有未完成社媒交换订单时展示 Social Connect on Instagram / Snapchat(金额=累积待处理订单金额合计),点击跳转官网登录。

**Task 1 — Authorize Snapchat 💲0.5(taskId 137,已知,可直接做)**
- 现状:挂在 Welcome Checklist(`taskRegistry` 的 `BindSnapchatAccount` 有 `v4OnboardingOrder:5 + v4OnboardingGating:false`)。用户判定「位置写错」。
- 改法:改 `types/taskRegistry.ts` 的 `[TaskId.BindSnapchatAccount]` —— **删** `v4OnboardingOrder/v4OnboardingTitle/v4OnboardingGating`,**加** `category:"onboarding" + onboardingOrder:5 + onboardingTitle:"Authorize Snapchat"` → 从 Welcome Checklist 挪到**首页 Task 栏**(`OnboardingTaskList`,基础列表=`onboardingTaskConfigs()`=category:onboarding)。奖励 💲0.5 仍取后端 `task.reward`。颜色按其在 Task 栏的 index 轮转(黄/粉/紫/蓝)。

**Task 2/3 — Social Connect on Instagram / Snapchat(taskId 待用户给,先预留)**
- **预留 id**:⏳ 待用户提供 2 个 taskId。custom 段现用到 **200013**(SecondEarn=200001…SeventhVideoDuration=200013),新 id 大概率 200014/200015,但**以用户给的为准**。占位命名建议 `SocialConnectInstagram` / `SocialConnectSnapchat`。
- **归属**:首页 Task 栏;`source:"custom"`(前端维护,非后端下发)。
- **展示条件**:该社媒**未登录/未授权** 且 **有该社媒未完成 CE 交换订单**。
- **金额(动态)**:该平台**累积待处理订单金额合计** = `listUserInsExchangeOrder()` 按 `order.cardType` 汇总 `pwaFollowReward`——**复用** `components/showSocialAuthDrawer.tsx` 里 `claimableByPlatform()` 的口径。无订单→不展示。
- **点击**:`openSocialProxyWebView(platform)` 跳官网登录。
- **机制坑**:现有 custom 任务(`useTask.ts:148` 的 `customTasks` useMemo)是**静态** `CUSTOM_TASK_CONFIGS.map`(`deps:[]`、reward 写死)。Social Connect 需**动态**(依赖 authByPlatform + 异步拉订单算金额 + 条件过滤),不能照抄——要扩展 customTasks 注入逻辑(读 insStore 授权态 + 拉 pending orders 汇总 + 仅在有订单时注入)。

**关键文件**:`types/task.ts`(TaskId 枚举 + `CUSTOM_TASK_CONFIGS`,L100)、`types/taskRegistry.ts`(TaskDef 注册)、`hooks/useTask.ts`(L148 customTasks 注入 / L300 合并)、`http/insApi.ts`(`listUserInsExchangeOrder`)、`utils/bridge.ts`(`openSocialProxyWebView`)、`pages/Home/OnboardingTaskList.tsx`(渲染)。

**待用户**:给 Social Connect IG / Snapchat 两个 taskId → 补占位后实现 Task 2/3。Task 1 可先做。

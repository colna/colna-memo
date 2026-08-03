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
| | FE-1.3 授权抽屉多社媒三态 | 🟡 UI 完成/未接生产 | showSocialAuthDrawer,含奖励标注(Claimable/Rewards)。**生产没触发点**、金额写死 |
| | FE-1.4 单登录态授权另一社媒卡 | ✅ 完成 | PausedCard authorizeOther,已接真实渲染链(scAuthed 判据) |
| **2 任务系统** | FE-2.1 一次性任务 Authorize snapchat + Social Connect | ⬜ 未开始 | taskRegistry 加条目 |
| | FE-2.2 双登录态强制 CE 交换任务 | ⬜ 未开始(**已解阻塞**) | platform_online 现可用;授权+在线+订单社媒判定 |
| **3 CE 链路+接口** | FE-3.1 女主动发卡 cardType 动态决策 | ✅ 完成 | 优先服务端 card_type,兜底 selectPrimaryPlatform(IG>SP) |
| | FE-3.2 InsExchangeBubble 按 cardType 渲染 | ✅ 完成 | 平台名 Instagram/Snapchat 按 payload.cardType |
| | FE-3.3 CE 接口补 cardType | ✅ 完成 | createBlurredCardOrder 传 cardType;**proto 已 regen 解除阻塞** |
| | FE-3.4 CE 机器人调度按社媒分支 | ⬜ 未开始 | useInsTaskInit startRobot 透传 platform |

## 已完成的「实质逻辑」vs「仅 UI」区分

- **真逻辑(Phase 0)**:状态模型(insStore map/selector)、bridge platform 透传、授权回调按平台写回、类型/配置表。
- **UI + 平台化触发(Phase 1)**:各卡/弹窗/抽屉点击真的调 `openSocialProxyWebView(platform)` / `showInsModal({platform})`;FE-1.1/1.4 已接进 AiGoldCashoutCard 真实渲染链。

## 未接线 / 占位(follow-up)

1. **授权抽屉(FE-1.3)生产触发点缺失** —— 组件 + `showSocialAuthDrawerFromStore()` 已建,但没有代码在「授权过期/丢失」时弹它(现仅 /dev/spv2 按钮)。需接异常判定链。
2. **奖励金额写死** —— Claimable $X / Rewards $X / 单登录卡金额都是占位;需 `listUserInsExchangeOrder` 带 cardType 分组求和(后端已可返回 cardType,前端未接)。
3. **Snapchat 真实授权闭环** 依赖 social-proxy-server 官网 SC 登录 + APK 回调传 platform(PWA 侧已就绪;Android p/ljb/snpachat 三端能力已支持 platform)。
4. **InsRobotUserInfo.insId/insAvatar 未改名**(语义已泛化,彻底改名留 follow-up)。
5. Snapchat 弹窗 SC 素材已从 Figma 导入(ic_logo_snapchat.svg / bg_snapchat_login.png);其余 SC 图后续按需补。

## 关键约定 / 踩坑(本项目)

- sitin-next2 有 husky **commitlint**:commit subject 必须**小写开头**(subject-case)。
- app-pwa 是**面向英文用户产品**,UI 文案用英文(CLAUDE.md「中文文案」不适用于此包),注释中文。
- **在 sitin-next2 干活**(主树 sitin-next 有脏 proto submodule + 并行 agent worktree,避开);同一 origin。
- rebase 换了底层 bridge.ts 后**必须重跑 tsc/lint 再 push**(曾漏跑)。
- Figma download_figma_images 的 image dir 是**工作区根**,素材要 mv 进 sitin-next2。

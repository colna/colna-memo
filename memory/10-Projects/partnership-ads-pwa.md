---
title: Partnership Ads · PWA 任务技术方案
date: 2026-07-27
tags: [sitin-next, app-pwa, partnership-ads, 任务, 技术方案]
---

# Partnership Ads · PWA 侧任务通知

> 真源文档(飞书 wiki):`Partnership Ads 自动化投放v1.0 RFC`
> `https://presence.feishu.cn/wiki/CnxLwWO03ion2FkwKLWcXsqsnvd`(底层 docx `Mfhndw9woo8Ibvx6wjgcNfyan8e`)
> PRD 原文(需求方 钟绍军):`https://presence.feishu.cn/docx/TQ4Admp4OoR45Sxx4PpchYWTnWN`
> 分支:`feature/pwa`

## 需求闭环
后端(邀请成功)下发任务 → 素人首页看到任务(按钮 **Go**)→ 点击经端能力跳 IG deeplink(`instagram://branded_content_ad`)→ 手动批准 → 回 PWA → 确认已批准 → 完成加钱。脚本部分(转专业账号)已完成,本次做 PWA 任务通知。

## 关键决策(定稿)
- **不推 WS**:任务出现与完成都走**用户操作触发的 refresh / 单次查询**。
- **不轮询**:查询接口后端代调 Meta,轮询会被判刷接口。查询只在**用户动作**触发(回前台一次 + 点 Received 一次),带节流(`lastCheckAt` <30s 跳过)。
- **两层状态**:服务端 `TaskStatus`(`notStart/finish`,权威、发奖依据)+ 客户端 `verifyPhase`(Go/awaitingReturn/checking/Received/Finish,仅 UI,独立 store 持久化 localStorage)。
- **持久化**:`verifyPhase`+`awaitingReturn`+`lastCheckAt` 落 localStorage(套壳 WebView 跳 IG 期间可能回收重载),重载后以服务端 `status` 为准重建。
- **回前台查询**:用 `visibilitychange`(独立监听,**不限首页**)消费一次性 `awaitingReturn` 查一次;Received 手动查一次。
- **完成加钱**:APPROVED → `useTask().finishTask(taskId)` + 奖励弹窗(建议 PWA 主动 finish;发奖归属待与后端确认)。

## 现有能力盘点(feature/pwa)
- 任务体系:`http/taskApi.ts`(`listAllCommonTasks` / `getCommonFinishedTasks` / `finishUserCommonTask`)、`TASK_REGISTRY`(TaskDef)、`useTask()`、奖励弹窗 `REWARD_MODAL_TASKS`。
- **现有任务刷新时机**(`refreshTasks`):①登录后(`useUserInit`,TIM 登录后)②进首页(`Home/index.tsx`:`pathname==="/" && !isBackground`)③在首页时回前台(`isBackground` 来自 `useVisibility`/visibilitychange)④完成任意任务后。**无 WS、无轮询**。⚠️ 回前台刷新**仅在首页**,回前台时若在别的页不刷任务列表。
- 端能力:`utils/bridge.ts` 有 `openSocialProxyWebview`/`openInsWebView`(开容器 webview),**没有跳 IG 原生 app 的 deeplink 能力 → 需客户端新增 `openDeeplink({url})`**。

## 前端任务拆解(8 项,M1~M4)
- M1 骨架:①TASK_REGISTRY 加 TaskDef ②usePartnershipStore ③首页三态卡(Go/Received/已完成)
- M2 跳转:④bridge `openDeeplink` ⑤点 Go 跳 IG + 置 awaitingReturn
- M3 闭环:⑥`usePartnershipVerify`(visibilitychange 回前台单次查+节流)+查询接口封装 ⑦Received 手动查 + finishTask 加钱
- M4 收尾:⑧埋点 + 边界验证(套壳重载恢复 / 节流 / status 为准)

## 待确认(阻塞)
1. 后端查询接口 `checkPartnershipStatus` 入参/出参枚举/确认延迟范围。
2. 该任务纳入 common task、分配 taskId。
3. 发奖归属:PWA finish 还是后端自动 finish。
4. `openDeeplink` 端能力:客户端有无现成通用能力,没有需新增(定能力名/参数)。
5. 「用户批准后不再回 PWA / 不点 Received」的尾巴风险(无 WS 下只能等下次回来触发)。

## 踩坑
- lark-cli 飞书身份被「步川」顶掉(同 app 只存一个 user token),写文档报 `4030004 缺编辑权`;重新 `auth login` 用张峥扫码切回。判断真因:`docs +update` 返回 `ok:true` 但 `data.result:failed` + warnings 才是真实结果,不能只看 ok。
- 飞书 wiki 文档写入需张峥有**可编辑**(仅可阅读能 fetch 但写入 4030004)。
- mermaid 图用 `<whiteboard type="mermaid">…</whiteboard>` 主 Agent 直插;整篇重写用 `overwrite`(含 `<title>` 保留标题)。

## 2026-07-30 实现更新

- 卡片已独立为首页「Become partner」区块,位于 Win Free Cash 上方;核心闭环由 `usePartnershipAds` 持有。
- 查询并发由 30s 时间节流改为 `useLockFn` 在途锁:上一轮结束即可再查;回前台、挂载补查、手动 Received 均可触发 `finishTask(2000)`。
- `getCommonFinishedTasks` 的后端语义已扩展为返回带 `status` 的用户任务,不能再把返回的每条记录都视为完成。当前 `useTask.ts` 未过滤 `status`,会把 `PENDING` 的 2000 标成 finished 并隐藏卡片;前端过滤还是后端恢复只返 FINISHED 尚待定口径。
- 若卡片未来改成复用通用任务卡,仍须保留 `usePartnershipAds` hook,否则回前台验证闭环会丢失。

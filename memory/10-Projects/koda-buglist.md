---
title: Koda(sitin-rn)张峥名下 Buglist 梳理
date: 2026-09-02
tags: [sitin-rn, koda, buglist, feishu-base]
---

# Koda 张峥名下 Buglist

**真源(飞书多维表格)**:`RN Android Koda bug list汇总`
`https://presence.feishu.cn/wiki/AiWBwAFiwiZ3L8kYi1Rcdvklnld?table=tblGoES4Sw6IgEmt&view=vewYmnlr2n`
- base_token `EDjmba79Ga41Cbs7lRUcrjUPnGf` / table `tblGoES4Sw6IgEmt`
- 指派人字段=user 类型;张峥 open_id `ou_9607323e76870d4bfed98efd5736f60d`(本 app 下解析出的即此值,与全局记忆一致)。
- **取数坑**:`record-list --filter-json` 对 user 字段筛选屡试 0 条;可靠做法是 `record-search --keyword 张峥 --search-field 指派人`(返回 20 条,has_more False)。envelope 里行数据在 `data.data`(纯值数组),列名在 `data.fields`(名字数组),非 `items`。

## 现状(2026-09-02 快照)

张峥名下 **20 个 bug**,全部 `状态=未开始`、`APP=Koda`、优先级 **19×P1 + 1×P2**(R00229)。表里「所属模块/当前版本/测试系统」字段都空,分类靠描述里的【】标签自己归。

### 按模块
| 模块 | 数量 | BugID |
|------|------|-------|
| 消息 chat | 6 | R00228/R00234✅/R00235/R00236/R00237✅/R00241✅ |
| 设置 settings | 5 | R00243✅/R00244✅/R00245⏸/R00246✅/R00247✅ |
| 首页 discovery | 4 | R00223/R00240/R00242/R00248 |
| 我的 me | 3 | R00222/R00232/R00249 |
| 拉黑 block | 1 | R00227 |
| CE | 1 | R00229(P2) |

### 按性质
- **功能/交互逻辑缺陷(6,优先)**:R00222 进度条不满 · R00223 卡片滑完不自动加载 · R00227 拉黑后用户未从消息页消失 · R00242 拉黑后首页卡片未消失 · R00244 设置拉黑未显示数量 · R00248 未订阅最后一张卡片点击无法进详情。
- **UI/设计稿还原(12)**:R00228/R00232/R00234/R00235/R00236/R00237/R00241/R00243/R00245/R00246/**R00247✅**/R00249。
- **文案(2)**:R00229 CE 介绍页文案 ✅已修(PR #347,merge `c7be799b`)· R00240 首页 say hello 文案。

> **进度**:R00243/R00244/R00246 已修复合入 `feature/koda-android`(PR #351,merge `cead46f1`)——设置页 Your character 卡跳已有编辑页/拉黑数量/privacy 版式。R00245(Your story)无后端支撑,决策**暂不做**(维持有意省略)。R00247 已修复合入 `feature/koda-android`(PR #350,merge `b297915f`)——legal-document 重写为 Koda 版式。R00249 核查=前端已达标,单 KPOP 是后端数据。R00229 已修复合入 `feature/koda-android`(2026-09-02)。根因=chemistry-how 绝对定位画布标题两行溢出压正文;修法=外层 canvas 改流式。详见当日 Daily。

**同批**:R00241~R00248 都带 `(ID：2100065646)`,同一测试同批提。
**设计稿**:Figma Koda `https://www.figma.com/design/RwULBso4PNCNqKYZbW6VSY/Koda`。

## Bug→代码定位(sitin-rn/apps/koda/src)
详见 [sitin-rn 结构](../30-Resources/sitin-rn-project-structure.md)。
- 消息 → `app/(tabs)/chat.tsx`、`app/chat/[id].tsx`、`components/chat`、`services/chat-*`、`stores/chat.ts`
- 设置/拉黑 → `app/settings/*`(`blocklist.tsx`)、`components/settings`、`services/settings.ts`
- 首页 → `app/(tabs)/index.tsx`、`components/discovery`、`services/discovery*`、`stores/discovery.ts`
- 我的 → `app/(tabs)/me.tsx`、`app/edit-profile/*`、`components/edit-profile`
- CE → `app/contact/*`、`packages/business-contact-exchange`、`services/contact-exchange.ts`

## 建议开工顺序
B 类逻辑缺陷 → A 类缺页面硬伤(R00243/R00245/R00232)→ 纯 UI 对齐与文案。

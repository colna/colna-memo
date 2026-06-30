---
title: PWA 女用户钱包 / 计费业务知识(Video 余额拆分定位沉淀)
date: 2026-06-30
tags: [presence-io, pwa, billing, wallet, call-order, video, business-knowledge, reference]
---

# PWA 女用户钱包 / 计费业务知识

本篇沉淀本次"拆分 PWA 散户女 Video 余额(视频通话 vs 非视频通话)"定位单的全部业务结论。配套查询手册:[DMS](dms-query-manual.md) · [Grafana/Loki](grafana-loki-query-manual.md)。

---

## 1. 钱包结构:三种 balance_type

`user_balance` 一个用户每种类型一行,`balance_type`:
- **`video`** — 视频相关钱包(本次重点)。**注意:它不只装视频通话收益**,还混入转盘 SPIN、等待奖励等。
- `ai_persona` — AI 消息金。
- `referral_bonus` — 推荐奖励。

金额是 varchar,取数用 `NULLIF(balance,'')::numeric`。

---

## 2. 核心结论:Video 余额 ≠ 视频通话收益

`video` 钱包的钱来源很杂。看一个真实用户(6167493)流水 `change_type` 构成:
```
SPIN_WIN      +3255      ← 转盘赢
VIDEO_CALL    +399.40    ← 真·视频通话
WAITING_REWARD+241.23    ← 等待奖励
MOCK_VIDEO    +123.90    ← 模拟视频(也算通话收益)
SPIN_COST     -2494.80   ← 转盘花费
...
```
所以**不能拿 Video 余额当"接客赚的钱"**。要拆成:
- **视频通话余额** = 真·通话(含 mock)赚来、还留在钱包里的部分
- **非视频通话余额** = 转盘 / 等待 / 奖励等撑起来的部分

---

## 3. 拆分模型(权威口径)

```
视频通话余额 = min( max(0, 视频通话累计收益 − 视频类已提现), 当前 Video 余额 )
非视频通话余额 = Video 余额 − 视频通话余额
```
隐含假设:**视频类提现优先消耗视频通话赚的钱**(因为线上 $50 提现任务的 gate 就是判定在视频通话收益上)。两端再用 `max(0,…)` 和 `min(…, Video余额)` 夹住,保证落在 `[0, Video余额]`。

---

## 4. ⚠️ 最关键的坑:视频通话收益用 call_order 口径,不是余额流水口径

这是本次定位单的**根因**(用户 7334158 / Rita 报错 $216.2 的来源)。

### ✅ 权威:call_order 口径(线上 $50 任务 gate 用的 `getUserCallOrderTotalEarn`)
```sql
SELECT female_user_id, ROUND(SUM(NULLIF(amount,'')::numeric),2)
FROM user_call_order
WHERE female_user_id IN (...)
  AND status IN ('INIT','PAID')
  AND NULLIF(amount,'') IS NOT NULL
  AND order_type <> 'REFERRAL_BONUS'
GROUP BY female_user_id;
```
- `INIT` 单无 amount,自动被 `NULLIF` 滤掉 → 实际等于 **VIDEO_CALL + MOCK_VIDEO 的 PAID 金额**。
- **MOCK_VIDEO 计入**。
- 代码真源:`CallOrderService.getUserCallOrderTotalEarn`(`listAllInitAndPaidCallOrder`)。

### ❌ 错误:余额流水口径
```sql
-- 不要用这个算视频通话收益:
SELECT to_user_id, SUM(NULLIF(balance_change,'')::numeric)
FROM pwa_user_balance_change_history
WHERE change_type IN ('PWA_USER_BALANCE_CHANGE_TYPE_VIDEO_CALL',
                      'PWA_USER_BALANCE_CHANGE_TYPE_MOCK_VIDEO');
```
流水里 `VIDEO_CALL` type 把**非通话订单产生的入账也混进去**,系统性**高估**:

| 用户 | 流水口径(错) | call_order 口径(对) | 差额 |
|---|--:|--:|--:|
| Rita 7334158 | $313.20 | **$289.20** | +$24 |
| 6167493 | $523.30 | **$504.80** | +$18.5 |

**Rita 修正**:视频通话余额 `min(max(0, 289.20−97), 348.53) = $192.20`(原算成 $216.20 ❌)。

---

## 5. 视频类已提现

```sql
SELECT SUM(amount) FROM user_withdraw_task
WHERE user_id=? AND earning_type='video' AND status='FINISHED';
```
- **只算 `FINISHED`**(真出账)。
- `FAILED` 已退回钱包,不扣;`APPROVED`/frozen 待处理,暂不扣。
- ⚠️ 提现**不记在** `pwa_user_balance_change_history` 流水里,必须查 `user_withdraw_task`。

---

## 6. 人群过滤口径(干净活跃散户女)

```sql
u.gender=2
AND o.updated_at >= '<N个月前>'   -- 活跃窗口(o=user_online_status_record)
AND u.regulation_status = 0       -- 未封禁(2/6=ban)
AND u.deleted_at IS NULL          -- 未软删除(与封禁独立!7156335 status=0 但已删)
AND u.cai_user_type <> 3          -- 非数字人(3=DigitalHuman)
AND NOT EXISTS (SELECT 1 FROM userinfo_pwa_club_mapping m WHERE m.user_id=o.user_id)  -- 散户(不在公会)
```
经验:数字人(cai_user_type=3)在活跃集里天然为 0 —— 它们在 `user_online_status_record` **没有行**,被活跃 JOIN 自动排除。

---

## 7. 最终量化结论(2026-06-30 跑出)

三个活跃窗口的散户女 Video 钱包拆分(修正后,飞书公式合计):

| 活跃窗口 | 人数 | Video 余额 | 视频通话 | 非视频通话 | 视频通话占比 |
|---|--:|--:|--:|--:|--:|
| 3 月内 | 27,091 | $199,270.93 | $66,897.56 | $132,373.37 | **33.6%** |
| 2 月内 | 16,189 | $154,221.25 | $54,004.77 | $100,216.48 | **35.0%** |
| 1 月内 | 10,013 | $113,693.44 | $41,826.43 | $71,867.01 | **36.8%** |

> 修正前(错误的流水口径)3 月占比是 ~37.5%,修正后降到 33.6%。

**业务洞察**:散户活跃女的 Video 钱包里,**真实接客(视频通话)赚的钱只占约 1/3**,大头(约 2/3)是**转盘 SPIN + 等待奖励**撑起来的虚高。多数活跃女把真实接客赚的钱基本提光了,留在钱包里的视频通话余额很少。

交付物:飞书表格 `https://presence.feishu.cn/sheets/ViBdss3GkhOsiGtXUF6cVbvrn1f`(owner 步川),3 个 sheet 对应 3 个活跃窗口,合计行用 `=SUM()` 公式而非硬写值。

---

## 8. 一句话记忆

> 拆 PWA 女 Video 余额时,**视频通话收益永远用 `user_call_order`(call_order 口径,含 mock),不用余额流水**;提现查 `user_withdraw_task` 的 `FINISHED`;人群剔除 ban / 已删 / 数字人 / 公会。

---
title: PWA 小美池 周度四指标 SQL & 计算链路
date: 2026-08-25
tags: [minerva, pwa, 权益池, 小美池, sql, dora, 保底]
---

# PWA 小美池 —— 周度四指标 SQL & 计算链路

> 从 dora `release/test` 抽取并核验(行号与源码一一对上)。
> 本周视频收益(剔保底) / 服务质量达标天数 / 本周 Go Live 有效时长 / 预计保底补贴。
> 所有源表均在 MAIN 库 archat(Minerva 可 dms 直读)。窗口区间半开:`created_at > $start AND created_at <= $end`。

---

## 1. 本周视频收益(剔保底) —— 两个口径

保底流水的 `change_type` 是 `..._GUARANTEE`,下面两条 SQL 都只查 `VIDEO_CALL`(/`MESSAGE_TEXT`),**天然不含保底**。

### (a) 周退池用(`PwaTierWeeklyReviewJob.java:52` `WEEKLY_INCOME_QUERY`)——VIDEO_CALL + MESSAGE_TEXT

```sql
SELECT to_user_id AS user_id, SUM(balance_change::numeric) AS weekly_income_usd
FROM pwa_user_balance_change_history
WHERE to_user_id = ANY($1)
  AND change_type IN ('PWA_USER_BALANCE_CHANGE_TYPE_VIDEO_CALL','PWA_USER_BALANCE_CHANGE_TYPE_MESSAGE_TEXT')
  AND created_at > $2 AND created_at <= $3
GROUP BY to_user_id
```

### (b) 周补差用(`PwaTierVideoMetric.java:54` `INCOME_QUERY`)——仅 VIDEO_CALL

```sql
SELECT to_user_id AS user_id,
       SUM(balance_change::numeric) AS video_income_usd,
       COUNT(*) AS settled_call_count
FROM pwa_user_balance_change_history
WHERE to_user_id = ANY($1)
  AND change_type = 'PWA_USER_BALANCE_CHANGE_TYPE_VIDEO_CALL'
  AND created_at > $2 AND created_at <= $3
GROUP BY to_user_id
```

> 差异:退池判定要把文字收益也算进「自然收益」,保底补差只按视频收益补。**两口径别用错。**
> 数据源都是 `pwa_user_balance_change_history`(MAIN),金额含 +50% 加成(post-uplift)。

---

## 2. 服务质量达标天数(`PwaTierWeeklyReviewJob.java:62` `QUALITY_DAYS_QUERY`)

数当周日 job 落库里 `quality_passed=TRUE` 的天数:

```sql
SELECT user_id, COUNT(*)::bigint AS quality_days
FROM pwa_tier_guarantee_daily
WHERE user_id = ANY($1)
  AND local_date >= $2 AND local_date <= $3
  AND finalized = TRUE AND quality_passed = TRUE
GROUP BY user_id
```

> 源表 `pwa_tier_guarantee_daily`(MAIN)。`quality_passed` 由日 job 的 `dailyQualityPass`
> (视频拒接率 / 短通话率)算好写入,这里只做天数聚合。
> ⚠️ 短通话率子项是 <1s 退化口径(真·<15s 埋点缺),达标天数会偏松。

---

## 3. 本周 Go Live 有效时长(`PwaTierGuaranteeService.java:16` `GOLIVE_QUERY`)

裁剪到 **每天 09:00–24:00 ET** 窗口后求和,按 `(user, ET日)` 分组;周补差里再 `merge` 成整周。

```sql
SELECT creator_id::bigint AS user_id,
       (connected_at AT TIME ZONE 'America/New_York')::date AS local_date,
       (SUM(
          GREATEST(0, EXTRACT(EPOCH FROM (
            LEAST(  -- 会话结束 vs 当日 24:00,取较早
              connected_at + (duration_ms * interval '1 millisecond'),
              (((connected_at AT TIME ZONE 'America/New_York')::date + 1)::timestamp)
                AT TIME ZONE 'America/New_York')
            - GREATEST(  -- 会话开始 vs 当日 09:00,取较晚
              connected_at,
              (((connected_at AT TIME ZONE 'America/New_York')::date::timestamp + interval '9 hour'))
                AT TIME ZONE 'America/New_York')
          )) * 1000)
       ) / 60000)::bigint AS valid_golive_minutes
FROM sp_v3_online_session
WHERE creator_id ~ '^[0-9]+$'
  AND creator_id::bigint = ANY($1)
  AND platform = 'IG'
  AND connected_at > $2 AND connected_at <= $3
  AND duration_ms >= 60000 AND duration_ms < 86400000
GROUP BY creator_id::bigint, (connected_at AT TIME ZONE 'America/New_York')::date
```

> 源表 `sp_v3_online_session`(MAIN)。裁剪逻辑:每段会话有效区间
> = `[max(开始, 当日9点), min(结束, 当日24点)]`,负值归 0;只认 `platform='IG'`、单段 1 分钟~24 小时。
> 单用户实时版见 `GOLIVE_QUERY_SINGLE`(`creator_id = $1` 走索引)。

---

## 4. 预计保底补贴 —— 无单独 SQL,纯函数计算

补贴额不是一条 SQL 查出来的,而是 `PwaTierGuaranteeWeeklyJob.reconcileWeek`
用**几条取数 SQL + 纯函数**算出来的。

### 计算链路(`PwaTierGuaranteeWeeklyJob.java:259-269`)

```
weeklyGap = weeklyGuaranteeGapMicros(周Go Live分钟, 周视频收益, 周门槛600min, 周保底floor $500)
              → 达周在线门槛则补足到 floor,否则 0
target    = weeklyPayoutMicros(日gap之和, weeklyGap, cap $500)
              → min(max(日gap之和, weeklyGap), cap)
topUp     = weeklyTopUpMicros(target, 本周日channel已发)
              → max(0, target - alreadyPaid)   ← 本次实发补贴
```

纯函数定义见 `PwaTierGuaranteeService.java`:
- `weeklyGuaranteeGapMicros`(:92):`周分钟 < 门槛 ? 0 : max(0, weeklyFloor − 周收益)`
- `weeklyPayoutMicros`(:108):`min(max(dailyGapSum, weeklyGap), cap)`
- `weeklyTopUpMicros`(:135):`max(0, target − alreadyPaid)`

### 依赖的取数 SQL

#### (a) 本周日 gap 之和(`PwaTierGuaranteeWeeklyJob.java:68` `DAILY_GAP_SUM_QUERY`)

```sql
SELECT user_id, SUM(guarantee_gap_micros)::bigint AS gap_sum
FROM pwa_tier_guarantee_daily
WHERE user_id = ANY($1) AND local_date >= $2 AND local_date <= $3 AND finalized = TRUE
GROUP BY user_id
```

#### (b) 本周经日 channel 已发保底(`PwaTierGuaranteeWeeklyJob.java:79` `WEEK_PAID_QUERY`)

```sql
SELECT to_user_id AS user_id, COALESCE(SUM(balance_change::numeric), 0) AS paid_usd
FROM pwa_user_balance_change_history
WHERE to_user_id = ANY($1)
  AND change_type = 'PWA_USER_BALANCE_CHANGE_TYPE_GUARANTEE'
  AND ext_id LIKE 'guarantee-daily:%'
  AND (split_part(ext_id, ':', 3))::date >= $2
  AND (split_part(ext_id, ':', 3))::date <= $3
GROUP BY to_user_id
```

### 单日预计补贴(`dailyGuaranteeGapMicros`,`PwaTierGuaranteeService.java:73`)

```
达在线门槛 ? max(0, 工作日/周末floor − 当日视频收益) : 0
```

落进 `pwa_tier_guarantee_daily.guarantee_gap_micros`(日 job 写入)。

---

## 小结

| 指标 | SQL 位置 | 源表(库) |
|---|---|---|
| 本周视频收益(剔保底) | 退池 `WeeklyReviewJob:52` / 补差 `VideoMetric:54` | `pwa_user_balance_change_history`(MAIN) |
| 服务质量达标天数 | `WeeklyReviewJob:62` | `pwa_tier_guarantee_daily`(MAIN) |
| 本周 Go Live 有效时长 | `GuaranteeService:16` | `sp_v3_online_session`(MAIN) |
| 预计保底补贴 | 无单独 SQL;`GuaranteeService` 纯函数 + `WeeklyJob:68/79` 两条取数 | 计算值 |

> **强前提**:服务质量达标天数 / 日 gap / 预计补贴均依赖 `pwa_tier_guarantee_daily` 被 dora 日 job(每日 02:13 ET)写入;dora 若未在对应环境部署 schedule job,该表为空,后台仍显 `—`。接线前先 dms 查这张表有无数据。

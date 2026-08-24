---
title: PWA 权益池(分级 tier)后台
date: 2026-08-21
tags: [minerva, pwa, tier, 权益池, 后台]
---

# PWA 权益池(分级 tier)后台

运营对女用户分级(新人考核池 / 活跃候选池 / 小美池)+ 权益 + 保底的 Minerva 后台。

## 关键坐标
- 分支 `personal/zz/admin-pwa-benefit-pool` → **PR #966**(base `feature/admin`)。
- 需求 wiki `AsNGwQa7`;前端 RFC/口径 wiki `Q1Dswuf8`(docx `HEPjdg9B4oPHWlxmRbQcfRHNndq`,末尾有「后台开发阻塞」节);指标计算 wiki `KjeTwEqc`(晁岳攀);UI `design.html`(3 截图)。
- **架构**:真源在 dora;Minerva 只读 dora 的 archat 库(视图 + 直读表)+ 前端 + **写操作 Minerva 自实现事务(不依赖 dora 契约)**。
- 直查 dev:`admin-api-dev.sitin.ai` `POST /api/aliyun-dms/run` + Bearer(aliyun-dms token,过期需重取)。库=archat。
- **V20260820 精简 schema**(已上 dev):留 `pwa_tier_membership / invitation / audit_event / guarantee_daily` + 2 视图 `pwa_tier_member_list_v / _detail_v`;删了 config_snapshot / run / candidate / settlement。

## 已完成(PR #966)
- 后端包 `business-minerva-pwa-pool`:读(2 视图 + userinfo join 昵称/头像/国家;小美今日直读 guarantee_daily;视频等级读 Redis `pwa:video:level:score`);写(kick/move→小美/import→小美/invite,事务、部分成功、审计,move/import 直接入池)。SQL 已对 dev 真数据验证。
- controller `app-minerva-server/src/pwa-pool/` 挂 app.ts;RBAC 6 条(读 app.read/写 app.write);冒烟测试。
- 前端 `app-minerva-web/src/pages/PwaPool/`:3 Tab + 表格 + 详情抽屉(已对齐真实响应)+ 导入弹窗;接真接口(USE_MOCK=false)。
- 枚举(dev 实测):pool_type {NEWCOMER_ASSESS/ACTIVE_CANDIDATE/XIAOMEI};status ACTIVE/EXITED;invitation PENDING/ACCEPTED/REJECTED/EXPIRED/CANCELLED;source SYSTEM_INVITATION(后台写入用 MANUAL_MOVE/MANUAL_IMPORT)。

## 未完成 / 待办
- **部署**:PR 合 `feature/admin` + 部署 `admin-api-dev`,前端才能真 e2e(SQL 已验证,dev 有 5 条 membership 测试数据)。
- **RBAC 细化**:目前复用 app.read/app.write,未建独立「运营池」角色/权限点。
- **次级页**:跑批监控 / 结算看板 —— 精简 schema 删了 run/settlement,基本作废(如需另议)。
- **本周考核收益(剔保底)**:小美留池判据列,窗口口径 dora 未定(见阻塞),暂缓、未实现。
- **前端**:活跃/新人 Tab 的指标列、命中分支列目前占位(依赖下方硬阻塞)。

## 阻塞(需 dora / 数据 / 运维)
1. **`female_user_features` 不在 archat 库** → 被右滑率 / 有效曝光 / Opener 回复率 取不到(可能在特征/分析库,需 dora/运维确认或暴露)。
2. **`pwa_daily_data` 不在 archat 库** → Go Live 有效在线原始源(小美 guarantee_daily 已有算好值可用;若要明细则卡这张)。
3. **命中分支 / 资格状态不落库**:candidate 表已删,dora 代码实时算 → 后台要展示需 dora 用视图/接口暴露,或确认不展示。
4. **本周考核收益(剔保底)窗口口径**:dora 未定义(周窗口起止 / 剔哪些);ledger `pwa_user_balance_change_history` 有数据 + change_type 分类已知,但窗口没定不敢硬编。
5. **权益(Nacos benefit snapshot)**:Minerva 不直连 Nacos;权益值多为固定(Level5/+50%),如需精确展示要 dora 给取数。
6. 🟢 小风险:`userinfo.user_id` 是 int32,超大 uid 理论溢出(dora 表)。

## 已定口径(供接线)
- 视频有效时薪 = 真实视频收益 × 3600 ÷ online_duration(秒)(= databoard「时薪不含Mock」)。
- 颜值分 = `userinfo.face_score`(0~100,≥70)。
- 视频收益/文字收益:ledger `pwa_user_balance_change_history`,文字类 change_type = DATING_APP/IG_HOSTING/INS_FOLLOW_REWARD/INS_MESSAGE_TEXT/MESSAGE_TEXT/PWA4,其余(含 NULL)=视频;`to_user_id NOT IN ('-1','0')`。
- 保底:Go Live≥180min 且当日含加成视频≥$80 → 补到 $100;周退池 ≤$100(均 Nacos 可调)。
- 阈值占位(数据侧回填):新人有效通话数 X、Opener 阈值 B、文字上浮 Y1、小美上浮 Y2。

## 卡点(2026-08-21,新人+活跃两池已做完 commit 3c0da1796 后剩余)

分支 `personal/zz/admin-pwa-benefit-pool`。三库:`getPgDb()`=archat、`getNamedPgDb("monitor")`=pwa_daily_data、`getNamedPgDb("strategy")`=strategy_feature(女性特征 features JSONB)。

### A. 缺数据源/埋点 —— 算不了(等后端/埋点)
1. **视频主动发起率(D)**:无数据源、无阈值。列/抽屉已占位显 `—`、标准 `≥ D%`。
2. **视频拒接率 / 短通话率**(小美今日服务质量):分子依赖新增「挂断方」埋点(女方主动挂断 & <15s / 女方拒绝或超时未接),埋点未上 → 算不出。文档明说「配置项就位≠指标可算」。
3. **小美周字段**(本周视频收益删保底 / 服务质量达标天数 / 本周 Go Live 有效时长 / 预计最终保底补贴):需 monitor `pwa_daily_data` 周聚合(仅统计每日 09:00–24:00、周一~周日窗)+ guarantee rollup。→ **小美池列表列 + 详情抽屉(每周考核/今日服务质量/保底执行计算)整体卡在这里**。

### B. 阈值 —— 已按「nacos 为准」定(用户 2026-08-24 拍板)
- B(opener 阈值)、D(视频发起率阈值)未配 → 按 `*_enabled` 视为**不参与判定**;活跃命中分支近似(只用右滑+划卡+收益),抽屉标准列占位。
- 小美(`pwa_tier_benefit_config` benefits.xiaomei):`daily_required_golive_minutes`=**90**、`weekly_quality_days_min`=**0**、`daily_video_reject_rate_max`/`daily_short_call_rate_max`=**0.5**、`short_call_duration_threshold_seconds`=**15**、周日保底$80/周末$120、周 Go Live≥600 补到$500、`weekly_auto_exit_threshold_micros`=$100(语义反转为「周收益≥$100 达标」之一)、`auto_exit_consecutive_weeks`=**2**(连续2周未达标才退)。**冲突以 nacos 为准**(文档正文说 golive 120/quality_days 3 作废)。
- 新人/活跃筛选阈值(《筛选指标汇总》定稿固定值,已写死):颜值≥70、新人右滑≥20%/活跃≥15%、曝光·划卡≥20、文字周收益≥$6、视频周收益≥$50、新人平均通话≥80s&通话数≥5(剔<5s/>10000s)。

### C. 需部署 dev 后运行验证(本地测不了)
1. **strategy SQL** 按「指标获取」文档口径写(`female_daily_features` features->'daily'->>'right_swipe_count'/'swipe_count';`female_user_features` features#>>'{breakthrough,count_total/replied_total}'),dms 只连 archat → 部署后实测表名/字段/`user_id` 类型(strategy 库 user_id 类型未知,service 已 Number 化 join)。
2. **`STRATEGY_DATABASE_URL` 变量名**:我按 `<PREFIX>_DATABASE_URL` 约定(运维给 envPrefix `STRATEGY_`),需与 k8s 实际注入名对齐,不一致 strategy 连接 warn 降级、被右滑率/曝光/opener 列显 `-`。
3. **生效前提**:必须部署 minerva-server 到 admin-api-dev(strategy 连接 + business 指标都在后端)。

### D. 前端占位待后端补的字段
- `invitationStatus`/`invitedAt`:已补查 `pwa_tier_invitation`(list 视图未含);
- `hitBranch`:minerva 按阈值现算(dora 未把命中分支落 list 视图),活跃因 B/D 未配为近似。

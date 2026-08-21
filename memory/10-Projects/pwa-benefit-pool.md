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

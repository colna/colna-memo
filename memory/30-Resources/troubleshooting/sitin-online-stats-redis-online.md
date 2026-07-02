---
title: sitin online-stats —— 实时在线源 & 统计口径坑
date: 2026-06-29
tags: [sitin-next, social-proxy, online-stats, redis, troubleshooting]
---

# sitin online-stats —— 实时在线源 & 统计口径坑

`/social-proxy/online-stats` 页面(minerva-web)+ 后端 `app-minerva-server/.../routes/sp-online/index.ts`。

## 实时在线源:Redis(跨服务共享)

- minerva-server 与 social-proxy-server **本应共享同一 Redis 实例**(同 host:port/密码;minerva 读走 `REDIS_URL` **只读从库**,写走 `REDIS_WRITE_URL`;social-proxy 用离散 `REDIS_HOST`/`REDIS_PORT`/`REDIS_DB`,见 `redis.module.ts`)。
- **当前在线 = Redis key `proxy:ig_online:{creatorId}`**,value `{ instance: string, connectedAt: number }`,**TTL 120s**,设备**连接时写、断开即删**(social-proxy `device-registry.service.ts`)。
- minerva 读法:`getRedisRead().scan(cursor,"MATCH","proxy:ig_online:*","COUNT",N)` + `mget`。已有先例 `middlewares/auth.ts invalidatePermissionCache`。
- 封装在 `GET /api/sp-online/current`(2026-06-29 新增)。
- ⚠️ **2026-07-01 起前端「当前在线」卡不再走此接口**,改读 social-proxy 网关内存表(见下「prod 恒为 0」节)。`/api/sp-online/current` 接口保留但暂无人调。

## prod「当前在线 0 人 / 最长在线 0s」根因 + 修法(2026-07-01)

- **现象**:`admin-prod` 在线数据页,顶部当前在线/最长在线恒 0,下方历史图表(Postgres)正常。
- **根因**:上面「共享同一 Redis」的前提在 **prod 没成立**。minerva 读的是 `REDIS_URL` **只读从库**,social-proxy 写的是主库;prod 上从库没同步到 `proxy:ig_online:*`(主从复制断 / 从库端点指错 / db 不一致三者之一)→ minerva SCAN 恒空 → 0。**dev 正常** = dev 两边落在同一个 Redis/从库能看到写入。
- **定性自测(无需集群权限)**:Action 控制台设备**列表**读 social-proxy 内存 Map(不碰 Redis)、在线**徽标**读同一个 `proxy:ig_online:` key。「列表有设备,但 minerva `/current`=0」即坐实跨服务 Redis 未打通。
- **验证盲区**:minerva/social-proxy 两服务在 Loki 于当日 06:28 起无日志(pod 重建后日志采集断,最后几行 `unable to retrieve container logs`),没法用日志实时验证,靠历史图表能加载反推 minerva 在跑。
- **采用的修法(产品级,绕开 infra)**:`app-minerva-web/.../SocialProxy/OnlineStats/index.tsx` `CurrentOnlineCard` 数据源从 `getSpOnlineCurrent`(Redis)换成 `getDevices()`(`../api` → `GET /api/social-proxy/gateway/devices`,social-proxy 内存设备表,与 Action 控制台同源)。当前在线=`count`,最长在线=遍历 `DeviceInfo.connectedAt` 取最早算 `(now-connectedAt)/1000`。
- **取舍**:网关设备表是 social-proxy **单实例内存**,原 Redis 方案是为**多副本聚合**设计;当前 prod 单实例、Redis 那条路本就坏着返回 0,换过来更准。**若 social-proxy 横向扩多副本,此数会只统计单副本** → 届时要么修 Redis 主从/对齐、要么网关侧聚合。
- **infra 侧根治(备选,未做)**:对齐 prod minerva `REDIS_URL`(从库)与 social-proxy 写库为同实例同 db、且主从复制正常。改 k8s deployment env,非 repo。

## 历史表只含已闭合会话

- `sp_v3_online_session`(creator_id/platform/connected_at/disconnected_at/duration_ms)**只在断开时 insert**,`duration_ms<=0` 直接丢弃。
- ⇒ 历史口径(summary/hourly/timeseries/users)**永远不含进行中、未断开的会话**。"当前在线的人"只能靠上面的 Redis 实时快照补,二者口径互补,别想用历史表查实时。

## 统计口径已修正的坑(2026-06-29)

1. **分时段/峰值**原是跨天累计去重,区间越长越大、被误读成并发峰值 → 改 **日均**(每天该时段 distinct ÷ 区间天数)。
2. **总/中位/人均时长**原 naive `SUM(裁剪时长)`,多设备并发重复计时 → 改 **gaps-and-islands 区间并集去重**(`MAX(end) OVER ... ROWS UNBOUNDED PRECEDING AND 1 PRECEDING` 判新岛 → 合并)。
3. **users 导出** join `ins_user_profile ON user_name=ins_id` 假设 1:1 未核验,可能扇出 → 改 `LEFT JOIN LATERAL (... LIMIT 1)`。
4. **5 万导出上限**静默截断 → 返回 `total`/`truncated`,前端 `message.warning`。

## 接口包装层(易混)

- `/api/sp-online/*` 是 **minerva 自有路由**,`success(ctx,payload)` **单层**包装,前端 `request` 直接解包 `data`。
- 与经 social-proxy 反代的 V3 接口(`/api/social-proxy/*`)的**双层 success**不同——那种要判内层。见 [[sitin-v3-signal-types]]。

## 本地 tsc 注意

- minerva-server 本地 tsc 前需 `prisma generate`(主 `prisma/schema.prisma` + `prisma/monitor/schema.prisma` 两个),否则 `prisma.$queryRaw<T>` 退化成 any、`rows.map((r))` 报 implicit-any。这是仓库既有状态,非新引入。

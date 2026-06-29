---
title: sitin online-stats —— 实时在线源 & 统计口径坑
date: 2026-06-29
tags: [sitin-next, social-proxy, online-stats, redis, troubleshooting]
---

# sitin online-stats —— 实时在线源 & 统计口径坑

`/social-proxy/online-stats` 页面(minerva-web)+ 后端 `app-minerva-server/.../routes/sp-online/index.ts`。

## 实时在线源:Redis(跨服务共享)

- minerva-server 与 social-proxy-server **共享同一 Redis 实例**(同 host:port/密码;minerva 读走 `REDIS_URL` 只读从库,写走 `REDIS_WRITE_URL`)。
- **当前在线 = Redis key `proxy:ig_online:{creatorId}`**,value `{ instance: string, connectedAt: number }`,**TTL 120s**,设备**连接时写、断开即删**(social-proxy `device-registry.service.ts`)。
- minerva 读法:`getRedisRead().scan(cursor,"MATCH","proxy:ig_online:*","COUNT",N)` + `mget`。已有先例 `middlewares/auth.ts invalidatePermissionCache`。
- 封装在 `GET /api/sp-online/current`(2026-06-29 新增)。

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

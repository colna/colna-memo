---
title: Grafana / Loki 线上日志查询操作手册(给 Claude / MetaBot)
date: 2026-06-30
tags: [grafana, loki, logql, presence-io, sitin, logs, manual, reference]
---

# Grafana / Loki 线上日志查询操作手册

给 Claude / MetaBot 用的 presence-io / sitin 基础设施日志脚本化查询手册。官方指南:飞书 wiki `https://presence.feishu.cn/wiki/YZfZwQPNIiZi1EkmCw8c3LKvnxb`。

---

## 1. 接入

| 项 | 值 |
|---|---|
| URL | `https://grafana-k8s.sitin.ai` |
| 账号 / 密码 | `read` / `^mTh}Nws65MGa-p*GGpg`(只读 Viewer) |
| 登录方式 | **只能走 Grafana 原生登录拿 session cookie** |

> ⚠️ **Basic auth 和 Bearer token 都会 401**。必须先登录拿 cookie。

```bash
# 1. 登录拿 cookie
curl -s -c /tmp/graf.cookie -H 'Content-Type: application/json' \
  -X POST https://grafana-k8s.sitin.ai/login \
  -d '{"user":"read","password":"^mTh}Nws65MGa-p*GGpg"}'
```

---

## 2. 数据源(datasource)

| 名称 | uid | 用途 |
|---|---|---|
| ✅ **`Loki-GKE`** | `afk7n19up9kaob` | 指向 GKE Loki,**日常用这个** |
| ❌ `Loki`(旧) | `cfino1bi6s0lca` | 指向退役 VM `10.142.0.62:3101`,connection refused / 502,仅历史 |

---

## 3. 查询接口

用 **`POST /api/ds/query`**,带 cookie + header `X-Grafana-Org-Id: 1`。
> ⚠️ **不要**用 `/api/datasources/uid/<uid>/resources/...` 代理路径 → 会 `plugin.requestFailureError`。

```bash
NOW=$(($(date +%s)*1000)); FROM=$((NOW-1800000))   # 近 30 分钟,毫秒
curl -s -b /tmp/graf.cookie -H 'Content-Type: application/json' \
  -H 'X-Grafana-Org-Id: 1' \
  -X POST https://grafana-k8s.sitin.ai/api/ds/query \
  -d '{"queries":[{"refId":"A",
        "datasource":{"type":"loki","uid":"afk7n19up9kaob"},
        "expr":"{app=\"user-service\", env=\"prod\"} |= \"finishCallOrder\"",
        "queryType":"range","maxLines":200,"direction":"backward"}],
       "from":"'$FROM'","to":"'$NOW'"}'
```
- `from` / `to`:**毫秒字符串**(Unix epoch × 1000)。
- `direction":"backward"`:从新到旧。
- `maxLines`:本次返回行数上限。

---

## 4. 关键 label(LogQL 选择器)

| label | 说明 |
|---|---|
| `app` | **主区分维度**:`user-service`、`gateway-service`、`llm-schedule-api`、`messaging-service`、`aichat-v2` 等 |
| `env` | `dev` / `prod` |
| `namespace` | `dora-prod-k8s` / `dora-dev-k8s`(**但部分服务如 user-service 只有 `env` 标签、没有 namespace** → 锁 prod 用 `{app="user-service", env="prod"}`) |
| `host` | 仅 haven VM 有 |
| `pod` | 高基数,慎用 |

### ⚠️ 仓库名 ≠ 部署名(高频踩坑)
- 仓库 `dora-service` → 线上部署是 **`user-service`**。Loki 里**没有** `app="dora-service"`。
- call order(`CallOrderService`)日志在 `app="user-service"`。

---

## 5. LogQL 性能要点

- 过滤优先 **`|=` 字面匹配**(比 `|~` 正则快)。
- 时间窗 **<1h**,并加精确 label,减少 chunk 扫描。
- 日志行是 JSON:`{"severity","message","timestamp","logger","thread"}`,可接 `| json` 解析后按字段过滤。

```logql
{app="user-service", env="prod"} |= "callOrderId" |= "MALE_CANCEL_TIMEOUT_5S"
{app="user-service", env="prod"} | json | severity="ERROR"
```

---

## 6. 已验证业务经验(call order)

- `CallOrderService.getUserCallOrderTotalEarn` 是**视频通话收益**的权威实现(见 [PWA 女钱包计费业务知识](pwa-female-wallet-billing.md))。
- `finishCallOrder` 的 `reasonType`(`MALE_CANCEL_TIMEOUT_5S` / `WITHIN_5S` 等)由**男方消费端 App**(`luma`/`com.odyssey.luma`、`romi` 等)随请求上报,服务端原样落库 `reason_type`;女方端(`haven_pwa`)不传。
- `MALE_CANCEL_TIMEOUT_5S` = 男方呼叫、女方未接、振铃 >5s 后男方取消;callDuration=0、不计费。

---

## 7. 排错速记

| 现象 | 原因 / 修法 |
|---|---|
| 401 | 用了 Basic/Bearer → 必须 cookie 登录 |
| `plugin.requestFailureError` | 用了 `/api/datasources/.../resources` 代理 → 改 `/api/ds/query` |
| connection refused / 502 | 用了旧 `Loki`(VM)→ 换 `Loki-GKE` uid `afk7n19up9kaob` |
| 查不到 `app="dora-service"` | 部署名是 `user-service`,不是仓库名 |
| user-service 锁不住 prod | 它无 namespace 标签 → 用 `{app="user-service", env="prod"}` |
| 查询慢 / 超时 | 时间窗收到 <1h,`|~` 改 `|=`,加精确 label |

相关:[DMS 查询手册](dms-query-manual.md) · [PWA 女钱包计费业务知识](pwa-female-wallet-billing.md)

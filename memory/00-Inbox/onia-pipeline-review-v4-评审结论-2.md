---
title: "onia-pipeline-review-v4 评审结论"
date: 2026-11-12
tags: review,code-review,onia,instagram-pipeline,server,dart
---

# onia-pipeline-review-v4 评审结论（2026-11-12）

评审对象：`/Users/colna/WORK/_review/onia-pipeline-review-v4`（真实 Instagram 管线：client + server + crates + scripts）
评审依据：目录内代码与 `git/tracked.diff`（1222 行）逐项核对，只读模式。
方式：仅基于本目录文件（硬性限制：不运行 flutter/cargo/pnpm/git 写操作）。

## 总判断：有条件可合
- **客户端（client/）+ scripts 可合**。
- **服务端带出不完整**：`server/lib.rs` 声明 6 个 pub mod，但其中 4 个模块无对应源码文件（`db`、`state`、`auth`、`config`、`error`、`idempotency` 中多个缺失），无法机器验收服务端。见 M1/M2。

## 必须修（M1–M8）
| # | 问题 | 位置 | 要点 |
|---|------|------|------|
| M1 | server/lib.rs 模块树断裂 | `server/lib.rs` | 声明 6 个 pub mod 无源码；`http`/`ws` 与实际文件名 `http_devices.rs`/`ws_mod.rs` 不匹配且无 `#[path]`，编译必失败 |
| M2 | crates 缺失 | `crates/` | 仅 `onia-device` 存在，`onia-protocol`（信封真源）缺失 |
| M3 | 失败结果被缓存 | `client/lib/runtime/device_runtime.dart` | 失败结果也进缓存，应只缓存成功结果 |
| M4 | `schedule_user` 吞包 | `client/lib/state/session.dart` | 两处吞包路径，需核对逻辑 |
| M5 | `ensureDeviceId()` 不持久化 | client | 每次冷启动可能生成新 device_id，违反设备稳定标识预期 |
| M6 | 吞幂等写失败 | `server/http_devices.rs:65/:218` | `let _ = idempotency::store(...)`，幂等写失败静默 |
| M7 | unwrap 直接解包 | `server/http_devices.rs` bind_account | `env.idempotency_key.as_deref().unwrap()`，缺键即 panic |
| M8 | 错误文案中文硬编码 | server | 「账号未绑在该设备」等，应走错误码/常量 |

## 建议（S1–S14）
1. 幂等分支返回 `PolicyView::default()` 与正常分支不一致。
2. promote/parse 软失败路径需明确。
3. 事务 `rows_affected` 未检查。
4. `cancel_todo unwrap_or(Pending)` 语义含糊。
5. redispatch 仅 hello 触发，可能漏派。
6. hello validate 失败无 error envelope。
7. CORS `mirror_request`+`credentials` 组合需收紧。
8. 重绑硬/软失败路径不一致。
9. `register_screen` 地址字段位置。
10. `index.html` 为演示壳，勿当真实产物。
11. `tsconfig` include 范围。
12. 测试覆盖缺口（scripts 侧）。
13. `src/errors.ts` 双别名冗余。
14. WebView 挂树需真机验证。

## 已核对无问题
- UA 去掉 `; wv`、`Version/4.0`；hello 空 capabilities 防护；social_proxy.js 空 if；demo/live 分支；HostContext 回退链；幂等键前缀规则（`device-bind:`/`account-bind:`/`behavior-result:`/`dev-device-account:`）；信封仅 `v=1`；risk/session 无自动重试；五项不回退项；`redispatch_open_behaviors` 与 tracked.diff 893–951 逐行一致。

## 无法核对（源码缺失）
db/state/auth/config/error/idempotency 模块、Hub 实现、`decode_token`、幂等存储、`http::ok/parse_env`、`db::move_account_binding`/`update_binding_handle`、`crates/onia-protocol`、`should_retry`/`apply_result`。

## 契约参考
- HTTP：`POST /v1/auth/register`、`/v1/auth/login`、`/v1/devices/bind`、`/v1/devices/accounts/bind`
- WS：`GET /v1/ws?token=&device_id=`
- 信封：禁止再包 `{ success, data }`，仅 `v=1`

## 跟进方向
① 复核 M1/M2（阻塞：需目录补齐缺失 server 模块与 onia-protocol）
② 核对修复（阻塞：当前无修复代码落地）
③ 沉淀结论（本文）
④ 结束

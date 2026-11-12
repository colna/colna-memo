---
title: "onia-pipeline-review-v4 评审结论"
date: 2026-11-12
tags: review,code-review,onia,instagram-pipeline,server,dart
---

评审对象：`/Users/colna/WORK/_review/onia-pipeline-review-v4`（Onia 真实 Instagram 管线 v4 代码评审，仅基于该目录，只读模式）。

## 总判断

**有条件可合**——客户端 + scripts 可合；服务端代码带出不完整（lib.rs 声明的 6 个 pub mod 无源码），无法机器验收服务端。

## 必须修（M1–M8）

| # | 问题 | 位置 |
|---|------|------|
| M1 | server/lib.rs 模块树断裂：6 个 pub mod 无源码；`http`/`ws` 与文件名 `http_devices.rs`/`ws_mod.rs` 不匹配且无 `#[path]` | server/lib.rs |
| M2 | crates 缺失：仅存 `onia-device`，`onia-protocol`（信封真源）缺失 | crates/ |
| M3 | 失败结果被缓存，重试不会重新尝试 | client/lib/runtime/device_runtime.dart |
| M4 | `schedule_user` 吞包（两处） | client/lib/state/session.dart |
| M5 | `ensureDeviceId()` 不持久化 | client/lib/... |
| M6 | `let _ = idempotency::store(...)` 吞幂等写失败 | server/http_devices.rs:65 / :218 |
| M7 | bind_account `env.idempotency_key.as_deref().unwrap()` 直接解包 | server/... |
| M8 | 中文错误文案「账号未绑在该设备」（`resolve_pipeline_identity`） | server/... |

## 建议（S1–S14）

幂等分支 `PolicyView::default()` 不一致；promote/parse 软失败；事务 `rows_affected` 未检查；`cancel_todo unwrap_or(Pending)`；redispatch 仅 hello 触发；hello validate 失败无 error envelope；CORS `mirror_request`+`credentials`；重绑 cancel/move 硬失败 vs `cancel_open_todos_for_binding` 软失败不一致；register_screen 服务器地址字段位置；index.html 演示壳打包保留；tsconfig include 未覆盖 test/；测试覆盖缺口（sync_profile mismatch / probeSession 失败）；errors.ts `cls`/`error_class` 双别名冗余；WebView 挂树建议真机验证。

## 已核对无问题

UA 去掉 `; wv`/`Version/4.0`；hello 空 capabilities 防护；social_proxy.js 空 if；demo/live 分支、HostContext 回退链、`SocialProxy = { execute, probeSession }`、幂等键前缀规则（`device-bind:`/`account-bind:`/`behavior-result:`/`dev-device-account:`）、信封 `v/kind/id/trace_id/idempotency_key/ts/payload` 仅 `v=1`、risk/session 无自动重试、五项不回退项（Hero `Clip.none`、可注入 Session 延迟、`buttonActive`、注册成功 `popUntil`、密码 ≥8 位）；`redispatch_open_behaviors` 与 tracked.diff 893–951 逐行一致。

## 无法核对（源码缺失，不计问题也不计通过）

db/state/auth/config/error/idempotency 模块、Hub 实现、`decode_token`、幂等存储、`http::ok/parse_env`、`db::move_account_binding`/`update_binding_handle`、`crates/onia-protocol`、`should_retry`/`apply_result`。

## 方案契约参考

HTTP `POST /v1/auth/register`、`/v1/auth/login`、`/v1/devices/bind`、`/v1/devices/accounts/bind`；WS `GET /v1/ws?token=&device_id=`；信封禁止再包 `{ success, data }`。

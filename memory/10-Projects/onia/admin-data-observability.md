---
title: Onia 后台回关资料与聊天记录
date: 2026-11-17
tags: project, onia, admin, profiles, chat, rbac
---

# Onia 后台回关资料与聊天记录

## 目标

后台可查看回关后 `get_profile` 抓取的目标资料，以及已同步的私信聊天记录。普通用户只能看自己当前账号的数据，超级管理员可看全部。

## 数据与权限

- 资料：`profile_snapshots` + `profile_posts` / `profile_stories` / `profile_highlights`；列表按 `(platform, account_id, owner_handle)` 取最新快照。
- 聊天：`thread_messages` + `thread_cursors`；通过 `account_bindings` 关联 `user_id`，普通用户按绑定账号过滤。
- 服务端 `/v1/me/*` 强制使用 JWT 的 `user_id`，忽略请求里的伪造 `user_id`；`/v1/admin/*` 仅管理员可用。普通用户访问他人 `account_id` 时返回空/未找到，不泄露数据。

## API

- `profile.list` / `profile.list.ok`
- `profile.detail` / `profile.detail.ok`（含最近 60 帖子、60 Story、30 Highlights）
- `chat.thread.list` / `chat.thread.list.ok`
- `chat.messages` / `chat.messages.ok`
- 路由：`/v1/me/profiles/list|detail`、`/v1/admin/profiles/list|detail`；`/v1/me/chats/threads|messages`、`/v1/admin/chats/threads|messages`。

## 后台

- 新增「日常运营 → 回关资料」`/profiles`：账号选择（普通用户仅自己的账号，默认当前账号；超管默认全部）、资料摘要、媒体计数、抓取状态、详情弹窗。
- 新增「日常运营 → 聊天记录」`/chats`：账号/对方/最近消息/消息数/时间，点击查看收发消息。
- 协议类型由 `onia-export-ts` 生成；新增页面已加入 Umi 路由、侧栏和中文/英文文案。

## 修复

`get_profile` 返回的是扁平 profile 字段；原 `apply_get_profile_result` 只读取 `result.profile`，导致快照 `profile` 为空。现在优先读嵌套 `profile`，否则把扁平字段整理成 profile JSON，排除媒体数组/统计/分页字段。新增单测覆盖。

## 验证

- 本地隔离测试用户 A/B + 超管：A 只看到 `target_one`/`thread_a`，访问 B 的 profile detail 为 `profile not found`，访问 B 的 messages 为 0；B 只看到自己的；超管看到全部；A 调 `/v1/admin/*` 返回 `super admin role required`。
- 本地后台 Playwright/CDP 实测：普通用户 `/profiles`、`/chats` 只显示自己的数据；超管显示全部。
- `cargo test --workspace` 通过；admin typecheck/build 通过；生产 `profile.list.ok`、`chat.thread.list.ok` 及 `/profiles`、`/chats` 页面 200。
- 生产 server/admin 已部署；当前生产暂无 profile/chat 数据，页面显示空状态。

## 备注

- 当前列表上限：资料 200（最多 500）、会话 100（最多 300）、消息 500（最多 1000）；后续可加分页/筛选。
- 已提交并推送：`e6f0dbf fix(scripts): push active release updates to online devices`、`40803c5 feat(admin): add scoped profile and chat observability`；`main == origin/main`。

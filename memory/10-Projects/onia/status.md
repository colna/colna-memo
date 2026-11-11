---
title: Onia 当前进展
date: 2026-11-11
tags: project, onia, status, testing
---

# Onia 当前进展

更新于 2026-11-11 20:24（亚洲/上海）。文档只写在 colna-memo，不使用飞书。相关代码在本机 `/Users/colna/WORK/onia`，GitHub 私有仓库 [colna/onia](https://github.com/colna/onia)；本轮客户端和后台修复尚未 commit / push。

## 本机环境

- 服务端监听 `0.0.0.0:8080`，本机局域网地址 `192.168.3.24`。手机和后台不要用 `127.0.0.1` 作为 API。
- 后台 Umi 开发服务 `http://192.168.3.24:5173`，代理 `/v1`、`/health` 到 `http://127.0.0.1:8080`。
- PostgreSQL 在 Docker 容器 `onia-postgres`，主机端口 `5433`。
- 真机联调时不要启动 `apps/dev-device`。`local-sim` 及其 Instagram 绑定 `demo-account` / `onia_demo` 是演示残留，不得作为真实流程的通过依据。

## 代码与评审

- 真实管线客户端（官方 Instagram WebView、会话探测、live 脚本 `host.demo=false`）已实现；Code Reviewer 对 4 条必须改的复审结论为可合。建议（不阻断）：换号后在途行为的归属稍后处理。
- 后台登录后白屏已修复：Umi 4.7 默认 wrapper 收不到 `Outlet`，`apps/admin/src/wrappers/auth.tsx` 鉴权通过后改为 `return <Outlet />`。Code Reviewer 结论可合。建议（不阻断）：登录回跳地址再排除 `//`。不要用 `reactRouter5Compat` 绕过。
- 登录地址与 GraceChat / SP 一致：首页 `https://www.instagram.com/`，官方登录页 `https://www.instagram.com/accounts/login/`。Onia 先打开首页做会话探测，判定未登录后再加载登录地址。

## 真机联调（今日已收口的范围）

- 当前测试机是 OPPO Reno7 5G，ADB serial `f7031061`，产品名 PFJM10。已安装 Onia `0.1.0` release APK，API 为 `http://192.168.3.24:8080`。
- 测试账号 `111@163.com`（用户 id `52b09fbd-651f-4533-8805-0cab0fa5a3ef`）已在 App 内登录。不要在文档里写该账号密码，也不要代填 Instagram 密码。
- 当前在线设备：`4f311d52-6f63-4a5d-b26e-cc84f497f104`（名称「Onia 手机」），创建于 19:36:48（亚洲/上海），无 Instagram 绑定，capabilities 为空。
- 同名旧设备 `0bce1f33-2eeb-4129-b32e-74013f1ecbae` 已离线，最后可见 19:09:26。明天下发 Signal 必须对准 `4f311d52-…`，不要打到旧设备或 `local-sim`。
- 工作台状态：Onia 已登录、设备在线、Instagram「需验证」。WebView 已挂上树，停在 Instagram 未登录落地页（可见「打开 Instagram」「登录或注册」），不是已登录信息流，也未见账号密码表单。release 包无法从 logcat 证实精确 URL。
- 登录态探测已生效（没有把未登录误判为已登录），但本次标成了检查点「需验证」，因此没有再跳到官方登录地址。

## 下一步

- 用户本人在 WebView 里完成 Instagram 登录（含检查点）后，再测真实全链路：后台向 `4f311d52-…` 发送 Signal → App 执行 live 脚本 → 结果在后台可见。
- 测试中新发现的问题交给 Coder 改，改完交 Code Reviewer，通过后再继续测，直到全链路通过。提交与 MR 仍走 Git 助手。

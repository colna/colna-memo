---
title: createStory 缺 sessionId → 端上「成功但无 storyPk」假成功
date: 2026-11-02
tags: troubleshooting, sitin-next, app-ins-scripts, instagram, story
---

# createStory 缺 sessionId 时的「假成功」

## 现象

`createStory` 在调用方没有传 `sessionid` 时，端上 `publishByClient` 仍会走完并**报成功，但没有 `storyPk`**。结果是一条「成功」的发布记录，实际拿不到可用的 Story 链接。

旁证（2026-11-02 11:05 的线上盘点）：server 发布 142 次、client 发布 52 次，而 client 发布**没有 storyPk**，拼 Story 链接时只能退化成取 `mediaId` 下划线前段。见 [2026-11-02 Daily](../../50-Daily/2026-11-02.md)。

## 根因

- 原 `createStory` 的 steps 含「客户端上传并发布」，走 `publishByClient`；这条路径不依赖登录态，于是缺 `sessionid` 时**失败点被吞掉**，以本地发布成功收尾。
- 服务端发布步在缺 `sessionid` 时是静默 `return`，不产生错误码，上游也看不出问题。
- 2026-10-20 曾在原生端补过读 HttpOnly `sessionid` 的能力（`createStory` 服务端发布必须显式传），但不是所有安卓版本都有——老版本就落回这条静默路径。见 [2026-10-20 Daily](../../50-Daily/2026-10-20.md)。

## 修法（sitin-next MR !1355，commit `3c4d14834`）

文件：`packages/app-ins-scripts/src/instagram/methods/createStory.js`（43+/18-）

1. `storyFailure(e, code)` 增加可选错误码（963/969）。
2. **入口**缺 `sessionid` 直接返回 `SESSION_LOST`，位置在 pipeline 之前，零网络请求（1059-1069）。
3. 服务端发布步去掉静默 `return`，改显式 `SESSION_LOST`（1239-1250）。
4. steps 移除「客户端上传并发布」；`publishByClient` + 5 个专用 helper 保留未删（约 300 行死代码），`@deprecated` 实际只标了 `publishByClient`（1099 行）。

MR：https://gitlab.sitinai.com/frontend/sitin-next/-/merge_requests/1355 → `feature/sp`

## 验证

- `node --check createStory.js` EXIT 0（Git 操作员独立复跑）。
- build EXIT 0（3 bundle / 51 脚本）、vm 沙箱 2/2 PASS：由后端提供，**未独立复跑**（worktree 无 `node_modules`）。

## 未决 / 风险

- 老版本安卓无 `AndroidBridge.getInstagramSessionId` 时，端上发布由「成功但无 storyPk」转为**失败**（调度口径：线上约 52 次/日，待核）。
- `createStory` 级无灰度，全量生效。
- `SESSION_LOST` 在 APK 侧是否映射为 `success:false`，待前端核验。
- 是否给 5 个 helper 补 `@deprecated`、是否 rebase 到远端最新 `8ca10b1a`，均未决。

## 通用教训

一条「有登录态才成立」的发布路径，如果同时存在一条不校验登录态的本地兜底，缺参数时兜底会**静默成功**——比直接失败更难发现（无 error、无 storyPk）。修法是**在上游入口直接判失败**，而不是在兜底里补判断。

## 相关

- [sitin-next Story 预烧录重复贴纸](sitin-next-story-bake.md) — 同属 Story 发布链路，根因是 `isBake` 服务端未部署，与本条不同。
- [IG 脚本真机测试链路](instagram-device-script-test.md)

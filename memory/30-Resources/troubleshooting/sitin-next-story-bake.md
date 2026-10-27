---
title: sitin-next Story 预烧录重复贴纸（isBake 服务端未部署）
date: 2026-10-27
tags: [sitin-next, instagram, story, troubleshooting]
---

# Story 预烧录后出现两套贴纸

## 症状

客户端以 `isBake:false` 发送已预烧录贴纸的视频，发布后的 Story 仍出现白底蓝字的 `SHOP!` 贴纸，叠在视频自带贴纸上。

## 根因

`isBake` 复制源视频的分支来自 `ee545432b`，但当时 dev 部署的 `release/test-sp` 不包含该提交。服务端仍走 `renderLinkOverlay`，重新烧录并编码视频，导致客户端预烧录贴纸之外又叠加一层。

## 快速判断

直接 POST dev `/script/bake-story-video`，传 `isBake:false`：

- 返回视频体积明显小于源视频，或 geometry 被服务端重测 → 服务端仍在重新烧录。
- 返回入参 geometry 且视频走复制分支 → 服务端版本已正确。

2026-10-26 实测 geometry 从 `0.410` 被重测成 `0.3167`，体积从 `40,087,202` 降到 `19,824,320`，即命中了重烧录路径。

## 修法

1. 部署包含 `ee545432b` 的构建到测试环境；若还需要 `params.width/height` 生效，确认同时包含 `b832c99a4`。
2. 部署后再次检查 `story-bake.service.ts` 存在 `isBake === false` 复制分支。
3. 客户端脚本无需修改；问题在服务端版本。

## 后续

仍需确认 `publish_story` 原生 link sticker 与预烧录贴纸的视觉叠加策略，避免服务端版本正确后仍出现两套交互元素。

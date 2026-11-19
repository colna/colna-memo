---
title: Vela
date: 2026-11-19
updated: 2026-11-19 19:47
tags: project, vela, product, live-assist, tts, voice-clone, architecture, mvp
---

# Vela

个人项目：对标竞品「播搭子 / bodazi」能力面的 **AI 语音助播** 新品牌。clean-room 自建，不复用竞品视觉、品牌、文案、密钥、接口或协议实现。文档只写在 colna-memo，不使用飞书。

- 产品文档：[vela-prd.md](vela-prd.md)
- 技术方案：[vela-architecture.md](vela-architecture.md)
- 项目名决策见 2026-11-19 Daily 18:10；VoiceStudio 可行性评估见同日 18:04；产品文档初稿见同日 18:21，完整逆向报告补全修订见同日 19:20。

## 实现状态（2026-11-19 19:47）

用户于 19:22 要求按 PRD + 架构开始实现，样式使用 Meta Astryx；MVP 已在本地跑通。

- 代码：`/Users/colna/WORK/vela`（独立 monorepo，未创建远程仓库、未提交）
- 服务端：Rust axum + sqlx + PostgreSQL；模式 2/3、流式 TTS、播放队列、Mock 直播、音色克隆/试听、最小账号、WS 票据与 PCM16 推送
- 控制台：Umi 4 + React 19 + Astryx（音色库 / 话术本 / 开播控制台 / 设置 / 登录）
- 官网：Vite + React 19 + Astryx 深色品牌页
- TTS：默认 `MockTts`（可听合成提示音，不是真人语音）；`VoiceStudioTts` 为实验性 adapter，clone API 未验证
- 验证：`cargo test` 6 项通过；`cargo clippy -D warnings` 通过；`pnpm -r typecheck` / `pnpm -r build` 通过；`scripts/e2e.sh`（模式 2 + 模式 3 真实 WS 音频流）通过；控制台用 CDP 实测开播、3 句播完、停止、模式 3 模拟进入 + 手动插话
- 启动：`docker compose -f docker/docker-compose.yml up -d postgres` → `cd apps/server && cargo run` → `pnpm --filter @vela/admin dev` (5173) → `pnpm --filter @vela/web dev` (5175)

**仍未做**：真实抖音/直播平台、碎片音频与二创、其余模式、训练微调、支付卡密/多租户、App、VoiceStudio 商用清关。

## AI 与 VoiceStudio clone（2026-11-19 20:18）

- **AI**：`apps/script-agent`（Node 22）接 DeepSeek，server 代理 `/v1/ai/{generate,rewrite,reply,status}`；控制台支持 AI 生成/改写话术、助播 AI 回复入队。DeepSeek key 只在本机 `.env`（gitignored）。
- **VoiceStudio clone**：`POST /api/profiles`（兼容 `/profiles`）multipart → 写 `provider_voice_id`；`/ws/tts` 按上游源码对齐为 JSON `{text,voice,speed}` → PCM16 mono 24k → `done`/`error`。已有 mock HTTP 单测；**真实实例未联调**。
- 验证追加：AI 真实调用、控制台 AI 生成/采用/AI 回复、`cargo test` 7 项、e2e AI 分支均通过。

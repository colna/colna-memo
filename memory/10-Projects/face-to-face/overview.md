---
title: FaceForge(face-to-face)自建换脸平台 · 总览
date: 2026-07-28
tags: [face-to-face, faceforge, 换脸, facefusion, deep-live-cam, project, 创造模式]
---

# FaceForge(face-to-face)· 项目总览

自建、质量优先的 AI 换脸平台,一套系统覆盖 **图片 / 视频 / 实时**。数据不出本机。
仓库 `git@github-colna:colna/face-to-face`;本地 `/Users/max/Dev2/zhangzheng/face-to-face`。创造模式,「一直 loop 直到实现」。

## 架构

Monorepo:`apps/web`(Next15+React19+TS+Tailwind)+ `services/api`(FastAPI,REST+WS)+ `services/face-engine`(Python 引擎)+ `docker/`(GPU compose)。

- 图片/视频:**FaceFusion 3.6.x**(`hyperswap_1a_256` 换脸器 + CodeFormer 增强 + face-parser 遮挡蒙版),subprocess 调 headless-run。
- 实时:**Deep-Live-Cam**(逐帧,CoreML/CUDA)。
- 引擎「命令组装 ↔ 执行」解耦、全程依赖注入,逻辑层无 GPU 也能 mock 单测。

## 关键决策

- **换脸器 license**:`inswapper` 为 InsightFace **非商用**;商用走 `hyperswap` 系,逐个核对。
- **无 GPU 沙箱如实标注**:真实推理/模型下载/docker 构建标 🖥️,只在有卡机器验证,不假称跑通。
- **语言选型:引擎留 Python,不用 Rust 重写**。换脸瓶颈在 GPU 推理(ORT/CUDA/CoreML),不在语言;Rust 重写胶水对核心提速≈0(同一套 ORT),还会丢掉 FaceFusion/DLC/InsightFace 整个 Python 生态。唯一值得 Rust 的是**实时链路的 WS 帧中继/零拷贝**(延迟敏感);真正提速靠 TensorRT/FP16/批处理,与语言无关。
- **Mac/Apple Silicon 看效果**:走 **CoreML**(非 docker/CUDA),画质与 CUDA 一致、只是慢(图片几秒、视频分钟级、实时个位数~十几 fps)。方案见仓库 `docs/Mac运行指南.md`;需装 brew+python@3.11+ffmpeg,FaceFusionRunner 加 `--execution-providers coreml`(待应用项)。

## 当前状态(2026-07-28)

- **P0 全实现**:文档 → 脚手架 → 完整 engine(schema/模型管理/图片·视频·实时封装)→ 完整 API(REST+WS+/models)→ 完整 Web(三页+落地页)→ 部署/Mac 文档 → P1 质量预设。
- **测试**:66 单测全绿(engine 33 / api 17 / web 16),三层 lint+type-check clean。约 20 commit 推 origin/main。
- **唯一真阻塞**:GPU 真实推理(T2.6)+ 全栈 e2e(T5.1)+ 镜像构建,需有卡机器;模型 URL/sha256 待真机填真值。
- **P1 剩余**:T6.1 批处理+结果历史、T6.2 溯源水印(C2PA)+审计日志、T6.3 Redis/RQ 并发队列。

## 踩坑速查

- 用户全局 shell `NODE_ENV=production` → React 走生产构建、`React.act` 缺失 → web `test` 脚本前置 `NODE_ENV=test`。
- py3.9 + pydantic v2 运行期无法 eval PEP604 `int|None` → 用 `typing.Optional`,ruff ignore FA100。
- FastAPI `Depends/File/Form` 作默认值 → ruff B008 误报 → ignore B008。
- FaceFusion 3.x 需 Python 3.10–3.12;本机系统 py 只有 3.9.6。

> 详细执行记录见仓库 `docs/任务进度.md` 与 `50-Daily/2026-07-27`、`2026-07-28`。

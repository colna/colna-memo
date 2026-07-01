---
title: shangtang-sdk-demo
date: 2026-07-01
tags: [project, sensetime, sensemars, webar, vue3, vite, h5, tryon, sdk]
---

# shangtang-sdk-demo — SenseMARS WebEffects H5 集成 demo

## 一句话

商汤 SenseMARS WebEffects 2.3 H5 交付 demo,Vue 3 + Vite 移动端项目,集成 9 类 AR SDK(试戴/美妆/特效),线上 demo:https://tryon.softsugar.com/

## 授权与联系

- **License**:根目录 `WebAR.lic`,授权期 **2026-06-26 ~ 2026-07-26**(需前续期,否则 SDK 不可用)
- **发件人**:许佳(商汤 SenseMARS 交付)
- **SDK 包**:WebEffects2.3-h5.zip
  - 下载:https://deliverycenter.sensetime.com/deliverycenter/download/s2/4b96eaf0-75f2-4d3a-82ee-39f176a0fa2e
  - 提取码:`JjkSSmfa`

## 本地位置

- 仓库:`/Users/a0000/Dev/zhangzheng/shangtang-sdk-demo`(克隆自 `git@github-colna:presence-io/shangtang-sdk-demo.git`)
- 大小:749M(含 `public/sdk/` 里各 SDK 的 wasm/data/js)

## 技术栈

- **框架**:Vue 3.2 (`<script setup>` SFC)+ Vue Router 4 + Vue I18n 9
- **构建**:Vite 2.9(dev / build:test / build:prod 三 mode)
- **UI**:Vant 3 移动端组件 + Less + `postcss-px-to-viewport`
- **调试**:vConsole + stats-js(帧率)
- **平台**:H5 移动端(依赖 `getUserMedia` 摄像头 + WebGL / WASM)

## 目录 & 关键文件

```
shangtang-sdk-demo/
├── index.html                     # SPA 入口(title = Softsugar)
├── src/
│   ├── main.js
│   ├── router/index.js            # 4 条路由:/、/tryon/:id、/makeup/:id、/effects/:id
│   ├── pages/
│   │   ├── home/AllProjectListPage.vue    # 首页 grid
│   │   └── editor/
│   │       ├── tryon.vue           # 试戴(watch/shoes/glasses/ring)
│   │       ├── bueaty.vue          # 美妆 / 染发 / 美甲(makeup/hair/nail)
│   │       └── effects.vue         # 美颜特效(effects)
│   ├── webar/webar.js              # Webar 类:相机接入 + 帧循环 + processFrame/render 分发
│   ├── common/
│   │   ├── config.js               # 5 张 TYPE_LIST(见下)+ 各 SDK 环境变量
│   │   ├── index.js                # SdkBase:根据 type 动态 import SDK js 并 initHumanActionHandle
│   │   └── st-web-common.js
│   └── modules/camera/             # camera / image / mediaDevice
├── public/sdk/                     # 9 个 SDK 目录(独立 wasm + license)
│   ├── watch/  shoes/  glasses/  ring/     # tryon 系
│   ├── makeup/  hair/  nail/               # makeup 系(共享 st-ar-makeup.js)
│   └── effects/                            # effects 系(st-ar-effects.js)
├── nginx/                          # 生产 nginx 配置(default.conf + mime.types)
├── Dockerfile / docker-compose.yml # 部署
├── build.sh / dockerimg.sh
└── .env.development / .env.test / .env.prod
```

## SDK 类型全景(9 类 × 若干效果)

| SDK 目录 | 入口 JS | 分类 | 支持效果 |
|---|---|---|---|
| `watch` | `st-ar-tryon.js` | tryon | 手表 |
| `shoes` | `st-ar-tryon.js` | tryon | 鞋 |
| `glasses` | `st-ar-tryon.js` | tryon | 眼镜(前置) |
| `ring` | `st-ar-tryon.js` | tryon | 戒指 |
| `makeup` | `st-ar-makeup.js` | makeup | 口红、唇线、眼影、眼线、眼印、美瞳、睫毛、腮红、修容、粉底、眉毛(共 11 种) |
| `hair` | `st-ar-makeup.js` | makeup | 染发 |
| `nail` | `st-ar-makeup.js` | makeup | 美甲 |
| `effects` | `st-ar-effects.js` | effects | 基础美颜、美形、微整形、滤镜、贴纸、整妆、背景虚化 |
| (`glasses_mult`) | 备选路径 | tryon | 需要 SharedArrayBuffer 时的眼镜多线程版(代码里已注释掉) |

## SDK 加载机制(`src/common/index.js`)

- 根据 `type` 分派到三条初始化路径:
  - **tryon**:`import(sdkPath + 'st-ar-tryon.js')` → `new stARTryOn.default(canvas, ratio, sdkPath)` → `initHumanActionHandle(true, 1, "", "")`;`watch` / `shoes` / `ring` 各自调用 `setWristParams` / `setParams` 微调 delay_frame / roi
  - **makeup / hair / nail**:`asyncLoadJs(sdkPath + 'st-ar-makeup.js')` → `new window.stARMakeup(gl, w, h, sdkPath, true)` → `initHumanActionModule()` → `checkLicenseFromPath(sdkPath)`(**license 就在这里从 sdkPath 加载**)
  - **effects**:`asyncLoadJs(sdkPath + 'st-ar-effects.js')` → `new window.STAREffects(canvas, null)` → `instance.ready(sdkPath)`
- `Webar.render()` 走 `requestAnimationFrame`,从 video/image 抓 imageData 塞给 `sdkModule.processFrame(imgData, 0, outParams)`(tryon/makeup)或 `sdkModule.render({image_data, rotate, mirror}, {face_count})`(effects)

## 环境变量(SDK 路径切换)

三个 mode 对应三份 SDK 路径:

| Mode | 值 | 说明 |
|---|---|---|
| `development` | `/sdk/effects/`、`/sdk/watch/` 等 | 走本项目 `public/sdk/*`,**license 必须放这里** |
| `test` | `https://cdf-tryon.oss-cn-hangzhou.aliyuncs.com/sdk/softsugar/test/*/` | 阿里 OSS 测试环境 |
| `prod` | `https://cdf-tryon.oss-cn-hangzhou.aliyuncs.com/sdk/softsugar/release/*/` | 阿里 OSS 生产环境 |

变量名:`VITE_EFFECTS_SDK_PATH` / `VITE_GLASSES_SDK_PATH` / `VITE_HAIR_SDK_PATH` / `VITE_MAKEUP_SDK_PATH` / `VITE_NAIL_SDK_PATH` / `VITE_SHOES_SDK_PATH` / `VITE_WATCH_SDK_PATH` / `VITE_RING_SDK_PATH` / `VITE_GLASSES_MULT_SDK_PATH`

## License 放置约定(**踩坑注意**)

- **每个 SDK 目录都要一份自己的 license**(邮件明确说了):不是全局共享
- 本项目已用同一份 `WebAR.lic`(2774 字节)复制到 `public/sdk/{earring,effects,glasses,hair,makeup,nail,ring,shoes,watch}/` 全 9 个目录(2026-07-01 完成)
  - 注意 `earring` 目录存在但 config.js 没定义(邮件说 9 类,代码里 8 类可切换),先按物理目录放齐
- makeup 系(makeup/hair/nail)代码里显式调 `checkLicenseFromPath(sdkPath)` 校验;tryon / effects 走隐式校验(相关代码被注释)
- **prod / test 走 OSS 路径时,license 已经在 OSS 上,本地不用管**;只有 `dev` 模式(`npm run dev`)必须在 `public/sdk/*/` 放齐

## 启动命令

```bash
cd /Users/a0000/Dev/zhangzheng/shangtang-sdk-demo
npm install
npm run dev            # dev 模式 走 public/sdk/*
# 或
npm run build:test     # 走 OSS test
npm run build:prod     # 走 OSS release
```

- 浏览器需允许摄像头权限
- 若前端摄像头出问题,先看 vConsole(已内置)
- `/tryon/watch`、`/makeup/lipstick`、`/effects/filter` 是三个入口路由

## 部署

- **Docker**:`Dockerfile` + `docker-compose.yml`(nginx 静态托管 dist/)
- **CI**:`.gitlab-ci.yml`(GitLab CI,不是 GitHub Actions)
- **build 脚本**:`build.sh`(`npm run build:prod`)、`dockerimg.sh`(打镜像)

## 未验证 / 待办

- [ ] `npm install` 首次跑通(未验证)
- [ ] `npm run dev` 起来后 9 类 SDK 实际能否加载(尤其 `checkLicenseFromPath` 是否接受这份 lic)
- [ ] License 到期前(2026-07-26)找许佳续期,顺便确认是否升级到 WebEffects 2.4+
- [ ] `earring` 目录用途待确认(邮件默认 9 类效果,代码定义只有 8 类可切换)

## 相关

- 线上 demo:https://tryon.softsugar.com/
- WebEffects2.3 SDK 包(30 天短链):https://deliverycenter.sensetime.com/deliverycenter/download/s2/4b96eaf0-75f2-4d3a-82ee-39f176a0fa2e(提取码 `JjkSSmfa`)
- 集成到 sitin-next PWA 的技术方案 → [tech-proposal-pwa-integration](tech-proposal-pwa-integration.md)

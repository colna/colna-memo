---
title: PWA 加载速度联合优化技术方案(前端 × 客户端 haven)
date: 2026-08-17
tags: [sitin-next, app-pwa, feature/pwa, haven, 性能, 加载速度, 首屏, 技术方案, webview]
---

# PWA 加载速度联合优化技术方案(前端 feature/pwa × 客户端 haven/release)

> 2026-08-17 出草案 v1。前端 `sitin-next@feature/pwa` 的 `packages/app-pwa`,客户端 haven(女方端 App)WebView 宿主。
> 全文档见:飞书 outputs `PWA加载速度优化技术方案.md`(勘察 vite.config/index.html/main.tsx/bridge/globalBridge/service-worker/router)。

## 现状(勘察结论)

- PWA = **Vite + React SPA**(不是 Next SSR),跑在 haven **原生 WebView**(Android `WebView` / iOS `WKWebView`)里;原生桥 `window.pwaBridge`,`PwaBridgeReady` 事件就绪。
- 前端**包体积优化已做得差不多,边际收益低**:路由 36 处 `lazy()`;`vite.config` `manualChunks` 把 trtc/tuicall/im(@tencentcloud/chat)/mediapipe/echarts/proto 各自分包;重 SDK 不进 SW 预缓存(`injectManifest.globIgnores` 排除 trtc/tuicall/vconsole);Sentry + vConsole 延迟加载;gzip+brotli;骨架 spinner(`#skeleton-loader`);SW `precacheAndRoute` 预缓存 app shell;firebase 仅懒触发不进 entry。

## 慢的真根因 = 一条串行瀑布(必须联合客户端)

```
WebView 冷启动 → 拉 index.html/JS(head 里 4 个营销 SDK:Rangers/TikTok/Meta/AppsFlyer 抢主线程+带宽)
→ JS 执行 → 等 native window.reloadUser(token) 在 onPageFinished 之后才注入 token
→ fetchUserInfo(拿到 token 才发)→ 首页 lazy chunk → 首页数据 → 首屏可交互
```

最值钱的两个改造:① **token 从 onPageFinished 后注入,提前到首个业务 JS 执行前**;② **native 用已有 token 并行预取首屏接口注入,让数据请求与 JS 加载并行**。

## 方案(用户已定三取舍:分阶段全做 + Android/iOS 双端 + 先建度量)

- **P0 度量先行**:两端同一 `launch_id` 时间轴打点(t0/t_webview_created/t_page_finished/FCP/t_token_ready/t_first_business_paint),建白屏/FCP/TTI 基线(**当前无基线**)。目标待基线后定,参考:白屏 P90<1s、可交互 P90<2.5s。
- **P1 预热+预注入+瘦身**(轻、快收益):
  - 客户端:WebView 预热复用(Android WebView 池 / iOS WKProcessPool 复用);token 在 `document_start` 注入 `window.__NATIVE_AUTH__={token,userId,timSig}`(替代 onPageFinished 的 reloadUser 事件,事件保留为回退);native 并行预取首屏接口注入 `window.__PREFETCH__={userInfo,homeOverview,ts}`。
  - 前端:营销 SDK(TikTok/Meta/AppsFlyer)`requestIdleCallback`/首屏后再注入;Rangers 保留早初始化但加 preconnect;`__NATIVE_AUTH__`/`__PREFETCH__` 同步读取 + SWR revalidate;entry import 链审计(devTools/sitin4Debug 生产 tree-shake);preconnect/modulepreload。
- **P2 离线包**(重、收益最大):HTML/JS/CSS 后台静默下发,Android `shouldInterceptRequest` / iOS `WKURLSchemeHandler` 本地拦截返回;版本号+hash 校验、灰度、回滚;**线上 Vercel 永远兜底**;与 SW 双向版本校验避免契约错配。

## 分工节奏

P0 与 P1 的「WebView 预热」「前端瘦身」可**立即并行**;P1 的「数据预取」需两端先对齐首屏接口清单与注入协议;P2 待 P1 契约冻结后启动。

## 待客户端确认(阻塞)

1. haven WebView 承载方式(Android 有无现成 WebView 池;iOS 是否 WKWebView + 自定义 scheme)。
2. 有无「进入 PWA 前」空闲时机可预热(入口前一页面)。
3. native 能否用已登录 token 直连首屏接口(并行预取),还是只注入 token 由 H5 发。
4. 离线包下发通道:是否已有静态资源热更/配置下发基建可复用。
5. 客户端阶段打点走 native 埋点还是回传 H5 统一上报。

## 相关

- [sitin-next 项目笔记](sitin-next.md)
- [pwa-call-web-native-decouple](pwa-call-web-native-decouple.md) — 同样是 PWA×native bridge 解耦话题
- Partnership Ads PWA 技术方案(bridge 协作范式):[partnership-ads-pwa](partnership-ads-pwa.md)

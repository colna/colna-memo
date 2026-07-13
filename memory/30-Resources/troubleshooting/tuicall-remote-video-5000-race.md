---
title: TUICall/TRTC startRemoteView code 5000 竞态 — 等 DOM 容器,别盲重试
date: 2026-07-12
tags: [sitin-next, app-pwa, tuicall, trtc, webrtc, race-condition, MutationObserver]
---

# 症状

女方 PWA 主叫接通后 console 报:

```
TRTCClient code 5000 'view' is not found in the document object
  when calling startRemoteVideo()/updateRemoteVideo()
```

远端视频黑屏。被叫路径不复现。

# 根因(两层)

## 第一层:导航时机

- **被叫**在 accept 时就 `navigate("/video-call")`,元素早已 mount → 无竞态。
- **主叫**无 accept 步骤,只能在 `CALL_BEGIN`(`handleCallBegin`, useWebCall.tsx:537)接通瞬间才导航;而对方(男方)摄像头已开,`USER_VIDEO_AVAILABLE` 与 `CALL_BEGIN` 几乎同时到 → SDK 视频就绪回调跑赢 React mount,`#remote-video` 还没渲染。
- 叠加 `/video-call` 是 `lazy()` 懒加载,首次拉 chunk 延迟极易击穿 `startRemoteVideo` 里 `500ms×2 retryCnt` 的盲重试兜底。

## 第二层(升级认知)错误不走 promise reject

**TUICall SDK 的 `startRemoteView` 出错走 `onError` 事件异步冒泡,promise 不 reject。**

结论:`try/catch + await + retryCnt` 对这个 5000 从未生效,是死代码。只要 `startRemoteView` 在 `#remote-video` 挂载前触发,必报错、必黑屏。

# 修法

## 方案 1(第一轮,只减少频率不根治)

「页面就绪反向补绑」:VideoCallView mount 后主动补调 `startRemoteVideo`。

- `webCallManager.tsx`:加实例字段 `remoteVideoUserId` latch;`handleUserVideoAvailable` 里视频可用记 userID、不可用/`cloesConnect` 清;新增 `bindRemoteVideoIfAvailable()` —— latch 有值就补绑。
- `VideoCallView.tsx`:`useEffect([callState])` 里 mount 后调 `bindRemoteVideoIfAvailable()`。

**问题**:两条路(SDK 回调 + mount 补绑)都可能触发,减少了竞态但没根治;偶发仍报 5000。

## 方案 2 ✅(第二轮,硬修根治)

**等元素进 DOM 再调 startRemoteView。**

改 `webCallManager.tsx`:

1. 模块级 helper `waitForElement(id, timeoutMs)`,用 `MutationObserver` 等元素 attach(超时 8s)。
2. `startRemoteVideo` 重写:先 `await waitForElement("remote-video")` 拿到元素**才**调 `startRemoteView`;**删掉失效的 retryCnt/delay 及 `delay` import**(死代码)。
3. 加去重:`remoteVideoBoundUserId` + `remoteVideoBindInFlight`,SDK 回调路和 mount 补绑路两侧都调,去重防并发重复绑定;对方关摄像头(video unavailable)清 bound 以便重绑;`cloesConnect` 一并清。

保留方案 1 的 latch + `bindRemoteVideoIfAvailable` + VideoCallView mount effect 作补绑兜底,与去重协同不 double-bind。

```ts
function waitForElement(id: string, timeoutMs = 8000): Promise<HTMLElement> {
  return new Promise((resolve, reject) => {
    const found = document.getElementById(id);
    if (found) return resolve(found);
    const observer = new MutationObserver(() => {
      const el = document.getElementById(id);
      if (el) { observer.disconnect(); resolve(el); }
    });
    observer.observe(document.body, { childList: true, subtree: true });
    setTimeout(() => { observer.disconnect(); reject(new Error("timeout")); }, timeoutMs);
  });
}
```

# 通用教训(**跨 SDK 复用**)

1. **第三方 RTC/媒体 SDK 的错误未必走 promise reject** —— 可能只发 `onError` 事件。写「await + try/catch + 重试」前先确认错误如何暴露,否则可能是死代码。
2. **涉及 DOM 容器的媒体绑定,先确保容器进 DOM 再调用**(`MutationObserver`),别用「盲固定次数 × 固定 delay」重试。UI 就绪反向驱动媒体绑定,比等 SDK 回调时机稳。
3. **懒加载路由 + SDK 回调**是经典竞态源;首次访问的 chunk 延迟会击穿常规兜底。

# 相关

- 分支 `personal/zz/pwa-web-call-order`(PR #591),两轮 commit:
  - `adddad630` `fix(app-pwa): rebind remote video on in-call page mount to fix black frame`(方案 1)
  - `029d24b81` `fix(app-pwa): wait for #remote-video before startRemoteView to kill 5000`(方案 2 根治)
- 项目笔记:[[../../10-Projects/pwa-call-web-native-decouple]]

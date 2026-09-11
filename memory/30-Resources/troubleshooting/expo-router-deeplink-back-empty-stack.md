---
title: expo-router：深链冷启动进二级页，返回键报 GO_BACK 且什么都点不动
date: 2026-09-11
tags: [troubleshooting, expo-router, react-native, navigation, sitin-rn]
---

# expo-router：深链冷启动后 `router.back()` 无处可回

出处：sitin-rn `apps/luka`，改编辑资料屏（Edit profile）时用户报
`The action 'GO_BACK' was not handled by any navigator.`

## 症状

- 控制台/系统日志一句 `The action 'GO_BACK' was not handled by any navigator.` +
  `Is there any screen to go back to?`，并注明 development-only。
- **界面上什么都没发生**：点返回键，页面纹丝不动，看起来像按钮坏了。警告比症状显眼，
  容易只盯着警告去查。
- 只在**冷启动直达某个二级屏**时出现：深链（`luka://edit-profile/name`）、通知/推送落地、
  从 dev client 的 recent 直接进某屏。

## 根因

栈里只有当前这一屏时，`router.back()` 没有目标，React Navigation 于是只打警告、不动作。
正常从上一屏 push 进来栈里总有东西，所以**本地点着玩永远复现不了** —— 触发条件是**进入方式**，
不是那一屏本身。

## 修法

`canGoBack()` 分流 + 一个兜底路由（koda `chat-header.tsx` 已有先例，这里抽成可传兜底的形式）：

```ts
export function useGoBack(fallback?: Href): () => void {
  const router = useRouter();
  return useCallback(() => {
    if (router.canGoBack()) { router.back(); return; }
    if (fallback) router.replace(fallback);
  }, [router, fallback]);
}
```

要点：

- **兜底要就近**：编辑资料各屏回 hub（`/edit-profile`），hub 回 `/(tabs)/me`。
- **存盘后也要走同一个口**：`save → router.back()` 在深链场景下同样卡在原地，改完就出不去。
- **共用外壳要让调用方传兜底**（props 上一个 `backFallback`），别在共用组件里写死目标。
- 不给兜底就**静默不动**（只可能被 push 进入的屏，如注册向导）—— 比打警告好，也不需要假目标。

## 验证

- 模拟器里能自动验的只有渲染，**点击无法脚本化**（`simctl` 没有 tap 子命令，本机没装 idb），
  「点返回落到兜底页」这步只能手点。
- 系统日志能事后取证（连用户那次也能查到，带时间戳）：
  `xcrun simctl spawn booted log show --last 10m --predicate 'eventMessage CONTAINS "GO_BACK"'`
  —— RN 的 console 警告会进 `com.facebook.react.log:javascript`。
- 模拟器里让 dev client 冷启动进 JS（而不是它自己的启动页列表）：
  `xcrun simctl openurl booted 'luka://expo-development-client/?url=http%3A%2F%2Flocalhost%3A8081'`，
  之后再深链具体页面。
- **完整冷启动链路**（模拟器已关时可以整套自己跑，不用等用户）：

  ```sh
  xcrun simctl boot <UDID>        # 先在 `xcrun simctl list devices available` 里挑
  xcrun simctl openurl booted 'luka://expo-development-client/?url=http%3A%2F%2Flocalhost%3A8081'
  sleep 35                        # 首屏 bundle 下来要等，别急
  xcrun simctl openurl booted 'luka://<业务深链>'
  xcrun simctl io booted screenshot /tmp/x.png
  ```

  ⚠️ **顺序反了会误判**：app 还没连上 Metro 时直接打业务深链，拿到的是 dev client 自己的
  launcher（DEV SERVERS 列表那屏），截图看着像「深链压根没生效」。先连 Metro，再打业务深链。

## 连带教训

RN 的 dev-only 警告会顺着 JS → 原生日志管道进 os_log，**事后也能取证**：用户只看到红字、
说不清什么时候触发的，日志里有时间戳，能对齐到自己当时那一步操作。

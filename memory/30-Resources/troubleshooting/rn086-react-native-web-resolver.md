---
title: RN 0.86 + react-native-web 0.21 打包 500（web 预览跑不通）
date: 2026-09-08
tags: [react-native, expo, react-native-web, metro, 版本不兼容]
---

# RN 0.86 + react-native-web 0.21：web 预览打包 500

## 症状

`expo start --web` 能起 Metro、端口在监听、首页 HTML 返回 200，但请求 bundle 返回
**HTTP 500**：

```
UnableToResolveError: Unable to resolve module
  ../../src/private/devsupport/rndevtools/ReactDevToolsSettingsManager
from node_modules/react-native/Libraries/Core/setUpReactDevTools.js
```

## 根因

RN 0.86 把 `ReactDevToolsSettingsManager` 移进了 `src/private/`，而
`react-native-web@0.21` 的 resolver 还按旧路径找。**与业务代码无关**，任何用这对版本的
app 都一样。

## 怎么确认不是自己的问题

不要靠猜。直接打一次 bundle：

```bash
npx expo start --web --port 8099 &
# 从首页 HTML 里取真实 bundle 路径（不是 /index.bundle，Expo Router 的入口不同）
curl -s http://localhost:8099/ | grep -o 'src="[^"]*\.bundle[^"]*"'
curl -s -o /tmp/b.js -w "%{http_code}\n" "http://localhost:8099/<上面那个路径>"
python3 -c "import json;d=json.load(open('/tmp/b.js'));print(d['type']);print(d['message'][:400])"
```

500 的 body 是 JSON，`type` + `message` 直接说清是哪个模块解析失败。

## 现状

**没修。** 这个仓库 web 只是顺带产物，iOS/Android 才是目标。要看效果走模拟器。

真要修的话方向是给 Metro 加一条 `resolver.resolveRequest` 把旧路径映射到新路径，或者
等 react-native-web 跟上 —— 但为一个不发布的平台加 resolver 补丁不划算。

## 顺带：本机跑模拟器的前提

- iOS：**要装 Xcode**。只有 Command Line Tools 不行 —— `xcodebuild` 会报
  `requires Xcode, but active developer directory is a command line tools instance`，
  `xcrun simctl list devices` 也列不出任何设备。
- Android：要 `~/Library/Android/sdk`（Android Studio 装）。
- 判断脚本：

```bash
xcodebuild -version                              # 报错 = 没 Xcode
xcrun simctl list devices available | grep -i iphone
ls ~/Library/Android/sdk/emulator/emulator
```

---
title: Luka 房号真机包：扩展签名与 USB 隧道连 Metro（sitin-rn）
date: 2026-09-24
tags: troubleshooting, ios, expo, react-native, xcode, devicectl, sitin-rn
---

# Luka 房号真机包：扩展签名与 USB 隧道连 Metro

场景：把 `apps/luka`（分支 `feat/luka-analytics-parity`，含未提交改动）的 Development
Build 装到 **iPad Air 11-inch (M3)（iPadOS 26.3.1）**并连 Metro 调试。公司 Team
`39CFYH6W55`。Xcode 27。四个坑，前两个是主线。

姊妹篇：[[ios-device-free-team-debug]]（免费 Personal Team + iOS 16 设备）。

## 坑 1：房号扩展没有开发描述文件，`expo run:ios` 不帮你建

- **现象**：`pnpm --filter luka ios:device`（内部跑 `expo run:ios --device`）在签名阶段
  失败，两类报错随本地缓存状态变化：
  - 本地有旧通配描述文件时：HomeWidget 落 `iOS Team Provisioning Profile: *` →
    `does not support the App Groups capability` / `doesn't support the group.com.presence.gracechat App Group` /
    NSE + Widget `doesn't include the currently selected device "iPad"`；
  - 删掉通配描述文件后：`No profiles for 'com.presence.gracechat.widget' were found …
    Automatic signing is disabled and unable to generate a profile. To enable automatic
    signing, pass -allowProvisioningUpdates to xcodebuild.`
- **根因**：`expo run:ios` **没有传 `-allowProvisioningUpdates`**，它只会用本地已有描述
  文件；而 `com.presence.gracechat`（主 App）有描述文件，两个房号扩展
  （`.widget` / `.notiservice`）在 2026-09-23 dev 档放开 widget 后是**第一次**上真机，
  Apple 后台还没有它们的开发描述文件。
- **修法**：走传统通道，显式让 Xcode 自动签名（**不需要**去 Apple 后台手动建 App ID）：
  ```bash
  cd apps/luka/ios
  xcodebuild -workspace Luka.xcworkspace -scheme Luka -configuration Debug \
    -destination 'generic/platform=iOS' \
    -allowProvisioningUpdates -allowProvisioningDeviceRegistration \
    -derivedDataPath build DEVELOPMENT_TEAM=39CFYH6W55 CODE_SIGN_STYLE=Automatic build
  ```
  这条命令自己把 `com.presence.gracechat.widget` / `.notiservice` 的 App ID + 开发描述
  文件注册好（含 App Group 关联，主 App 的描述文件本就带 iPad UDID）。
- **判据**：报错里出现 `Automatic signing is disabled` ⇒ 加 `-allowProvisioningUpdates`；
  出现通配描述文件的名 `iOS Team Provisioning Profile: *` 且提到 App Groups ⇒ 该扩展
  App ID 未注册，同一条命令一次就能建出来。
- **注意**：`expo run:ios` 前还要先 `-allowProvisioningUpdates` 建好一次；之后本地有
  描述文件了，普通 `expo run:ios` 才不会再卡。

## 坑 2：Wi-Fi 传输极慢（~2%/min）→ 用 USB 隧道给 dev client 指向 Metro

- **现象**：App 连上 Metro 后 bundle 进度 `Downloading 1% → 3% → 5%`（约 2%/分钟，
  估计 40 分钟下完）；`netstat -anv -p tcp` 看 Metro→iPad 的 socket **sendq 积压 1.6 MB**；
  Metro 进程 CPU 0.3%（不是打包慢）。Mac 侧 Wi-Fi 信号 −43 dBm / 286 Mbps（没问题）。
- **根因**：iPad 侧 Wi-Fi 到 Mac 的实际吞吐极差（同一 5GHz SSID 也未必快），
  而 iOS 17+ 的 CoreDevice 里本来就有一条 **USB 隧道**（`devicectl` 的 wired 连接），
  只是 dev client 默认用 LAN IP。
- **修法**：把 dev client 的 Metro 地址指到 **Mac 在 USB 隧道上的 IPv6**：
  1. 取地址：`xcrun devicectl device info details --device <UDID>` →
     `Tunnel IP Address` 是**设备侧**（如 `fd79:1a6d:1adf::1`）；Mac 侧是同前缀的
     `::2`，`ifconfig` 里能在对应 `utun` 上看到（如 `utun5: inet6 fd79:1a6d:1adf::2`）。
  2. 深链拉起（URL 里 IPv6 要 `[...]` 且整体百分号编码）：
     ```bash
     xcrun devicectl device process launch --terminate-existing \
       --device <UDID> \
       --payload-url "luka://expo-development-client/?url=http%3A%2F%2F%5Bfd79%3A1a6d%3A1adf%3A%3A2%5D%3A8081" \
       com.presence.gracechat
     ```
  3. 验证：`lsof -nP -iTCP:8081 | rg ESTABLISHED` 出现 `[fd79:…::2]:8081->[fd79:…::1]`
     的连接；bundle 秒级下完，`curl http://localhost:8081/json/list` 出现
     `com.presence.gracechat (iPad)`。
- **注意**：隧道地址**每次重连可能变**（本次实测下午还是 `fd2a:f2f0:bffb::1`，晚上变成
  `fd79:1a6d:1adf::1`）；每次重新取。`ping6` 那个地址会报 `nodename nor servname`，
  不影响使用。

## 坑 3：首次启动卡在「本地网络」权限弹窗

- **现象**：dev launcher 一直停在 "Find Dev Servers"，`/json/list` 空、8081 无连接。
- **根因**：iOS 的本地网络权限弹窗在等用户点「允许」（提示文案是
  `允许 "Luka" 查找本地网络中的设备?`）。
- **修法**：让用户点「允许」（或 设置 → 隐私与安全性 → 本地网络 里开）。
- **取证**：`xcrun devicectl device capture screenshot --device <UDID> --destination /tmp/x.png`
  可以直接看设备当前屏幕（Xcode 27 的 devicectl 支持），比反复猜快得多。

## 坑 4：房号撞包 —— 装 Luka 会覆盖同机其它派生包

- 非生产档（dev/preview/nightly/本机真机调试）全产线共用 `com.presence.gracechat`
  （AGENTS.md 强制规则 10）。同一台 iPad 上装 Luka 会**顶掉** Rocco 等同样落房号的
  派生包；若对方来自 TestFlight，之后自动更新又可能把 Luka 换回去。看进程名就能发现
  （`xcrun devicectl device info processes` / `--console` 输出里的进程名不是 Luka 时）。

## 常用命令速查（本次实际用到的）

```bash
# 构建（含 pod install 的 prebuild 见 apps/luka/package.json 的 ios:device 脚本）
cd apps/luka/ios && xcodebuild -workspace Luka.xcworkspace -scheme Luka -configuration Debug \
  -destination 'generic/platform=iOS' -allowProvisioningUpdates -allowProvisioningDeviceRegistration \
  -derivedDataPath build DEVELOPMENT_TEAM=39CFYH6W55 CODE_SIGN_STYLE=Automatic build
# 安装 / 拉起 / 看进程 / 截图
xcrun devicectl device install app --device <UDID> build/Build/Products/Debug-iphoneos/Luka.app
xcrun devicectl device process launch --terminate-existing --device <UDID> --payload-url "…" com.presence.gracechat
xcrun devicectl device info processes --device <UDID>
xcrun devicectl device capture screenshot --device <UDID> --destination /tmp/x.png
```

前提：`apps/luka/.env`（gitignored）里要有 `EXPO_APPLE_TEAM_ID=39CFYH6W55`，
否则 `app.config.ts` 的 `appleTeamId` 为空、自动签名会去问 Team。

## 2026-09-24 追加：dev client 会拿旧 bundle；iPad 兼容窗口的真实几何

### 改了 JS 但 iPad 上没生效 —— dev client 缓存了上次的 bundle

- **现象**：Metro（8081）明明已经含新代码（`curl` 整个 entry bundle 里有新符号），但 iPad 上
  反复 `devicectl ... --terminate-existing --payload-url <expo-development-client URL>` 拉起后
  画面还是旧版；换一个端口起第二个 Metro（8082）也没用。
- **根因**：expo-dev-client 把上次加载成功的 bundle 缓存在 App 里，冷启动时可能直接用它，
  不重新拉。
- **修法（不用重装、不用点屏幕）**：Expo CLI 的 dev server 有 **`POST /reload`**：向所有已连
  客户端广播 reload，客户端会重新拉 bundle。
  ```bash
  curl -s -X POST http://localhost:8081/reload     # 200
  ```
  之后截图确认即可。排查顺序：① `curl` entry bundle 确认 Metro 内容；② `/reload`；③ 再不行才
  换端口/重装。
- **取 bundle 的正确 URL**（`/index.bundle` 那个是老写法，会 404）：
  ```bash
  curl -s -H "expo-platform: ios" http://127.0.0.1:8081/ | python3 -c "import json,sys;print(json.load(sys.stdin)['launchAsset']['url'])"
  # 想看源码文本：把 launchAsset.url 里的 transform.bytecode=1 改成 0 再 curl
  ```

### iPad 兼容窗口的真实几何（375×669 逻辑点，整体放大 ~3.4×）

- `useWindowDimensions()` 报 **375 宽**（与 iPhone 相同），但高度只有 **669**（iPhone 11 是 812）；
  `PixelRatio` = 2，而截图 1640px 宽 → 实际渲染放大 ~3.4 倍。**别按截图像素猜布局**。
- 实测（Luka 划卡页）：`vh 647 / topInset 124 / tabBarClearance 100 / BELOW 97 / hero 112`
  → 卡片可用高度只有 **~214pt**（iPhone 上约 350）。
- 推论：给 iPad 做「兼容」时，问题通常是**高度不够**，不是宽度太宽；把卡按 350×547 比例收窄
  之后还要把卡内内容（字号/内边距/标签）一起收，否则固定的信息带仍会吃掉大半个卡。
- 布局回归：改完用 `devicectl device capture screenshot` 看真机，同时在 `PixelRatio`/尺寸函数
  上写单测（`discoveryCardFit` 那套）。

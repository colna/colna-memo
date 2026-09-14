---
title: iOS 真机调试:免费 Personal Team + iOS 16 设备(sitin-rn)
date: 2026-09-14
tags: troubleshooting, ios, expo, react-native, xcode, sitin-rn
---

# iOS 真机调试:免费 Personal Team + iOS 16 设备

场景:`apps/luka` 真机调试,公司付费 Team 未生效,先用免费 Personal Team 绕过;设备是
iPhone X(iOS 16.7.16)。四个坑连环踩,每个的根因都不在表面。

## 坑 1:证书创建了,`find-identity -v` 仍是 0 valid

- **现象**:Xcode → Manage Certificates 创建 Apple Development 成功(列表可见),
  但 `security find-identity -v -p codesigning` 返回 `0 valid identities found`;
  不带 `-v` 能看到 1 个 identity。`security verify-cert` 报 `CSSMERR_TP_NOT_TRUSTED`。
- **根因**:系统里只有 **2013–2023 的旧 WWDR 中间证书(已过期)**;新证书由
  **WWDR G3** 签发(`openssl x509 -noout -issuer` 看 OU=G3),缺中间证书 → 信任链断。
- **修法**:
  ```bash
  curl -sL -o /tmp/AppleWWDRCAG3.cer https://www.apple.com/certificateauthority/AppleWWDRCAG3.cer
  security import /tmp/AppleWWDRCAG3.cer -k ~/Library/Keychains/login.keychain-db
  security find-identity -v -p codesigning   # 应显示 1 valid
  ```
- **判据**:证书在、`-v` 为 0、verify 报 NOT_TRUSTED → 先查 issuer 是 G3 还是 G4,再补对应中间证书。

## 坑 2:USB 看不到设备

- **现象**:`xcrun devicectl list devices` 里设备 `unavailable`;`ioreg -p IOUSB`
  里根本没有 iPhone。
- **根因**:用了**纯充电线**。`system_profiler SPUSBDataType` 在这台 Mac 上无输出
  (macOS 26 的怪癖),用 `ioreg -p IOUSB | grep iPhone` 判断更可靠。
- **修法**:换支持数据传输的线,直插 Mac;手机上「信任此电脑」。Xcode →
  Window → Devices and Simulators 显示 **Connected** 才算就绪。
- **注意**:Xcode 显示 Connected ≠ devicectl 可用(iOS 16 见坑 3)。

## 坑 3:`expo run:ios --device` 找不到设备(devicectl 不支持 iOS 16)

- **现象**:`expo run:ios --device "iPhone2-T"` 报
  `No device UDID or name matching "iphone2-t"`,而 Xcode Devices 窗口里设备明明 Connected。
- **根因**:**`devicectl` 只支持 iOS 17+**;iOS 16 设备在 devicectl 里是
  `pairing: unsupported` / `state: unavailable` 的陈旧记录(甚至显示为另一个 UUID)。
  而 `@expo/cli` 的 run:ios 用 devicectl 发现物理设备 → 直接看不到。
- **修法(传统通道构建 + Xcode 安装)**:
  ```bash
  cd apps/luka
  # 传统 UDID 从 Xcode Devices 窗口的 Identifier 抄,40 位 hex
  xcodebuild -workspace ios/Luka.xcworkspace -scheme Luka -configuration Debug \
    -destination "id=<40位UDID>" \
    -allowProvisioningUpdates -allowProvisioningDeviceRegistration \
    -derivedDataPath ios/build \
    DEVELOPMENT_TEAM=<TeamID> CODE_SIGN_STYLE=Automatic build
  ```
  - 命令行传 `DEVELOPMENT_TEAM` 会 **override 全部 target**(主 target + 两个
    extension),不需要手改 pbxproj;`-allowProvisioningUpdates` 自动注册
    App ID + 生成描述文件(免费号也能注册 3 个普通 App ID)。
  - `devicectl device install app` 同样不支持 iOS 16 → 安装用 Xcode →
    Devices and Simulators → INSTALLED APPS 的 `+` 选
    `ios/build/Build/Products/Debug-iphoneos/<App>.app`。
  - 首次打开报「不受信任的开发者」→ 设置 → 通用 → VPN与设备管理 → 信任。

## 坑 4:免费 Team 不支持的 entitlement(autolink 注入,清不干净)

免费 Personal Team 不支持 **推送 / App Groups / Sign in with Apple**;而这三个
entitlement 由 **autolink 的 config plugin** 注入,**即使包不在 `plugins` 数组里**:

| entitlement | 注入者 | 后果 |
| --- | --- | --- |
| `aps-environment` | `expo-notifications` | 签名报 "profile does not support the Push Notifications capability" |
| `com.apple.security.application-groups` | `@heyhru/rn-home-widget` | App Group 是付费功能 |
| `com.apple.developer.applesignin` | `expo-apple-authentication` | Sign in with Apple 是付费功能 |

- **修法(临时,`ios/` 不入库)**:prebuild 后手动清,每次 `prebuild --clean` 都要重清:
  ```bash
  /usr/libexec/PlistBuddy -c "Delete :aps-environment" ios/<App>/<App>.entitlements
  /usr/libexec/PlistBuddy -c "Delete :com.apple.developer.applesignin" ios/<App>/<App>.entitlements
  /usr/libexec/PlistBuddy -c "Delete :com.apple.security.application-groups" ios/<App>/<App>.entitlements
  /usr/libexec/PlistBuddy -c "Delete :com.apple.security.application-groups" ios/HomeWidget/HomeWidget.entitlements
  ```
- **正规方案参考**:`apps/intro` 的 `INTRO_CLEAN_INSTALL=1` 模式 —— 条件排除推送插件、
  换专用 bundleId、挂 `@heyhru/rn-expo-plugins/strip-ios-push-entitlements`(但它
  故意不删 applesignin,那一个仍需手工处理)。
- **代价**:推送收不到、桌面小组件读写不了、Apple 登录不可用;App 本体能跑。

## 完整流程(免费 Team 本地真机,以 luka 为例)

1. Xcode → Settings → Accounts 登录 Apple ID;Manage Certificates 创建 Apple Development。
2. `security find-identity -v -p codesigning` 必须 ≥1 valid(否则回坑 1)。
3. 手机数据线连接 + 信任,Xcode Devices 显示 Connected。
4. `app.config.ts` 的 `appleTeamId` 填真实 Team ID(占位符会让自动签名解析不出描述文件)。
5. prebuild(带该 app 的 ios:device 环境变量,如 `HEYHRU_BYTEPLUS_NATIVE=1`)。
6. 清 entitlements(坑 4 的三条命令)。
7. xcodebuild 传统通道构建(坑 3 的命令)。
8. Xcode Devices 窗口 `+` 安装 `.app`;手机信任证书。
9. 起 Metro(`pnpm <app>:start`),手机打开 App。
10. **之后不要跑 `<app>:ios:device`** —— 它会 `prebuild --clean`,冲掉第 6 步并把
    entitlements 写回,还会重新全量编译。

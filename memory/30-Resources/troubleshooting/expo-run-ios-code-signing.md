---
title: expo run:ios 排错(模拟器也要求签名证书)
date: 2026-09-09
tags: troubleshooting, expo, ios, react-native, sitin-rn
---

# expo run:ios 排错

## 跑模拟器却报 `No code signing certificates are available to use.`(2026-09-09)

- **现象**:`pnpm luka:ios` → prebuild / CocoaPods 全过,最后挂在
  ```
  › Your computer requires some additional setup before you can build onto physical iOS devices.
  CommandError: No code signing certificates are available to use.
  ```

- **不是环境问题**(这些都排掉了,别再查):`xcode-select -p` 指向 Xcode.app 正常、Xcode 26.6、
  `simctl list devices available` 有 11 个模拟器(iPhone 17 还是 Booted)、磁盘 63Gi 可用、
  `xctrace list devices` 里没有物理 iOS 设备(只有 Mac 自己)。

- **根因**:`@expo/cli` 的 `build/src/run/ios/XcodeBuild.js:279`
  ```js
  if (props.device && (!props.isSimulator || simulatorBuildRequiresCodeSigning(projectRoot))) {
      await ensureDeviceIsCodeSignedForDeploymentAsync(projectRoot)
  }
  ```
  `simulatorBuildRequiresCodeSigning`(`run/ios/codeSigning/simulatorCodeSigning.js`)读 entitlements,
  **命中下面任一条,模拟器构建也强制要开发证书**:
  - `com.apple.developer.associated-domains`
  - `com.apple.developer.applesignin`

  `apps/luka/app.config.ts` 的 `ios.usesAppleSignIn: true` 正好生成后者,而这台机器 Xcode 没登 Apple ID、
  keychain 里没有 Apple Development 证书 → 报错。

- **判据**:报错文案提到 "physical iOS devices" 但你明明在跑模拟器 → 直接去 app.config 查
  `usesAppleSignIn` / `associatedDomains`,不要去查 Xcode 环境和模拟器列表。

- **修法 A(只要先跑起来)**:注释掉 `usesAppleSignIn: true`,然后**必须 `--clean` 重新 prebuild**
  (entitlements 已生成过,不 clean 不会重写):
  ```bash
  cd apps/<app> && npx expo prebuild --clean && cd ../.. && pnpm <app>:ios
  ```
- **修法 B(正解)**:Xcode → Settings → Accounts 加 Apple ID(免费个人号即可签 Apple Development),
  并把 `appleTeamId` 从占位符换成真实 Team ID,再 `prebuild --clean`。

- **sitin-rn 特有坑**:`apps/blueprint` 模板里写死 `appleTeamId: "XXXXXXXXXX"` + `usesAppleSignIn: true`,
  **凡是从 blueprint 派生的新 app(如 luka)都继承这对占位配置**,第一次跑 iOS 必踩。
  对照组:`lumi`/`koda` 用真实 `39CFYH6W55`,`intro`/`iris`/`naya` 用 `DEVELOPMENT_TEAM` 常量。
  派生新 app 时把这两行一起补掉,能省一轮排查。
  (app.config.ts 里原本就有注释警告占位值会让自动签名解析不出描述文件 —— 但它只提了真机场景,
  没提模拟器也会被 applesignin 连累。)

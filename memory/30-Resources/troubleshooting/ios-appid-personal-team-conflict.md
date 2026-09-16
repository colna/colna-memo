---
title: 同一 Apple ID 的 Personal Team 占 App ID,阻塞付费 Team 注册(sitin-rn/luka)
date: 2026-09-16
tags: troubleshooting, ios, xcode, provisioning, appid, sitin-rn, luka
---

# 同一 Apple ID 的 Personal Team 占 App ID,阻塞付费 Team 注册

场景:`apps/luka` 从免费 Personal Team 切到公司 Team(AI FANTASY,`39CFYH6W55`)后做真机
构建,三个 target 全部报「不能注册 App ID」。

## 现象

```
error: Failed Registering Bundle Identifier: The app identifier "com.lukasoc.luka.dev"
cannot be registered to your development team because it is not available.
Change your bundle identifier to a unique string to try again.
```

主 target 与两个 extension(`.luka.notiservice` / `.luka.widget`)全中,构建在
`GatherProvisioningInputs` 阶段秒挂(没到编译)。

## 根因

- **同一个 Apple ID**(本机两张证书 UID 都是 `U5P7RUMJ28`)下,免费 **Personal Team**
  (`PD2XGB735Y`)在 09-14 先用掉了这三个 bundle id;
- Apple 的 App ID 注册表**跨 team 也唯一**(同账号的 Personal Team 与付费 Team 冲突);
- **免费 Personal Team 不提供网页管理入口** —— developer.apple.com 的账号菜单里只有付费
  Team(`AI FANTASY`),切不出 Personal Team,因此在门户里删不掉这三个 ID。网上「切到
  Personal Team 删除」的教程只对**付费个人账号**有效。
- 旁证:公司 Team 的 Identifiers 列表里有别人的 `.dev` 变体(如
  `com.pogosoc.pogo.dev`),说明模式没问题,纯粹是 ID 被占。

## 判据(快速确认是不是这个坑)

```bash
# 1. 两 team 的证书是否同一 Apple ID(UID 相同 = 同一账号)
security find-certificate -c "Apple Development" -p | openssl x509 -noout -subject
# OU=<TeamID>, UID=<账号ID>；两个 OU 不同但 UID 相同 → 同账号跨 team 冲突

# 2. 旧 Team 的 profile 是否注册过这些 ID
for f in ~/Library/Developer/Xcode/UserData/Provisioning\ Profiles/*.mobileprovision; do
  security cms -D -i "$f" | plutil -extract Entitlements.application-identifier raw -
done
# 形如 PD2XGB735Y.com.lukasoc.luka.dev → 被 Personal Team 占用
```

## 修法(本地临时,不碰仓库)

`ios/` 是生成物、不入库,直接改它的 pbxproj 绕开被占的 ID:

1. 改 `ios/<App>.xcodeproj/project.pbxproj` 里**三个** `PRODUCT_BUNDLE_IDENTIFIER`
   (Debug/Release 各一份):`com.lukasoc.luka.dev` → `com.lukasoc.luka.devx`。
   **extension 的 ID 必须以 host 的新 ID 为前缀**(`...devx.luka.notiservice` /
   `...devx.luka.widget`),否则签名校验不过。
   Info.plist 里旧的 bundle id URL scheme 可留可改,不参与注册。
2. 构建(命令行传 Team 覆盖全部 target,不需要 prebuild):
   ```bash
   xcodebuild -workspace ios/Luka.xcworkspace -scheme Luka -configuration Debug \
     -destination "id=<40位UDID>" \
     -allowProvisioningUpdates -allowProvisioningDeviceRegistration \
     -derivedDataPath ios/build \
     DEVELOPMENT_TEAM=39CFYH6W55 CODE_SIGN_STYLE=Automatic build
   ```
3. iOS 16 设备装包(devicectl 不支持 iOS 16;Xcode Devices GUI 也行):
   ```bash
   brew install ios-deploy
   ios-deploy --bundle ios/build/Build/Products/Debug-iphoneos/Luka.app \
     --id <40位UDID> --no-wifi
   ```
4. 验证签名身份:
   ```bash
   codesign -d --entitlements - <App>.app
   # application-identifier = 39CFYH6W55.com.lukasoc.luka.devx ✓
   ```

## 注意

- **改 pbxproj 是本地临时手段**:下次 `expo prebuild --clean` 会还原成
  `com.lukasoc.luka.dev`,冲突复现。
- 根治路径(未实测):等免费 profile 过期(7 天)后 App ID 是否释放,或把个人账号升为付费
  后在门户删除旧 ID。**09-21 之后再试改回原 ID** 并记录结果。
- 公司 Team 的证书首次装真机后,手机上要「设置 → 通用 → VPN与设备管理 → 信任」。

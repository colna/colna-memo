---
title: Ad Hoc 内测 IPA 装不上：设备不在描述文件里（无法验证其完整性）
date: 2026-09-24
tags: troubleshooting, ios, ipa, adhoc, provisioning, sitin-rn, luka
---

# Ad Hoc 内测 IPA 装不上：设备不在描述文件里

## 现象

iPhone 上安装本机/云端出的内测 IPA，系统弹：

> 无法安装"Luka"
> 无法安装此 App，因为无法验证其完整性。

**这不是 IPA 损坏，也不是签名坏了** —— 是这台设备的 UDID 不在 IPA 内嵌的 Ad Hoc
描述文件（`ProvisionedDevices`）里。Ad Hoc 包只允许装在登记过的设备上，iOS 校验失败
时统一报「无法验证其完整性」。同一个包在登记过的设备（如 iPad）上装就正常。

## 判据（3 条命令实锤）

```bash
# 1) 取设备的 UDID
xcrun devicectl list devices          # 物理设备看 UDID 列（新式 8-8-8 格式）

# 2) 解包，列出 IPA 内嵌描述文件登记了哪些设备
TMPD=$(mktemp -d); unzip -q apps/luka/ios/build/export/Luka.ipa -d "$TMPD"
security cms -D -i "$TMPD/Payload/Luka.app/embedded.mobileprovision" -o /tmp/p.plist
python3 -c "import plistlib;p=plistlib.load(open('/tmp/p.plist','rb'));print(p['Name'],p['ExpirationDate']);print('\n'.join(p.get('ProvisionedDevices',[])))"

# 3)（可选）本机缓存 profile 里有没有它 —— 判断「从未注册」还是「注册过但 profile 旧」
for f in ~/Library/Developer/Xcode/UserData/Provisioning\ Profiles/*.mobileprovision; do
  security cms -D -i "$f" 2>/dev/null | grep -q "<UDID>" && echo "FOUND: $f"
done
```

## 修法

1. 手机 USB 连 Mac，信任此电脑，开开发者模式（iOS 16+：设置 → 隐私与安全性 → 开发者模式）。
2. 用一次真机构建触发注册（同 `ios-appid-personal-team-conflict.md` 的命令）：

   ```bash
   xcodebuild -workspace ios/Luka.xcworkspace -scheme Luka -configuration Debug \
     -destination "id=<设备UDID>" \
     -allowProvisioningUpdates -allowProvisioningDeviceRegistration \
     -derivedDataPath ios/build \
     DEVELOPMENT_TEAM=39CFYH6W55 CODE_SIGN_STYLE=Automatic build
   ```

   Xcode 会把设备注册进公司 Team（39CFYH6W55）并刷新/新建相关描述文件。
3. **重新出包**（复用工程几分钟）：`build-ipa.sh luka test preview`，再按
   `sitin-rn-local-ipa-verification.md` 核验新 IPA 内嵌 profile 是否含该 UDID。
4. 若 Xcode 没自动把新设备刷进 **Ad Hoc** 描述文件（它只管了 Development 档），去
   developer.apple.com → Profiles revoke 掉旧的 `iOS Team Ad Hoc Provisioning Profile:
   com.presence.gracechat`，下一次 archive/export 会自动重建（09-18 已验证这条路会
   自动新建 Ad Hoc profile）。

## 注意

- 房号 `com.presence.gracechat` 的 Ad Hoc profile 是全产线共用的，设备名单里混着别的
  产品机（71 台）；**不要按名字猜，直接列 UDID**。
- 给测试同学发包前先收 UDID 注册——`ios-appid-personal-team-conflict.md` 2026-09-18
  那节已提过同一件事。
- 设备不在名单时**构建与导出全程不报错**，只有装机那一刻才失败；只能靠装机或列
  profile 发现。

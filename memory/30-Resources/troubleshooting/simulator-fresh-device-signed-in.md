---
title: 模拟器：新建设备拿到「已登录」会话（拷 Keychain + 数据容器）
date: 2026-09-17
tags: [simulator, ios, testing, secure-store]
---

**场景**：要在一个**新建 / 重置**的 iOS 模拟器上验证已登录后的页面（Me 页、设置……），
但真账号登录 / Apple 登录在模拟器上不现实，又不想动别人正在用的那台设备。

**结论**：会话（token）在 `expo-secure-store` → 模拟器 **Keychain** 里，**不在 app 数据容器**；
只拷数据容器不够，keychain 也要一起拷。三步：装包 → 拷 keychain（目标先关机）→ 拷数据容器。

## 步骤

```sh
SRC=<有已登录会话的设备 UDID>; DST=<新设备 UDID>; APP=com.lukasoc.luka.devx

# 0) 把源设备上的 .app 装到目标
xcrun simctl install $DST "$(xcrun simctl get_app_container $SRC $APP app)"

# 1) keychain：目标必须先 shutdown（sqlite 别在开着时覆盖）
xcrun simctl shutdown $DST
KEY=~/Library/Developer/CoreSimulator/Devices
cp -f $KEY/$SRC/data/Library/Keychains/keychain-2-debug.db* $KEY/$DST/data/Library/Keychains/
xcrun simctl boot $DST && xcrun simctl bootstatus $DST -b

# 2) 数据容器（AsyncStorage / 缓存 / dev-client 设置）
xcrun simctl terminate $DST $APP 2>/dev/null
rsync -a --delete "$(xcrun simctl get_app_container $SRC $APP data)/" \
                  "$(xcrun simctl get_app_container $DST $APP data)/"

# 3) 起 app（dev client 直接带 Metro 地址，桌面 Metro 在 8081）
xcrun simctl openurl $DST "luka://expo-development-client/?url=http%3A%2F%2Flocalhost%3A8081"
```

## 坑

- **只拷数据容器不够**：token 在 Keychain（`keychain-2-debug.db`，要连 `-wal` / `-shm` 一起拷，
  否则最近的写入丢）。
- **通知授权是设备级的**：拷完第一次启动还会弹「想给你发送通知」，点掉即可（不是失败）。
- 导航用深链（`luka://me`）比盲点坐标稳。Simulator 的 AXGroup（窗口内设备屏区域）**size 比真实屏
  小约 7%**，按它算的坐标在屏幕偏下处会点空（中上部一般够用，要精确得先用截图标定真实屏矩形）。

---
title: PWA 按 app 隐藏 Snapchat · 回归测试范围
date: 2026-08-24
tags: [pwa, snapchat, 回归测试, savvy]
---

# PWA 按 app 隐藏 Snapchat · 回归测试范围

需求:`savvy`/`savvy_android` 端**临时隐藏** Snapchat 社媒授权入口(端暂不支持;端支持后从 `HIDE_SNAPCHAT_APPS` 移除即放开)。`haven_pwa` 及其它 app 不隐藏。
实现(sp-v2,commit 3c159b051 / 66758fabb):端能力 `getApkName` → `appConfigStore.hideSnapchat` → gate 三处入口。

## 一、appname 判定(数据源维度)
| appname 来源 | 值 | 预期 |
|---|---|---|
| getApkName(APK) | `savvy` | 隐藏 Snapchat |
| getApkName(APK) | `savvy_android` | 隐藏 Snapchat |
| getApkName(APK) | `haven_pwa` | **不隐藏**(照常显示) |
| getApkName(APK) | 其它(gracechat/luma/romi…) | 不隐藏 |
| 非 APK(H5 浏览器) | getApkName 返回 "" → 回退 URL `?app_name=` / localStorage | 命中 savvy* 才隐藏,否则不隐藏 |

## 二、三处入口(隐藏 app 下都应无 Snapchat、只剩 Instagram)
1. **授权抽屉 `showSocialAuthDrawer`**
   - 自动弹(有 expired 断连时冷启动弹)→ 抽屉里无 Snapchat 行
   - 设置页「Link Social Media」force 手动入口 → 无 Snapchat 行
2. **Home Social Connect 卡**(该平台有积压 CE 交换订单且未登录时出现)→ 无 Snapchat 卡
3. **Task 列表一次性任务「Authorize Snapchat」**(`BindSnapchatAccount`)→ 不出现

## 三、必须回归的非隐藏路径
- **haven_pwa / 其它 app**:上述三处 Snapchat **正常显示**,且可正常授权/登录/完成任务。
- **Instagram 全程不受影响**(隐藏 app 下):IG 授权抽屉、Social Connect 卡、Task 任务、登录/重连照常。
- **进聊天放行**(authBlock):隐藏 Snapchat 不应影响「授权任一社媒即放行」逻辑——savvy 用户授权 IG 后能正常进聊天。

## 四、时序 / 边界
- **冷启动首帧**:`initAppName` 是异步(getApkName 走 bridge),拉到前 `hideSnapchat` 默认 false → 首帧可能短暂显示 Snapchat,拉到后 reactive 收起。**重点回归**:savvy 端进入首页/授权弹窗时 Snapchat 是否稳定不显示(必要时确认拉取时机是否够早)。
- **刷新 / 重进 App**:每次都应正确隐藏(initAppName 每次 mount 拉)。
- **历史已授权 Snapchat 的 savvy 用户**(若存在):隐藏入口后,其已授权/登录态、聊天放行是否异常(入口隐藏≠清授权态)。
- **非 APK 调试**:H5 用 `?app_name=savvy` 可复现隐藏(走 URL 兜底)。

## 五、快速验证点
- savvy 包:首页 Task 无「Authorize Snapchat」;有积压订单也不出 Snapchat Social Connect 卡;设置页 Link Social Media 只有 Instagram。
- haven_pwa 包:以上 Snapchat 全部照常。

## 六、H5 web 环境(2026-08-24 补充 commit cf9b36d2d)
- **H5 一律不隐藏 Snapchat**:`hideSnapchat` 只由真机 `getApkName` 决定;H5 下 getApkName 返 "" → hideSnapchat=false。即使 URL `?app_name=savvy` 也不隐藏(URL 兜底只用于展示/tracking,不触发隐藏)。
- **H5 点任意社媒授权入口 → 弹「下载 App」弹窗**(三处一致):
  - 授权抽屉(showInsModal / authorizeOrLogin):H5 本就弹 `showApkDownloadModal` ✓
  - Task 一次性授权任务(authorizeOrLogin):同上 ✓
  - Home Social Connect 卡(openSocialProxyWebView):本次加 H5 判断,非 APK 弹 `showApkDownloadModal("social_connect_card")`(原直调 bridge 在 H5 静默无反应)
- **回归点**:H5 用 `?app_name=savvy` 打开 → Snapchat 仍显示(不隐藏);点 IG/Snapchat 授权(抽屉/任务/Social Connect 卡)都弹下载 App 弹窗,不再静默无反应。

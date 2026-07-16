---
title: Social-Proxy 容器 App:多社媒 WebView 挂机 + 内存控制方案
date: 2026-07-16
tags: [troubleshooting, social-proxy, android, webview, 架构, 内存]
---

# 多社媒 WebView 挂机 + 内存控制

## 需求

Social-Proxy 容器 App:**单账号、多社媒**(现 IG + Snapchat,以后更多),需要"多开 WebView"跑自动化脚本,但不想占太多内存。
命令由 **sp-server 下发**:某社媒有命令就激活、没命令就待命、长期没有就不定期检查一次。

## 结论(定稿架构)

**命令驱动 + 按需激活 + 空闲释放 + FCM 门铃 + WorkManager 兜底。空闲时活 WebView = 0。**

每个平台三态,全局串行(M=1):

```
COLD(无 WebView,0 内存)
  ↑ 空闲超时 T_idle           ↓ 有命令
HOT/待命(WebView 活着等后续命令) ⇄ ACTIVE(执行中)
```

流程:
1. sp-server 有命令 → 发 **FCM high-priority**(**只当门铃,不塞命令内容**)
2. App 收到 → 校验 `getPriority()==PRIORITY_HIGH` → **立刻**起前台服务(FGS)
3. FGS 起来后再拉 sp-server 命令队列 → 激活对应平台 WebView → `evaluateJavascript` 执行
4. 该平台无命令 → **热待命 T_idle(建议 60~120s)**,期间新命令直接复用,省冷启
5. 超时 → `destroy()`;全部空闲 → **停 FGS** → 内存回 0
6. 兜底:**WorkManager periodic**(最小 15min)拉一次,防 FCM 丢包

内存表现(**与平台数 N 彻底解耦**):长期空闲 0 / 热待命 1 / 执行中 1。

## 官方硬约束(这些否掉了别的方案)

| 约束 | 后果 | 出处 |
|---|---|---|
| Chrome 88+ 对**隐藏页面**链式 JS timer **重度节流** | ❌ 不能用 WebView 内 `setInterval` 轮询;**调度权必须在原生** | [Chrome 88 timer throttling](https://developer.chrome.com/blog/timer-throttling-in-chrome-88) |
| Android 13+ cached 进程「limited or no execution time」 | ❌ 一退后台进程被冻结,WebView 停摆 → 必须 FGS | [bg-work-restrictions](https://developer.android.com/develop/background-work/background-tasks/bg-work-restrictions) |
| Android 15+ `dataSync`/`mediaProcessing` FGS **24h 限跑 6h**,超时 `onTimeout()` | ⚠️ 24/7 常驻 FGS 会被掐 → **按需短开 FGS 正好避开** | [FGS timeouts](https://developer.android.com/develop/background-work/services/fgs/timeout) |
| Android 12+ 禁后台启 FGS,**收到 high-priority FCM 是明确豁免** | ✅ 唯一正解叫醒方式;**窗口很短**,慢了抛 `ForegroundServiceStartNotAllowedException` | [restrictions-bg-start](https://developer.android.com/develop/background-work/services/fgs/restrictions-bg-start) |
| FCM high priority 可唤醒 Doze,给临时网络 + partial wakelock | ✅ 门铃机制成立 | [FCM message priority](https://firebase.google.com/docs/cloud-messaging/android-message-priority) |

## 关键坑

1. **FCM 只当门铃,命令从 server 拉** —— FCM 不保证送达、有大小限制、可能被 collapse
2. **收到 FCM 立刻起 FGS**,起 FGS 前**别做任何网络请求**(豁免窗口短)
3. **别依赖 WebView 里的 JS timer** —— 隐藏页被节流;原生 `evaluateJavascript` 主动驱动天然免疫
4. 热待命期挂 `setRendererPriorityPolicy(RENDERER_PRIORITY_BOUND, waivedWhenNotVisible=true)` + 实现 `onRenderProcessGone`:系统内存紧张随便杀,**登录态在 cookie,重建 `loadUrl` 就回来**。⚠️ 改了 renderer 优先级**必须**配 termination handling,否则崩
5. renderer 被杀后**该 WebView 不可复用**,必须 `removeView` + `destroy()` + 新建
6. 别滥发 high-priority FCM(Google 会降权),只在真有命令时发
7. WebView 需 attach 到 window 才可靠执行(1×1 / offscreen 容器),注意有站点检测 `visibilityState`

## 被否掉的方案(别再走回头路)

- **❌ Profile API(androidx.webkit)**:是给「**同平台多账号**」(2 个 IG 号 cookie 打架)用的。本场景**单账号多社媒 = 不同域名,cookie 天然按域隔离**,用不上,徒增复杂度和 WebView 版本门槛。**以后若要同平台多号才引入**。
- **❌ 多进程 + `setDataDirectorySuffix()`**:官方建议「只在一个进程用 WebView」;多进程内存翻倍。Android 9+ 多进程还强制每进程唯一 data dir suffix。
- **❌ N 个平台常驻 N 个活 WebView**:内存 ∝ N 且必被系统杀。
- **❌ 固定周期轮询每个平台**(`T = ceil(N/M) × t`):命令驱动模型下完全没必要,且撞 FGS 6h 上限。仅当真要"主动查新消息"才考虑。

## 待定参数

- `T_idle` = 内存↔延迟旋钮(冷启 `loadUrl` + 恢复登录态约 10~20s;命令成串来则取 60~120s)
- 命令频率量级 → 若基本不断,热待命常开,内存 ≈ 1 个 WebView 恒定(仍远好于常驻 N 个)

## 相关

- [[sitin-next-pwa-chat-tim]]、[[mobile-keyboard-and-viewport]](同为 APK WebView 环境踩坑)

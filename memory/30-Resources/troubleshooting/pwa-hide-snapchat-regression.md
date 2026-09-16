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

## 七、2026-09-16 放开 savvy_android(分支 feat/pwa-open-snapchat-live-savvy-android)

- **新状态**:白名单扩为 `haven_pwa + savvy_android` —— UA `savvy_android` 的 Snapchat 四处门控 + Live 开播入口(EarnModule)全部放开;**savvy iOS 仍隐藏**,H5 / 其它 app 不变。
- **实现**:`useUA().isSavvyAndroid`(UA token `savvy_android`);Snapchat 门控拆成 `hideSnapchat = !isHavenPwa && !isSavvyAndroid`(4 文件);`LiveAction.canGoLiveNatively = (isHavenPwa || isSavvyAndroid) && hasNativeMethod("startHostLiveStream")`。
- **直播注意**:UA 放开后仍靠端能力 `startHostLiveStream` 存在性兜底,老 Android 端(未实现)不会出入口,不担心点了开不了。
- **新增回归点**:① savvy_android 三处 Snapchat 入口正常显示、可授权/登录/完成任务;② Live 页 savvy_android 出 EarnModule(端已实现能力时);③ 老 Android 端不出 Live 入口;④ savvy iOS / H5 / 其它 app 维持原隐藏行为。
- **落地**:2026-09-16 已合入 `release/test-pwa`(merge `aa9df14fc`,随该线 proto 升级到 release/test 最新 `21c01821`)。

## 八、2026-09-16 复核:savvy_android 直播卡片不显示的原因(真机 UA 无 token)

排查「savvy_android 直播卡片不显示」时,拉了 `presence-io/savvy-android`(女端 APP,default `master`)源码核对,**UA 判定在 savvy 端永远为 false**:

- **UA 无 app 后缀**:savvy 端 WebView UA 是写死的纯 Chrome UA —— `app/src/main/java/com/savvy/app/web/PwaWebConfig.kt` `USER_AGENT`,从不追加 app 名(宿主 `PWAWebViewFragment.kt:350` 与预热 `PwaPreheater.kt:156` 共用)。端标识另走 URL `?app_name=savvy`(`PWAWebViewFragment.kt:707`、`PwaPreheater.kt:165`,值来自 `BuildConfig.APP_NAME="savvy"`,app/build.gradle.kts:42)与 `app_name` 网络头;
- 所以 `parseUA()` 拿不到 `savvy_android` 这个 token —— 该 token 只在需求里出现过,真机 UA 从无(历史 `getApkName` 在 savvy 返空同源);
- **端能力名也不匹配**:savvy 的 `app/src/main/java/com/savvy/app/bridge/PwaNativeInterface.kt` 只有 `startLiveNative`(腾讯 LiveActivity 原生直播,`PwaNativeInterface.kt:533`)与 `syncLiveState`(:518),**没有 `startHostLiveStream`** → `hasNativeMethod("startHostLiveStream")` 恒 false。
- **结论**:`LiveAction.canGoLiveNatively = (isHavenPwa || isSavvyAndroid) && hasNativeMethod("startHostLiveStream")` 在 savvy_android 上两道门都过不了,走的是「老端未实现开播能力 → 不出入口」的兜底,属预期行为而非 bug。
- **若要在 savvy Android 放开入口**,需端侧二选一/都对:① 在 UA 追加 app 名,或 PWA 改按 `?app_name=` 识别;② 实现 `startHostLiveStream` bridge 方法(或 PWA 侧改判 `startLiveNative` 并对齐参数契约)。
- **教训**:「UA 追加 app 名」只是 haven 端的实现(`... ${NetworkConf.appName}`),不能外推到同产线其它 App;写 app 识别分支前先要真机 UA 原文。

## 九、2026-09-16 复核补充:savvy develop 已支持 + 各环境 PWA 部署矩阵(第八节仅对 master 成立)

第八节结论基于 `savvy-android` 默认分支 **master(release/2.2 线)**,只看它不完整。**develop(下一版,2026-09-16 tip `a89de1c2`)两个门都已具备**:

- **UA**:`PwaWebConfig.USER_AGENT` 末尾追加 `savvy_android` —— commit `3ed7cd5 fix: add Savvy identifier to WebView user agent`(2026-08-25);
- **端能力**:`PwaNativeInterface.kt:805 @JavascriptInterface startHostLiveStream`(走 `com.savvy.livekit.bridge.LiveWebBridge`,随 `58bab0a` 于 2026-09-15 从 GraceChat beauty-live 1:1 迁移,`tuilivekit`/`livekit-biz` 模块一并引入);另有 `isSupportFeature("hostLive")=true`(注释写明「Web 按此决定是否展示 Go Live 入口」)。
- **master/release 2.2(线上版本)两者都没有** —— 所以现网 savvy 包不可能出该入口,分支改动只对 develop 之后的新包生效。

### 各环境 PWA 部署实测(2026-09-16 17:03,直接拉线上 bundle 验证)

| 域名 | 用途 | LiveAction 门控(压缩后) | 是否含分支 `1d0fdd86b` |
|---|---|---|---|
| app-test.sitin.ai | savvy debug 默认(GlobalConf TEST_A) | `(isHavenPwa\|\|isSavvyAndroid) && hasNativeMethod(...)` | ✅ 已部署(~14:57) |
| app-test2.sitin.ai | DebugActivity 可切 | `isHavenPwa && ...` | ❌ 旧包 |
| app-pre.sitin.ai | 预发 | `isHavenPwa && ...` | ❌ 旧包 |
| app.sitin.ai | release 包默认(生产) | `isHavenPwa && ...` | ❌ 旧包 |

> 验证方法:拉 `service-worker.js` 的 precache 列表 → 找含 `startHostLiveStream` 的 `index-*.js` → grep 门控表达式;main bundle 的 `__vite__mapDeps` 可核对 Live chunk hash。

- **排查结论**:若在 app-test2 / app-pre / app.sitin.ai(或新包之前的缓存 bundle)测试,卡片必不显示,与客户端能力无关。app-test + develop 新包则应满足两道门;再不出就查第三道后端 `LiveCanGoLive.canShow`(主播资格)与 Live 页是否处于 Action 态。
- **客户端缓存因素**:savvy 有 PWA 预热 + `LOAD_CACHE_ELSE_NETWORK` 策略,旧 bundle 可能被缓存复用 —— 验证时冷启动/清缓存。

## 十、2026-09-16 app-test.sitin.ai bundle 复核(用户实测环境)

用户确认在 app-test.sitin.ai 测 savvy_android 直播卡片。直接拉线上文件核对(17:03–17:08):

- `app-test.sitin.ai` 线上 index → `main-CQtZVLWJ.js`(main 内 parseUA 已含 `savvy_android`,`__vite__mapDeps` 指向 Live chunk `index-Btt5Ky_m.js`);
- Live chunk 门控实测(压缩后):`{isHavenPwa:n,isSavvyAndroid:r}=K(),i=t.useMemo(()=>(n||r)&&S("startHostLiveStream"),[n,r])`,`S` = `Gb(e){const t=window.pwaBridge;return"function"==typeof t?.[e]}` → **分支已上线,环境侧没问题**。
- 时间:main last-modified 09-16 06:57 UTC(北京 14:57),index 07:42 UTC(15:42)。
- 对照:app-test2 / app-pre / app.sitin.ai 的 Live chunk 仍是 `isHavenPwa && ...`(旧包)。

**剩下能挡住卡片的**(按概率):① 端包不是 develop 构建 —— `savvy_android` UA 是 savvy develop `3ed7cd5`(8-25)加的,`startHostLiveStream` 是 `58bab0a`(9-15)加的;master/release 2.2/旧 `pwa` 分支都没有;② 浏览器里测(即使 UA 伪装,savvy 无 `window.pwaBridge` → hasNativeMethod 恒 false,设计如此);③ 后端 `LiveCanGoLive` `canShow=false`(主播资格);④ 不在 Live 页 Action(live lounge)态。⑤ 预热/SW 旧 bundle 缓存(app-test 当天 14:57 才部署)。
**真机自检**:`navigator.userAgent`(含 `savvy_android`?)+ `typeof window.pwaBridge.startHostLiveStream`("function"?)。

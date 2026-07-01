---
title: 商汤 SenseMARS 美颜集成 sitin-next PWA 技术方案
date: 2026-07-01
tags: [tech-proposal, sensemars, sensetime, pwa, sitin-next, trtc, video-call, beauty, ar]
---

# 商汤 SenseMARS 美颜集成 sitin-next PWA 技术方案

**作者**:zhangzheng 工作区调研 / 2026-07-01
**目标分支**:`sitin-next feature/pwa`
**目标包**:`packages/app-pwa`
**基础项目**:[shangtang-sdk-demo](README.md)
**可视化版本**:[tech-proposal-pwa-integration.html](tech-proposal-pwa-integration.html)(light 风格 HTML,含彩色 Mermaid 流程图 + 粒子动效,浏览器直接打开)
**下位实现指南**:[implementation-guide-pwa](implementation-guide-pwa.md)(不含选型讨论,直接给正确方案 + package 抽离 + PWA 完整接入代码)

## 零、核心业务价值验证:对方能看到美颜吗?

**能。这是这个方案的核心业务价值,不是加分项。**

美颜有两种做法,咱们方案是第二种:

| 做法 | 对方看不看得到 | 说明 |
|---|---|---|
| ❌ 本地 CSS 滤镜 / 只改 `<video>` 显示 | **看不到** | 上行推流用的还是原始摄像头帧,只是本地渲染多加了一层 filter |
| ✅ 改上行流本身(咱们方案) | **一定看得到** | 商汤处理后的 canvas → `captureStream()` → `MediaStreamTrack` → TRTC 推流,推的就是美颜后的帧 |

### 端到端数据流

```
[主播摄像头] → getUserMedia() → [原始 MediaStream]
                                        │
                                        ↓ 送进商汤 SDK
                              [商汤 SDK 处理:美颜/滤镜/贴纸/美妆]
                                        │
                                        ↓ 渲染到 canvas
                                 [美颜后的 canvas]
                                        │
                                        ├──→ 本地 <video>          ← 主播自己看到美颜 ✅
                                        │
                                        └──→ canvas.captureStream(fps)
                                                     │
                                                     ↓
                                           [美颜后的 MediaStreamTrack]
                                                     │
                                                     ↓ trtc.startLocalVideo({ videoTrack })
                                           [TRTC 上行 → 腾讯服务器 → 对方 TRTC SDK]
                                                     │
                                                     ↓
                                             [对方 <video>]         ← 对方看到美颜 ✅
```

### 三个附加要点

1. **对方零负担**:美颜全在主播这边算,对方拿到的就是"已经美颜好的视频帧"。不需要对方装任何 SDK、不消耗对方 CPU、带宽也不变(推流分辨率 / 码率与现在一致)
2. **对方 App 不用改**:只要主播端 PWA 集成一次,对方无论是 PWA / iOS / Android / Web 端,只要能收 TRTC 流就能看到美颜
3. **主播自拍所见即对方所见**:本地预览和上行流是**同一份 canvas**,主播看到什么效果对方就看到什么效果 —— 避免"我这看着好看,对方看着还是原相机"的落差

### 反过来的边界

- 如果**对方**(男用户)也想给自己加美颜,那对方也得集成一份 SDK。按 heyhru 业务模型,男用户是消费方,通常不需要
- 美颜**只作用在视频流上,音频不受影响**(音变/降噪是另一条链路,腾讯有 `voice-changer` 插件)

---

## 一、结论(TL;DR)

**可以接,技术路径是通的**。

- TRTC v5 官方原生支持自定义 `videoTrack: MediaStreamTrack`(见 `LocalVideoConfig.option.videoTrack`,官方 tutorial `20-advanced-customized-capture-rendering`),商汤 SDK 处理后的 canvas 走 `captureStream()` 即可上行推流
- app-pwa 已有 `useCameraStream` hook 自主管理 `getUserMedia`,已经有 `@mediapipe/tasks-vision` 处理视频帧的能力,技术栈本身兼容
- 底层 `_trtc` 实例 `webCallManager` 已经暴露出来(`this.trtc = this.engine?.getTRTCCloudInstance()?._trtc`),可以绕开 TUICallEngine 高层 `openCamera` 用自定义源

**建议裁剪范围**:只接 **effects(美颜/滤镜/贴纸)+ makeup(11 类细分美妆)+ hair(染发)+ nail(美甲)**,合计 4 类 SDK。**不接** tryon 系(手表/眼镜/戒指/鞋)—— 视频通话场景无意义,且 SDK 体积大。

**推荐先做 P0 MVP**(基础美颜 + 3-5 个滤镜),先验证性能和链路,再决定是否上美妆分品和贴纸。

## 二、上下文

### 目标产品

sitin-next `app-pwa` 是 heyhru 的 PWA 端(H5 女主播端,视频通话主要发生在这里),视频通话走腾讯云 TRTC。当前视频通话没有美颜,女主播上镜前必须自己修图,产品需要**通话内实时美颜/滤镜/贴纸**能力。

### 参考实现

商汤 [shangtang-sdk-demo](README.md) 已经拿到 SenseMARS WebEffects 2.3 授权(license 到 **2026-07-26**),里面 8 类 SDK 全部本地可跑,可作为集成参考。

### 现有栈评估

- **框架**:React 19 + Vite + Zustand(和商汤 demo 的 Vue 3 不同但都是纯浏览器 SDK,不受框架约束)
- **视频通话**:`trtc-sdk-v5 ^5.15.2` + `tuicall-engine-webrtc ^3.1.7`
- **已有帧处理能力**:`@mediapipe/tasks-vision ^0.10.32`(说明已经能处理 WASM + WebGL 视频帧,商汤 SDK 是同类)
- **相机流管理**:`hooks/useCameraStream.ts` 已自主 `getUserMedia` + 共享 stream 单例(为 Live/MockCall 场景),视频通话时目前走 TUICallEngine `openCamera(viewId)` 高层封装

## 三、方案对比

TRTC v5 官方内置两个美颜插件 + 支持自定义视频源,合计三条路径。

### 路径 A:TRTC 官方 `BasicBeauty` 插件

- **能力**:magnitude / brightness / ruddy 3 项(基础美颜)
- **成本**:TRTC v5 内置,不用商汤,不用额外授权
- **短板**:能力单一,没有细分美妆、没有滤镜/贴纸、没有 AR 试戴

### 路径 B:TRTC 官方 `Beauty` 高级插件

- **能力**:whiten / dermabrasion / lift / shave / eye / chin + `Effect[]` 滤镜/贴纸
- **成本**:需要腾讯单独购买(要 `sdkAppId + userId + userSig` 授权,是腾讯的美颜商务包)
- **短板**:能力比商汤薄(没有 11 类细分美妆、没有染发/美甲、没有 AR 试戴)

### 路径 C:自定义 `videoTrack` + 商汤 SDK(**推荐**)

- **能力**:商汤 4 类 SDK 全部特效
  - **effects**(基础美颜 / 美形 / 微整形 / 滤镜 / 贴纸 / 整妆 / 背景虚化)
  - **makeup**(口红 / 唇线 / 眼影 / 眼线 / 眼印 / 美瞳 / 睫毛 / 腮红 / 修容 / 粉底 / 眉毛)
  - **hair**(染发)
  - **nail**(美甲)
- **成本**:商汤 license(已有,到期 2026-07-26,需商务续期 + 谈自己的 OSS 分发路径)
- **技术风险**:见「四、技术设计」

### 结论

**路径 C 是唯一能覆盖产品完整需求的方案**。路径 A 可以作为**降级兜底**(SDK 加载失败 / 弱机时退回 BasicBeauty,不影响通话)。

## 四、技术设计(路径 C)

### 4.1 数据流

```
                                          ┌─── st-ar-effects.js (WASM + WebGL)
                                          │
navigator.mediaDevices.getUserMedia()      │
        │                                  │  【商汤 SDK 处理】
        ↓                                  ↓
   MediaStream ─→ HTMLVideoElement ─→ webar.js (Webar 类)
                                          │
                                          ↓  内部渲染到 tempCanvas
                                          │
                                    HTMLCanvasElement (处理后)
                                          │
                                          ↓  canvas.captureStream(fps)
                                          │
                                    MediaStream (video-only, processed)
                                          │
                                          ↓
                            trtc.startLocalVideo({
                              view: 'local-video',        // 本地预览也用这个
                              publish: true,
                              option: {
                                videoTrack: <processed track>,
                                profile: '480p_1',        // 保持现有推流分辨率
                              }
                            })
                                          │
                                          ↓
                                   TRTC 上行推流给对端
```

### 4.2 关键组件改造

**新增**(4 个文件级新增,估算)

1. `packages/app-pwa/src/services/beautySdkManager.ts` — 商汤 SDK 单例封装
   - `preload()`:登录后 idle 时预下载(和 `preloadCallSDK()` 同一时机)
   - `startBeauty(source: MediaStream, opts): Promise<MediaStreamTrack>`:输入原始摄像头流,返回处理后的 track
   - `updateEffect(type, params)`:切换美颜等级 / 滤镜 / 贴纸
   - `stopBeauty()`:释放 WebGL / WASM,回收 canvas
   - `isAvailable()`:能力探测(iOS Safari 版本、WebGL2、SharedArrayBuffer)
2. `packages/app-pwa/src/services/beautyBridge.ts` — canvas → MediaStreamTrack 桥接
3. `packages/app-pwa/src/hooks/useBeauty.ts` — React 侧订阅美颜状态 + 面板控制
4. `packages/app-pwa/src/components/BeautyPanel.tsx` — 通话中调节 UI

**改造**

1. `packages/app-pwa/src/services/webCallManager.tsx`
   - `startLocalVideo(viewId)` 分叉:
     - 若 `beautySdkManager.isEnabled()`:先 `getUserMedia` → `beautySdkManager.startBeauty(stream)` → 拿到 `processedTrack` → `this.trtc.startLocalVideo({ view: viewId, publish: true, option: { videoTrack: processedTrack, profile: ... }})`
     - 否则:保持现状 `this.engine.openCamera(viewId)`
   - `hangup()` / `handleCallEnd()` 里 `beautySdkManager.stopBeauty()`
   - 前后置切换:自定义流下要走 `useCameraStream` 换 `facingMode` + 重新 `startBeauty`,不能直接用 TRTC 的 `switchCamera`
2. `packages/app-pwa/vite.config.ts`
   - 商汤 SDK js/wasm 放 `public/sdk/{effects,makeup,hair,nail}/` 走静态托管
   - 或走 CDN(参考商汤 demo `.env.prod` 的阿里 OSS 模式,但要谈自己的 OSS 路径)
3. `packages/app-pwa/index.html` / SW
   - Service Worker 里 `workbox-precaching` 排除 sdk/*.wasm(体积大,懒加载)
4. `packages/app-pwa/public/sdk/*/WebAR.lic`
   - 每个 SDK 目录一份 license(商汤规定,4 类目录 4 份)

### 4.3 SDK 加载策略

商汤单个 SDK ≈ 2-5MB(js + wasm + data),4 类合计 15-20MB。**不能首屏加载**。

策略(和现有 `preloadCallSDK` 一致):
- 登录后 `requestIdleCallback` 里**只预热 effects**(最常用)
- makeup / hair / nail **懒加载**:用户点击「细节美妆」时才 `import()`
- 通话开始时如果 SDK 未加载完,先降级到 BasicBeauty 上行,后台继续加载,加载完再热切换 `updateLocalVideo({ option: { videoTrack: newTrack }})`

### 4.4 License 管理

- **dev**:`public/sdk/*/WebAR.lic`(直接放本地,商汤 demo 就是这么用的)
- **test / prod**:走自有 OSS(**需要商汤商务开通**,现在 demo 的 OSS 路径 `cdf-tryon.oss-cn-hangzhou.aliyuncs.com/sdk/softsugar/*` 是 softsugar 客户的,咱们没权)
- 代码里 makeup/hair/nail 系走 `checkLicenseFromPath(sdkPath)` 显式校验,effects/tryon 系隐式校验
- **续期**:到期前 7 天(2026-07-19)找许佳续,同步升级到 2.4+ 如果有

### 4.5 性能与降级

**风险点**

| 风险 | 场景 | 缓解 |
|---|---|---|
| iOS Safari 15- 不支持 `canvas.captureStream` | 老 iPhone / iOS 14 | `TRTC.isSupported()` 之外加 `HTMLCanvasElement.prototype.captureStream` 探测,不支持自动 BasicBeauty |
| 双 SDK(商汤 wasm + TRTC webrtc)并行掉帧 | 低端 Android / 弱机 | 帧率检测 <20fps 持续 3s → 自动降到 BasicBeauty + Toast 提示 |
| 电池 / 发热 | 长通话(>15min) | 通话时长阈值 + 环境温度探测(实验),超阈值降级 |
| 前后置切换卡顿 | 用户切镜头 | 自定义流下切换要 stop 老 track → 换 `facingMode` 重新 `getUserMedia` → 商汤 SDK reinit,预计 300-500ms 黑屏,加过渡动画 |
| SharedArrayBuffer 未启用 | eyeglasses 多线程版依赖(本次不接) | 不受影响,普通 SDK 单线程 |

**降级链**:自定义商汤流 → BasicBeauty(TRTC 官方) → 裸流。任何一层失败都能兜底,不影响通话。

### 4.6 兼容 TUICallEngine

`TUICallEngine.accept()` → 内部走 `TRTC.enterRoom`,视频推流由 `TUICallEngine.openCamera(viewId)` 触发。要接自定义流:
- **不能**同时用 `openCamera` 和 `trtc.startLocalVideo` — 前者会覆盖后者
- 方案:在 `accept()` 之后**跳过** `openCamera`,直接 `this.trtc.startLocalVideo({ view, publish: true, option: { videoTrack }})`,view 传本地预览 DOM ID(和 TUICallEngine 用的是同一个 `local-video`)
- 需要 hack 一下 `webCallManager.startLocalVideo` 分叉,**不修改 TUICallEngine 或 TRTC 库本身**

## 五、工作量估算

按 1 前端 + 0.3 后端(license/OSS 商务)+ 0.5 测试 计:

| 阶段 | 交付物 | 前端工时 |
|---|---|---|
| P0(1.5-2 周) | 基础美颜(effects/baseSkinCare)+ 3 个滤镜 + 通话内开关 + 降级链 + 移动端兼容测试 | 8-10 人天 |
| P1(1 周) | 美形 / 微整形 / 更多滤镜 / 贴纸 | 5 人天 |
| P2(1 周) | makeup 11 类细分美妆(口红/眼影/腮红等) | 5 人天 |
| P3(可选,3-5 天) | 染发 hair / 美甲 nail | 3 人天 |

**建议先只交付 P0**,上线后看指标(通话质量、掉线率、GPU 占用、女主播接单意愿变化)再决定 P1+。

## 六、开放问题(需要决策)

1. **License 商务**:是否已经和商汤签了 sitin/heyhru 的独立授权?邮件里给的是 softsugar 客户 lic,直接用可能有法律风险,**建议尽快和许佳确认商务口径**
2. **OSS 分发**:走商汤给的 OSS 还是自建 CDN?走自建需要每次新版 SDK 自己上传/更新
3. **腾讯 vs 商汤**:如果只做基础美颜+简单滤镜,BasicBeauty 免费直接用 ROI 更高;是否有细分美妆/贴纸/AR 的强诉求?
4. **PWA 首屏体积影响**:每加一类 SDK 增加 2-5MB。**懒加载能规避首屏影响**,但用户第一次进美妆面板时会有 2-3s 加载 loading,能否接受?
5. **iOS 兼容底线**:app-pwa 目前最低支持哪个 iOS 版本?iOS 14 不支持 `canvas.captureStream()` 会强制走 BasicBeauty
6. **和现有 `useCameraStream` 单例的关系**:通话时是共用同一份 `getUserMedia` stream(商汤消费一份,预览消费一份),还是新开一份?共用省资源但要处理引用计数

## 七、下一步建议(如果要立项)

1. 与产品对齐 P0 效果清单和 UI 交互(优先级最高)
2. 商汤商务续期 + OSS 分发路径确认
3. 找一台低端 Android + 老 iPhone(iOS 15/16)先做**技术可行性 spike**:2-3 天原型验证性能上限,决定 P0 范围能不能扩到 P1
4. 在 `feature/pwa` 上开新分支(建议 `feat/pwa-beauty-sensetime`)按 `feature/pwa` 的规范提 PR:先 doc 后 code(sitin-next CLAUDE.md 硬规则)

---

## 八、下行美颜:给主播看到的对方加美颜(对方无感知)

> 上行美颜(主播自己)一定要做;下行美颜是**独立可选扩展**,ROI 明显低于上行,建议只做"环境增强"这一档,**不做"改脸"档**。

### 8.1 场景定义

- **上行美颜**(前面 0-7 节):主播摄像头帧被商汤 SDK 处理后再推流 → 对方看到美颜后的主播 ✅ 对方能看到
- **下行美颜**(本节):主播 SDK 收到对方推来的视频后,在**主播端本地**做处理 → 主播 UI 上渲染的是"美颜后的对方",**对方那边的画面/App/CPU/带宽完全不变**,对方无感知

### 8.2 技术可行性:已核验(SDK 一手证据)

**结论:TRTC v5 官方原生支持,不需要任何 hack。**

关键 API:`trtc.getVideoTrack({ userId })` 直接返回远端用户的 `MediaStreamTrack`,可以塞给商汤 SDK 处理。

证据(直接来自本仓库已安装的 SDK 类型定义,不是搜索或记忆):

- **文件**:`sitin-next/node_modules/trtc-sdk-v5/index.d.ts:2216`
- **SDK 版本**:`5.15.3-beta.9`(与 `app-pwa/package.json` 声明的 `^5.15.2` 一致)
- **官方注释原文**:
  > If not passed or passed an empty string, get the local videoTrack.
  > Pass the userId of the remote user to get the remote user's videoTrack.

```ts
getVideoTrack(config?: {
  userId?: string;                    // 传对方 userId 拿远端 track
  streamType?: TRTCStreamType;        // STREAM_TYPE_MAIN(摄像头) / SUB(屏幕共享),都能拿
  processed?: boolean;                // v5.8.2+,拿"SDK 处理后"的 track(虚拟背景/镜像/水印之后)
}): MediaStreamTrack | null;
```

顺带核验到:`getAudioTrack` 同样支持传 remote userId(index.d.ts:2207),意味着"下行音频降噪/变声"技术路径也通,虽然本方案不做。

### 8.3 端到端数据流

```
[对方摄像头] ← 对方 App 里播的是他自己的原相机,零变化
      │
      ↓ TRTC 上行推流
[腾讯 TRTC 服务器]
      │
      ↓ 下行到主播
[主播 TRTC SDK 收到 remoteVideoTrack]
      │
      ↓ trtc.getVideoTrack({ userId: 'male-123' })
      │
[远端原始 MediaStreamTrack]
      │
      ↓ new MediaStream([remoteTrack]) → 隐藏 <video> 播放
      ↓ 每帧抓进 canvas
      │
[商汤 SDK 处理(和上行共享同一个 SDK 单例)]
      │
      ↓
[美颜后的 canvas]
      │
      ↓ 主播 UI 渲染这个 canvas,原始 <video> 隐藏
      │
[主播屏幕:看到"美颜后的对方"] ✅
```

**关键性质**:美颜完全在主播设备本地算,对方那边**播的还是他自己的原相机**,不知道也不需要知道你在美化他。对方 App 不用改、不装 SDK、不消耗对方 CPU、带宽也不变。

### 8.4 工程实现(复用上行 SDK 单例,边际成本很低)

前面 4.2 已设计好 `beautySdkManager`,下行只是把同一个 SDK 单例用在另一路 stream 上:

**新增 API(在 `beautySdkManager` 上扩)**:

```ts
// 输入:任一 remote track;输出:美颜后 canvas 对应的 MediaStreamTrack
startRemoteBeauty(
  remoteTrack: MediaStreamTrack,
  opts: { effects: 'env-only' | 'face' }   // 见 8.5 分级
): Promise<HTMLCanvasElement>;

stopRemoteBeauty(remoteUserId: string): void;
```

**UI 侧改造**(`webCallManager` 里挂 remote 视图那段):

- 原来:`this.trtc.startRemoteVideo({ userId, view: 'remote-video' })` 让 SDK 自动挂 `<video>`
- 改成:拿 `trtc.getVideoTrack({ userId: remoteUserId })` → 塞给 `beautySdkManager.startRemoteBeauty` → 拿到 canvas → 把 remote 视图容器渲染这个 canvas(隐藏 SDK 挂的 `<video>`,或用 `option.view: null` 让 SDK 别挂)

**估算**:+2-3 人天集成到 P1 里(SDK 单例已有,是复用不是新写)。

### 8.5 产品分级建议(**关键**:技术能做 ≠ 产品该做)

按"改不改人脸"分两档,建议分开决策:

| 档位 | 内容 | 商汤 SDK | 建议 | 理由 |
|---|---|---|---|---|
| **A. 环境增强档(推荐做)** | 背景虚化 bokeh、亮度/曝光补偿、时域降噪 | `effects` 的 bokeh / filter | ✅ P1 做 | 男用户环境昏暗/房间乱/光线差 → 让主播看清对方,减少视觉疲劳,续单友好;弱网 480p 下行画质差 → 降噪缓解;**不改脸,不影响主播对对方真实长相的判断** |
| **B. 改脸档(不推荐)** | 磨皮、美白、瘦脸、眼线、唇线、贴纸 | `effects` + `makeup` | ❌ 至多 P2,且只上磨皮/美白 | ①下行是 480p 默认,商汤关键点精度明显下降,眼线/唇线/贴纸这类精细效果基本不能用;②主播看到的对方被美化,和其他场景/线下见面的落差会造成情感判断错位(heyhru 讲主播粘性,这个落差有风险);③双向都开美颜,GPU/CPU 翻倍,弱机掉帧 ×2;④"让男用户顺眼"和"让主播接单"因果链条弱,ROI 不如把资源投在上行 |

**最终建议**:

- **P0** — 只做上行(主播美颜),不做下行(0-7 节的方案不变)
- **P1** — 追加下行 A 档「环境增强」(bokeh + 亮度 + 降噪),对主播实际体验有帮助,又没有"改脸带来的预期落差"风险
- **P2** — 如果产品坚持要下行 B 档「改脸」,只开磨皮/美白两档,不上精细美妆,回避精度问题

### 8.6 还没核验的两点(诚实标注)

写进正式立项前需要补证:

| 断言 | 状态 | 怎么补证 |
|---|---|---|
| 商汤 `effects` 具体包含 bokeh 背景虚化 + 亮度补偿 + 降噪滤镜 | ⏳ 未核验 | 进 `shangtang-sdk-demo/` 看它导出的 effects 清单,或查商汤官方 API |
| 480p 下行流商汤关键点精度会明显下降(眼线/唇线不能用) | ⏳ 经验判断非文档结论 | 查商汤官方"最低分辨率建议",或跑一次 spike 实测 |


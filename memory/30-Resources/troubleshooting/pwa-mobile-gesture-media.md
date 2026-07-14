---
title: 移动端长按手势 + 录音/媒体(getUserMedia/AudioContext)踩坑合集
date: 2026-07-02
tags: [troubleshooting, pwa, frontend, mobile, gesture, audio, react]
---

# 移动端「长按手势 + 录音/媒体」踩坑合集

来源:sitin4.0 前端语音录制 ChatVoiceRecorder / useVoiceRecorder(PR #499,2026-07-01)。
主线是一类共性坑:**手势生命周期被 `await` 打断** + **移动端媒体资源的异步/默认挂起状态**。

## 1. async 事件 handler:await 后 `event.*` 已被回收
- **现象**:`onPointerDown={async e => { await start(); e.currentTarget.setPointerCapture() }}` 报 `Cannot read properties of null (reading 'setPointerCapture')`。
- **根因**:React SyntheticEvent 在 handler **首次 yield(await)后即被回收**,`currentTarget`/`target`/`pointerId`/`clientY` 全失效。
- **修法**:await **之前**把要用的字段读进局部变量,await 后用局部变量。对所有 `onX={async e => {...await...e.*}}` 都成立。

## 2. 按住触发 async 授权,await 期间松手 → setPointerCapture 崩 + 卡录制态
- **现象**:`Failed to execute 'setPointerCapture': No active pointer with the given id is found`;且录音开始却卡在 recording(松手的 pointerup 在 idle 态被 guard 忽略)。
- **根因**:`await getUserMedia`(首次权限弹窗尤慢)期间用户已松手,pointer 已释放。
- **修法**:① `setPointerCapture` 包 try/catch;② 用 `startingRef`(in-flight)+ `pendingUpRef`(await 期间松手标记),await 完成后若 pending 为真直接走松手逻辑(通常 too-short),不进 recording;③ `startingRef` 防重入。

## 3. 长按无延迟:先切 UI 态,媒体后台起
- **现象**:长按语音条要「延迟一下」才进录制态。
- **根因**:`await start()`(getUserMedia,即使已授权也有几十~几百 ms)**之后**才 `setRecState('recording')`。
- **修法**:按下**立即**同步切 UI 态,`start()` 放后台 `.then`;流就绪才真正采集/计时;失败回退 idle;流启动期间松手/上滑用 `pendingUpRef`+`pendingZoneRef` 记意图,就绪后 `finishByZone` 重放。
- **通用**:「按住即进态 + 需 async 资源」不要 `await` 阻塞进态;资源就绪再补副作用,并处理「资源就绪前用户已操作」的竞态(记录意图→就绪后重放)。

## 4. AudioContext 移动端默认 suspended → 波形/音量条不动
- **现象**:录制中声纹一直平的(时域值恒 128,RMS≈0)。
- **根因**:手势链里 await 之后创建的 `new AudioContext()` 常是 `suspended`,AnalyserNode 读到静音。
- **修法**:建 ctx 后 `if (ctx.state === 'suspended') await ctx.resume()`。这是「可视化不动」的头号原因。

## 5. 长按容器里的 `<img>` 弹浏览器图片菜单,盖掉手势
- **现象**:Android Chrome 长按语音条,弹出 `<img>` 的图片上下文菜单(Open/Save/Share),抢走长按手势。
- **修法**:容器 className 加 `[&_img]:pointer-events-none [&_img]:select-none` + `onContextMenu={e=>e.preventDefault()}`;img 无交互不影响 button 的 onClick。iOS 还可能需 `-webkit-touch-callout:none`。头像/贴纸等可长按区的装饰图同理。

## 6. pointer capture 要绑在跨态不卸载的元素
- idle→recording 若换 DOM,capture 会随旧元素卸载而丢失,手指滑出后 move/up 收不到。**capture 绑在整个手势期间持久存在的 root 容器**。

## 7. 自建 media/timer hook 必须有卸载 cleanup;回调式 Promise 要保证 resolve
- 录音中导航离开会泄漏麦克风 track + `setInterval` + 对已卸载组件 setState → `useEffect` 卸载 cleanup(停 recorder + teardown)。
- `recorder.stop()` 在 `inactive` 态抛异常 → `onstop` 不触发 → `await stop()` 的 Promise **永挂** → UI 卡死。加 `state==='inactive'` 早退 + try/catch(catch 里也 `resolve(null)`)。
- `setTimeout`(如 too-short 提示)存 ID、重触发时 clear、卸载 cleanup;`URL.createObjectURL` 用完 revoke(占位日志干脆别建 url)。

## 8. iOS `<video>` 显示 MediaStream 黑屏:paused 不渲染 + 负 z-index 被父层盖死
来源:sitin4.0 响铃弹窗 ReceiveCallModal 本机预览黑屏(2026-07-12,PR #593)。**盲猜了 5 版**(权限/超时/churn/延迟释放)全错,最后靠诊断日志钉死是两层叠加,教训是「别猜,加日志」。
- **排查铁律**:`getUserMedia` 成功 ≠ 在渲染。日志要打全:`srcObject` 是否挂上、`onLoadedData` 是否 fire(`videoWidth>0` 说明帧解出了)、**`video.paused`**、以及元素的 `z-index` / 父层背景。缺一个都可能误判方向。
- **坑 A(paused)**:iOS 对「已挂载 `<video>` 之后再赋 `srcObject`」的 `autoPlay` 常不触发 → 停在 `paused:true`,而 **iOS 不渲染暂停态的 MediaStream** → 黑。修:挂流后主动 `await video.play()`(所有预览统一在 attachStream 里做);`onLoadedData` 里若仍 `paused` 再补一次 `play()` 兜底。`onLoadedData` 触发 ≠ 在播。
- **坑 B(负 z-index)**:背景视频用 `-z-20`、遮罩 `-z-10`,被父容器**不透明**渐变(`bg-gradient`)盖在下面 → 摄像头这层从来没露过(老 bug,paused 修好后才暴露)。修:视频 `z-0`、遮罩 `z-[1]`、内容 `z-[2]`,层叠 = 父渐变(底)→视频→遮罩→内容。负 z 只有在父层透明时才可见。
- 与远端视频黑屏(TUICall startRemoteView 时序 / 5000)是两码事,那条见 [[tuicall-remote-video-5000-race]]。

## 附:UI 布局小坑
- hover/激活放大按钮用 `transform scale`(配固定尺寸)而非改 width/height —— 改尺寸占布局、挤兄弟;transform 不参与布局,原地放大。
- overlay 要「相对整行/屏幕居中」时,`relative` 定位基准要设在**整行容器**上,不是被 flex 挤在中间的子项。

## 验证环境坑
- **app-pwa dev 预览页做登录态交互验证走不通**:未登录约 700ms 重定向 `/onboarding`,headless 抢窗口(500–750ms)来不及;注 mock 登录又被 `useUserInit` 异步覆盖回 NotLogin。→ 登录态后的交互 UI 要**真机登录态实测**(接入 `/chat-detail` 真机验证),或搭脱离 App shell 的独立组件挂载 harness。idle 态静态样式可抢窗口截图。

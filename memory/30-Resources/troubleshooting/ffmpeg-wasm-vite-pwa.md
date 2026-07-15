---
title: ffmpeg.wasm 在 Vite + PWA(app-pwa)里集成的坑
date: 2026-07-14
tags: [ffmpeg, wasm, vite, pwa, sitin-next, app-pwa, audio, transcode]
---

# ffmpeg.wasm 在 Vite + PWA 集成的坑

> **注(2026-07-14)**:sitin-next 语音消息**最终未采用** ffmpeg —— 改用现成 `@heyhru/web-util-media` 的 `convertWebMToWav` 转 **WAV**(零依赖,对齐腾讯 IM 官方 Web SDK 发 WAV 的做法)。下面经验保留备查:将来真需要浏览器端 **AAC** 转码(WAV 体积敏感场景)时参考。

场景:`sitin-next / app-pwa` 用 `@ffmpeg/ffmpeg@0.12` 在浏览器把语音 webm/opus → m4a(AAC),兜底老 Android WebView 录不出 AAC 的情况。以下是踩过的坑 + 修法。

## 1. 依赖与 core 自托管
- 装 `@ffmpeg/ffmpeg` + `@ffmpeg/util`(dependencies);core 用**单线程** `@ffmpeg/core`(免 COOP/COEP 跨源隔离头,多线程 `-mt` 才需要)。
- **core 不走 CDN**:APK WebView 的 CSP 大概率禁外部 → 必须自托管。把 `node_modules/@ffmpeg/core/dist/umd/ffmpeg-core.{js,wasm}` 拷到 `public/ffmpeg/`,`ffmpeg.load({ coreURL, wasmURL })` 用 `toBlobURL(`${import.meta.env.BASE_URL}ffmpeg/...`)`。
- **`@ffmpeg/core` 的 `package.json` 不在 `exports`** → `require.resolve("@ffmpeg/core/package.json")` 报 `ERR_PACKAGE_PATH_NOT_EXPORTED`。改用 `require.resolve("@ffmpeg/core")` 拿入口,再按 `/@ffmpeg/core/` marker 截取包根,拼 `dist/umd`。
- core 30MB **不入 git**:`.gitignore` 排除 `public/ffmpeg/`,写个 `scripts/copy-ffmpeg-core.mjs`,`package.json` 的 dev/build/prod **每个入口都串** `pnpm copy:ffmpeg-core &&`(别只靠 pnpm `pre*` 钩子——turbo 直调 build 时 pre 钩子未必触发)。
- core 放 **dependencies** 而非 devDependencies:Vercel `--prod` 构建不装 devDep,copy 脚本会找不到 core。

## 2. VitePWA 预缓存会吞掉 30MB wasm(最坑)
- `VitePWA({ injectManifest, maximumFileSizeToCacheInBytes: 50MB })` 会把 `public/` 里 ≤50MB 的文件**全部预缓存进 Service Worker** → 30MB 的 `ffmpeg-core.wasm` 被塞进 precache,**PWA 一装/更新就下 30MB**,彻底毁掉"懒加载只在需要时下"。
- 修:`injectManifest.globIgnores` 加 `"**/ffmpeg/**"`。验证:build 后 `grep ffmpeg-core.wasm dist/sw.js` 应无输出。

## 3. worker 与运行时
- `@ffmpeg/ffmpeg` 0.12 内部 `new Worker(new URL('./worker.js', import.meta.url), {type:'module'})`。Vite 5 能正常 emit worker chunk(`dist/assets/worker-*.js`),构建不用额外配置。
- **但 module worker 在老 Android WebView 可能不支持** → 恰恰是"录不出 AAC、需要转码"的老设备最可能挂。务必 `try/catch` 转码,失败**回退原 blob**(webm 部分端能播,总比丢消息强),并在真实老 WebView 上实测,别只信桌面 Chrome。

## 4. 验证清单
- `pnpm lint` + `pnpm build:dev` exit 0;
- `dist/ffmpeg/` 有 js+wasm;`dist/assets/worker-*.js` 存在;
- `grep ffmpeg-core.wasm dist/*.js` 无(SW 未预缓存);
- `vite preview` + `curl /ffmpeg/ffmpeg-core.wasm` = 200 + `content-type: application/wasm`;
- 运行时转码 + 老 WebView 兼容性:**真机测**(桌面新 Chrome 录 AAC 不走转码分支,代表不了)。

相关:语音消息发腾讯 IM 的格式来源见 `[[../../50-Daily/2026-07-14]]` 10:11 工作日志。

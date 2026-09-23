---
title: 本机 build-ipa.sh 出包的两个坑：buildNumber 参数可能无效、产物必须验 bundle
date: 2026-09-21
tags: troubleshooting, ios, ipa, build, sitin-rn, luka
---

# 本机 build-ipa.sh 出包的两个坑

场景：`scripts/build-ipa.sh <app> test preview` 出内测 Ad Hoc 包（正式包流程见 `docs/release-machine.md`）。
两个坑都**不报错**，只能靠核验产物发现。

## 坑 1：`buildNumber` 位置参数可能完全不生效

- 脚本第 5 个参数只作为 `xcodebuild ... CURRENT_PROJECT_VERSION=$IOS_BUILD_NUMBER` 传入；
  而 **CFBundleVersion 的真正来源是 prebuild 时写进 `ios/<App>/Info.plist` 的字面量**，
  来自 `app.config.ts` 里的 `BUILD_NUMBER` 常量。
- luka 是硬编码 `const BUILD_NUMBER = 10000`（`apps/luka/app.config.ts:15`，且与 Android
  `versionCode` 共用）。因此对本包：
  - 复用工程时改第 5 个参数**无效**；
  - **clean prebuild 也无效**（常量写死，`IOS_BUILD_NUMBER` 环境变量根本没被读）。
- 要改必须改 `app.config.ts` 的常量 —— 仓库约定那是「提审前 +1、单独一个 PR 只改这个数」，
  内测包不要动。**命令行传了也不报错**，最容易蒙混过关 → 核验只能看产物。
- 各包实现不同：有的 app 的 `app.config.ts` 会读 `IOS_BUILD_NUMBER`，别按经验套。

## 坑 2：打包期间工作区被并行会话改动 → bundle 是那一刻的快照（静默）

- 复用 `ios/` 时，JS bundle 在 xcodebuild 的 bundle 阶段生成（archive 末尾）。
  若此刻有并行会话在 `git stash` / `git apply` / 编辑源文件，打进去的就是那一瞬间的代码。
- 实例（2026-09-21，luka）：20:09 首打，bundle 写于 20:10:54，而 `discovery-deck.tsx`
  mtime 20:11:05、修复 commit `a759a0692` 20:14 才成型 → 包里**没有** `deckSkipTargetFromCurrent`
  与整卡可划修复。单看 mtime 有歧义（biome `--write` 也会 touch 文件），**必须验产物内容**。
- 修法：打完在产物里 grep 本次新引入的跨模块符号；不一致就重打（复用工程约 2–6 分钟）。

## 坑 3：podspec 新增资源文件（usdz 等）必须重新 pod install 才进包

- 场景（2026-09-23，luka）：`ArkitPlay.podspec` 的 `s.resources = 'Resources/**/*.usdz'`，
  09-22 新增 `LukaSniff.usdz`（代码 `CritterRig.swift` 直接引用）。复用工程打包，
  产物里**没有**它 —— AR sniff 动作会静默失效，构建全程不报错。
- 根因：CocoaPods 在 `pod install` 时把 glob 固化成 Pods.xcodeproj 里的静态文件列表。
  之后源目录新增文件，不重新 pod install 就不会进 build phase。**已存在文件的内容更新
  没问题**（如 LukaRun.usdz 改版就进包了），只有**新增文件**受影响。
- 判据：`grep -c "<新资源名>" apps/<app>/ios/Pods/Pods.xcodeproj/project.pbxproj`
  （0 → 没进工程；pod install 后应为 2）；或直接解包产物看文件在不在。
- 修法：`cd apps/<app> && pod install --project-directory=ios` 后重打。
  **build-ipa.sh 不会替你跑**：它只在 Podfile.lock 与 Manifest.lock 内容不一致时才
  pod install，而新增文件不改 lock，所以跳过 —— 必须手动跑一次。
  注意从 repo 根跑 `pod install --project-directory=apps/<app>/ios` 会报
  `Pathname.new requires a String`（cwd 相关），与 build-ipa.sh 一致在 app 目录下执行。

## 产物核验三件套

```bash
TMPD=$(mktemp -d); unzip -q apps/luka/ios/build/export/Luka.ipa -d "$TMPD"
# 1) 身份 / 版本 / 描述文件
plutil -extract CFBundleIdentifier raw "$TMPD/Payload/Luka.app/Info.plist"
plutil -extract CFBundleVersion raw "$TMPD/Payload/Luka.app/Info.plist"
security cms -D -i "$TMPD/Payload/Luka.app/embedded.mobileprovision" | plutil -extract Name raw -
# 2) 运行时配置（环境 / 域名 / appName）—— 只信 EXConstants.bundle/app.config（JSON）
python3 -c "import json;d=json.load(open('$TMPD/Payload/Luka.app/EXConstants.bundle/app.config'));print(d['extra']['apiBaseUrl'], d['extra']['appName'])"
# 3) 代码是否包含目标改动 —— grep 跨模块符号或用户可见文案
grep -c -a 'deckSkipTargetFromCurrent' "$TMPD/Payload/Luka.app/main.jsbundle"
```

- **别用 `main.jsbundle` grep 域名**：Hermes 的字符串是拼成一整块的表，会命中
  `...lukasap.com...` 这类跨字符串粘连的假阳性（如 `app@lukasap.complexity...`）。
  环境/域名只信 `EXConstants.bundle/app.config`。
- **terser 会内联模块内数值常量**：`const FULL_CARD_SWIPE_BAND = 1_000_000` 不会出现在包里，
  选 grep 目标时优先跨模块函数名 / 用户可见文案（如 `Like post`）。

## 本机出包固定姿势（2026-09 现状）

```bash
export PATH="$HOME/.nvm/versions/node/v24.21.0/bin:$PATH"   # 默认 Node 22 不满足 engines >=24
printf 'n\n' | IOS_TEAM_ID=39CFYH6W55 scripts/build-ipa.sh luka test preview "" 10001
```

- `printf 'n\n'` 回答脚本的「是否 clean prebuild」交互（非 clean；原生依赖变了才 clean）；
  team 用 `IOS_TEAM_ID` 跳过选择，避免 stdin 被抢。
- 构建日志在 `apps/luka/build-logs/build-<时间戳>.log`；脚本**不上传蒲公英**。

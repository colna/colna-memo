---
title: AppCompatActivity 用了非 AppCompat 主题导致进程崩溃
date: 2026-10-30
tags: [android, appcompat, crash, theme, manifest]
---

# AppCompatActivity 用了非 AppCompat 主题导致进程崩溃

## 现象

Activity 一启动就闪退；`adb logcat -b crash` 里是：

```
FATAL EXCEPTION: main
Process: <pkg>:<process>, PID: ...
java.lang.RuntimeException: Unable to start activity ComponentInfo{...}: java.lang.IllegalStateException: You need to use a Theme.AppCompat theme (or descendant) with this activity.
    at androidx.appcompat.app.AppCompatDelegateImpl.createSubDecor(...)
    at androidx.appcompat.app.AppCompatActivity.setContentView(...)
```

## 根因

`AppCompatActivity` 要求窗口主题继承 `Theme.AppCompat`。Manifest 里给该 Activity（或 application）指定了 `android:Theme.Material.*` / `Theme.MaterialComponents` 之外的非 AppCompat 主题时，`setContentView` 走到 `ensureSubDecor()` 直接抛 `IllegalStateException`。

典型触发：项目主主题是 Compose 用的 `android:Theme.Material.NoActionBar`，新增一个继承 `AppCompatActivity` 的 Activity 时直接复用了主主题。

## 修法

1. 最小改动：给该 Activity 单独指定一个 AppCompat 主题（如项目里已有的 `Theme.AppCompat.NoActionBar` 派生主题）。
2. 或把主主题 parent 改为 `Theme.AppCompat.*`（影响全局，Compose 项目一般不需要）。
3. 或让该 Activity 改继承 `ComponentActivity`/`Activity`（若不需要 AppCompat 特性）。

## 排查要点

- 崩溃进程名可能是 `:子进程`，别只看主进程；`adb logcat -b crash -d` 能保留历史崩溃。
- 堆栈第一行 `Unable to start activity` + AppCompat 主题异常，基本可以直接去 Manifest 搜该 Activity 的 `android:theme`。
- Android 12+ splash 配置不背锅，问题在主题 parent，不在 `windowSplashScreenBackground`。

## 实例

`smart-replay-android` 1.1.3：`SocialLoginActivity : AppCompatActivity()`，Manifest 用 `@style/Theme.SmartReply`（parent `android:Theme.Material.NoActionBar`），点 IG Reconnect 拉起登录页即崩；改用 `@style/Theme.SP`（parent `Theme.AppCompat.NoActionBar`）即可。

---
title: 追 SP 线上 createStory 下发与发布（SLS 三行关联 + Story 链接规则）
date: 2026-11-03
tags: troubleshooting, sitin-next, social-proxy, sls, instagram, story
---

# 追 SP 线上 createStory 下发与发布

## 场景

要盘点「某时段内哪些 creator / todo 触发过 IG Story 发布、实际下发了什么参数、最终成没成、Story 链接是什么」。数据只在线上 SLS，库里没有等价结构。

## 关联口径：一个 todoId 串三行

| 行 | 内容 | 用途 |
|---|---|---|
| `[APK] trace=sp:{todoId} method=createStory creator={creatorId} params={JSON}` | 下发入参 | 素材、贴纸、发布方式 |
| `[APK:result] ... data={...}` | APK 返回值 | 成功/失败、`storyPk` / `mediaId` |
| `[APK:storyPublished]` | 发布结果 | 区分 server 发布 / client 发布 |

- 同一 todo 会重试多次（实测 233 个 todo 对应 342 次尝试），**统计 todo 数要 `distinct`，别直接数行**。
- 日志位置：prod `prod-presence-service-log` / `dora-prod-stdout`，service=`social-proxy-server`，namespace `dora-prod`。

## 坑 1：`[APK:dispatch]` 是同一请求的重复行

- **现象**：按 method 关键字统计下发次数会偏大近一倍。
- **根因**：`[APK:dispatch]` 与 `[APK]` 是同一次下发的两条日志。
- **修法**：只保留 `[APK] ...` 行（同时 `creator=` / `params=` 也只在这行），`[APK:dispatch]` 直接过滤掉。

## 坑 2：creator handle 有三个来源，优先级要定

1. `params.link` 里的 `ref` 就是 creator 的 IG handle（抽样与 ProfileSync 一致）；
2. `ref=uid*` 形式拿不到 handle → 用 `[ProfileSync] platform=instagram handle=` 的**最新值**补；
3. 两者都没有的会残留少量（本次 7 个 creator）→ 报告里标「handle 未知」，不要猜。

## Story 链接规则

- 正常：`https://www.instagram.com/stories/{handle}/{storyPk}/`
- client 发布没有 `storyPk` → 取 `mediaId` 下划线前段顶替；这类记录本质是「发布成功但拿不到规范链接」，见 [createstory-missing-session-fake-success](createstory-missing-session-fake-success.md)。

## 查询命令

```bash
sls_query.py --service social-proxy-server --query '"method=createStory"' --minutes 1440 --reverse --raw
```

凭证走 `~/.aliyun/config.json` 的 OAuth profile `zuoyongjie` + `/tmp/pylibs` 里的 `aliyun-log-python-sdk`；OAuth/STS 细节见 [aliyun-cli-oauth-sls](aliyun-cli-oauth-sls.md)。

## 2026-10-30 → 11-02 的口径（存档）

- 近 24h：105 次下发、64 creator、65 todo（14 个 todo 重试 2~4 次）。
- 10-30 00:00 → 11-02 12:00：233 todo / 342 次尝试 / 194 成功 / 39 无返回 / 138 creator；server 发布 142、client 发布 52。
- 按天：10-30 93、10-31 75、**11-01 0**、11-02 65 —— 11-01 整天无 createStory 下发，属异常口径，未查明原因。

## 来源

2026-11-02 Daily 10:51 / 11:05 / 11:10 三条工作日志；导出物 `/tmp/createStory_*.jsonl`、`packages/app-ins-scripts/createStory-report-2026-10-30_2026-11-02.md`（343 KB，untracked）。

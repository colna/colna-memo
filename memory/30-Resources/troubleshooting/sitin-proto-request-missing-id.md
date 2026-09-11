---
title: proto 请求少一个 id：HTTP 200、日志全对，但什么都没发生
date: 2026-09-10
tags: [troubleshooting, protobuf, sitin-rn, backend]
---

# proto 请求少一个 id：HTTP 200、日志全对，但什么都没发生

出处：sitin-rn `apps/luka` 的「删除账号」删不掉（PR #484）。这一类在 protobuf-over-HTTP
的接口上会反复出现。

## 症状的形状

**接口「成功」了，但业务没发生**：HTTP 200、`[接口] resp …` 那行看着正常，只有响应里
的 `isSuccessful=false`（或 `code` 不是 Success）。UI 上就是一句「操作失败」，很容易
被当成后端拒绝。

## 根因

```proto
message DeleteUserRequest { int32 user_id = 1; }
```

客户端发的是空请求体（`client.call(DeleteUserRequest, {}, …)`）。**protobuf 会把等于
默认值的字段省略不编码**，于是服务端读到的是 `user_id = 0` —— 去删 0 号用户，什么也
没删到。

注释里当时写着「服务端从 bearer token 解析当前用户」，那是想当然。

## 判定方法：三条互相印证，一条都不够

1. **proto 里有这个字段** —— 有字段就说明服务端要读它。
2. **找一个已知可用的参照实现**，看它传了什么。当时是 PWA 的
   `deleteUser(userInfo.userId)`（跨仓库找，别只在本仓库里打转）。
3. **看本仓库同类接口的写法** —— 所有「改我自己」的写接口都显式传了 `selfUserId()`，
   只有这一个没传。**同一个仓库里的不一致，本身就是证据。**

## 顺带的两个坑

- **凭证要和 id 一起存。** 冷静期到期后才发的那个删除请求，执行时用户早已登出，
  `user_id` 在 store 里拿不到 —— 所以持久化的不能只有 token。
- **拿不到 id 就别发。** 一个 `user_id=0` 的请求删不掉任何东西，却会回一个看起来正常的
  响应，把「凭证丢了」伪装成「后端拒绝了」。宁可直接返回失败。

## 影响面

这类错通常在**共享引擎**里（`packages/business-*`），也就是说所有派生 app 一起中招。
修在 app 层只救一个包；下沉到共享包要动版本号 + CHANGELOG + 全部消费方，先确认范围
再动手。

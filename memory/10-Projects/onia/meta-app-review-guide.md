---
title: Onia Meta App 申请与审核流程
date: 2026-11-19
tags: project, onia, meta, instagram-api, app-review, business-verification
---

# Onia Meta App 申请与审核流程

目标：让 Onia 的官方通道（Webhook 入站、评论转私信、模式 B 官方代发）从「测试用户内测」走到「对所有专业号用户开放」。
前置判断：**正式开放必须有真实企业主体 + Business Verification + App Review**；测试阶段只需测试用户，不需要审核。

## 0. 前置条件

- 真实 Facebook 个人号（已开 2FA，账号不要是刚注册的新号）
- 真实企业主体：营业执照（公司或个体户）；法定名称、注册地址、电话、网站
- 至少一个真实 Instagram 专业号（商业号/创作者号）用于测试
- 公网 HTTPS 域名：`api.onia.asia`（Webhook 与 OAuth 回调）
- 官网已有隐私政策、服务条款；需确认「数据删除」说明/回调
- 服务端环境变量：`ONIA_META_APP_SECRET`、`ONIA_META_WEBHOOK_VERIFY_TOKEN`、`ONIA_OFFICIAL_TOKEN_KEY`

## 1. 创建 Meta App

1. 打开 https://developers.facebook.com/apps/ → Create app
2. 类型选 **Business**，名称用 `social-proxy` / `Onia`
3. Use case 选 **Manage messaging & content on Instagram**
4. 关联 Business portfolio（没有就创建；名称避免被保留词占用，如用 `Colna` / `Onia Asia`）
5. App settings → Basic：填 App ID / App Secret、隐私政策 URL、服务条款 URL、App 图标、类目
6. 添加 **Instagram** 产品，选择 **Instagram API with Instagram Login**（Unified login，不依赖 Facebook Page）

## 2. 技术接入（开发模式，先跑通）

1. **权限**（Permissions and features）：
   - `instagram_business_basic`
   - `instagram_business_manage_messages`
   - `instagram_business_manage_comments`
   - 可选：`instagram_business_manage_insights`
   - 可选：`Business Asset User Profile Access`（读对方昵称/头像）
2. **OAuth 回调**：Unified login 需要一个 redirect URI。当前服务端尚未实现 OAuth 端点，需先补：
   - `GET /v1/official/instagram/oauth/callback`
   - 流程：跳转 Instagram 授权 → 回调 code → 换 short-lived token → 换 long-lived token → 加密存 `official_channels` → 创建通道
3. **Webhook**：
   - Callback URL：`https://api.onia.asia/v1/official/instagram/webhook`
   - Verify Token：自行生成随机串，写入 `ONIA_META_WEBHOOK_VERIFY_TOKEN`
   - 订阅字段：`messages`、`comments`、`live_comments`、`messaging_postbacks`、`message_reactions`、`messaging_seen`
   - App Secret 写入 `ONIA_META_APP_SECRET`（用于 `X-Hub-Signature-256` 验签）
4. **测试用户**：
   - App Dashboard → App roles（或 Use cases → API setup → Add Instagram testers）
   - 填真实测试 IG 专业号用户名
   - 在 Instagram App → Settings → Apps and websites → Tester invites → 接受
5. **联调验收**：
   - 测试号授权 → 自动/手工创建通道
   - 私信进入 → Webhook 入站 → 会话/草稿 → 人工审核
   - 评论触发 → Private Reply（7 天窗口、一条评论一次）
   - 模式 B：官方代发（24 小时窗口）
   - 失败路径：签名错误 401、未配置 503、窗口外拒绝

## 3. Business Verification（企业验证）

1. Business Suite → Security Centre → **Business verification** → Start verification
2. Use case 选：`App requires access to permissions on Meta for Developers`
3. 填写：
   - 法定名称（与营业执照一致，中英文对应）
   - 注册地址、电话、官网（`https://onia.asia`）
   - 域名邮箱（如 `support@onia.asia`，比个人邮箱更容易过）
4. 上传营业执照扫描件；可能需要地址证明（水电单/银行对账单）
5. 可能要求域名验证：按提示加 DNS TXT 或上传 HTML 验证文件
6. 提交后通常 1–5 个工作日；被拒可按提示补件，不要伪造材料

## 4. Tech Provider

App Dashboard 会提示 **Become a Tech Provider**。Onia 要访问其他商家的 Instagram 数据，提交 App Review 前需要完成这一步（按向导填主体、用例、合规信息）。

## 5. App Review（权限高级访问）

1. App Dashboard → App Review → Permissions and features
2. 对每个权限点 **Request advanced access**：
   - `instagram_business_basic`
   - `instagram_business_manage_messages`
   - `instagram_business_manage_comments`
3. 每次申请填写：
   - 英文用途说明：用户如何授权、消息如何进入、人工如何审核/接管、失败如何处理
   - 录屏（真实操作，不要剪辑造假）：登录 → 授权 → 收私信 → 生成草稿 → 人工批准 → 发送；评论 → 私信
   - 隐私政策链接（含数据用途、存储、删除）
4. 提交后通常 1–2 周，可能补件；通过后权限对所有用户生效
5. 全部通过后，App 右上角 Publish（从 Development 切 Live）

## 6. 上线前检查

- 生产 `.env` 已配置三个 Meta 变量，重启服务端
- Webhook 验签失败路径已测；回调延迟/重试可接受
- Token 加密存储、过期刷新/重授权流程
- 数据删除请求入口（Meta 要求 Data Deletion URL）+ 隐私政策一致
- 速率限制、错误日志、告警
- 官网 Pro 的「官方触发」文案与实际能力一致（Meta 审核通过后再宣传）
- 商业条款：官方触发只对专业号；个人号仍走设备通道

## 7. 常见被拒原因

- 录屏没展示完整授权流程，或功能与申请权限不符
- 没有隐私政策/数据删除说明
- 使用测试数据/假用户演示
- App 名称、图标、类目与官网不一致
- 未完成 Business Verification / Tech Provider
- 刚注册账号、频繁改 App 设置触发风控

## 8. Onia 当前缺口（2026-11-19）

- 服务端缺少 Instagram OAuth 授权端点（当前手工填 token，仅适合内测/运营代配）
- 生产 `.env` 未配置 Meta 三个变量
- 数据删除回调未实现
- 官网条款/隐私已存在，需补「Instagram 数据删除」说明
- 其余：Webhook、通道管理、评论规则、模式 A/B、额度与审计已实现并部署

## 9. 时间与成本

- 无申请费；成本是真实主体、域名、邮箱与人力
- Business Verification：通常 1–5 个工作日
- App Review：通常 1–2 周（可能更久）
- 期间可继续用测试用户内测，不影响设备通道售卖

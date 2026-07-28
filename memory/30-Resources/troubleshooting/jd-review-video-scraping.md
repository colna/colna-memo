---
title: 京东商品「评价测评视频」抓取实测
date: 2026-07-28
tags: [爬虫, 京东, 反爬, playwright, 报价]
---

# 京东商品「评价测评视频」抓取实测(2026-07-28）

需求:输入京东商品链接 → 抓取评价区「测评视频/买家秀视频」,有多少抓多少 → 封装 API。用于技术报价。

## 接口层实测(curl 裸抓)

| 目标 | 接口 | 免签名裸抓 | 结果 |
|------|------|:---:|------|
| 短链还原 SKU | `3.cn/xxx?jkl=@口令@` | ❌ | curl 直接 302 跳 `www.jd.com` 首页,需 App/H5/浏览器环境 |
| 买家秀**图片** | `club.jd.com/discussion/getProductPageImageCommentList.action?productId=SKU&page=N` | ✅ | 能抓,返回 imgList(**只有图片 mediaType=1**)、imgCommentCount 可翻页。**返回 GBK 编码** |
| 评价文字 | `club.jd.com/comment/productPageComments.action` | ❌ | 返回「系统繁忙」 |
| **视频** | `videoListByProductId.action` / `api.m.jd.com?functionId=...` | ❌ | 「系统繁忙」/ 空,需 **h5st 签名** 或登录 cookie |

**结论**:免签名只能拿买家秀图片;真正的测评视频接口全部要 h5st 签名或登录态。

## 方案 A(Playwright 让浏览器自带签名)实测

思路:不逆向 h5st,用 Playwright 打开评价页,浏览器自己发带签名请求,监听 response 拦截视频数据。

- ✅ **短链还原成功**:Playwright(移动 UA)goto 短链 → 落地 URL 的 returnurl 里解析出 `SKU`。curl 做不到,浏览器能。
- ⚠️ **撞京东风控**:headless 打开商品页 → 落地 `cfe.m.jd.com/privatedomain/risk_handler/...` 风控页,评价不加载,捕获 0 接口。
- ⚠️ **有头 + 反检测(去 navigator.webdriver + `--disable-blink-features=AutomationControlled` + PC UA)仍被拦**,PC 商品页 `item.jd.com/SKU.html` 也跳 risk_handler。

**结论**:裸环境(无登录 + 可能被标记 IP)连商品页都进不去。**这个项目真实成本是「风控对抗」不是写代码。**

## 关键更新(2026-07-28 晚):带真实登录 cookie 仍被风控拦

用户提供了真实 H5 登录 cookie(含 thor/pin=jd_5cd6de89bb77a/_pst + 设备指纹 3AB9D23F7A4B3C9B/shshshfpb)。

- curl 带 cookie 调 `productPageComments`:从「系统繁忙」→ 返回 `{}`(**登录态生效但缺 h5st 签名**,拿不到数据)。
- **Playwright(有头 + stealth + 注入 cookie)打开 `item.m.jd.com/product/SKU.html` 仍落地 `risk_handler` 风控页。**

**根因(重要)**:京东风控 = 登录态 + **设备指纹(3AB9D23F/shshshfpb)** + IP + 自动化检测(`is_headless_browser`)四重绑定。cookie 绑原始设备,拿到异环境(不同 IP/浏览器指纹/自动化 Chrome)复用直接被识别拦截。**cookie 救不了异环境。**

**结论:独立服务器爬虫方案不成立。** 正确产品形态 = **在用户自己已登录、未被风控的真实浏览器里跑**(油猴脚本 / 浏览器扩展 / Console hook fetch+XHR 抓 mp4),等于用户本人操作,绕开全部风控。报价与交付物应按"浏览器扩展/脚本"重定义,而非服务器 API。

## 突破方向(按有效性)

1. **登录态 cookie(最有效)**:账号扫码登录一次,持久化 context 复用,登录用户风控宽松。
2. **住宅代理 IP**:数据中心/被标记 IP 易触发。
3. **过滑块验证**:接打码,最脏。

## 报价影响

- 方案 A(无头浏览器 + 登录态)现实报价:**开发 ¥1.5万~2.5万 + 月维护 ¥800~1500**(养号/换IP/跟风控改版)。
- 必须单收维护费,反爬类改版频繁,不收必亏。
- 别承诺 100% 稳定与永久可用。

## 复用资产

PoC 脚本在会话 scratchpad:`jd_review_videos.py`(图片买家秀抓取,能跑)、`recon.py`/`recon2.py`(风控侦察)。核心是 `getProductPageImageCommentList`(GBK)+ Playwright `channel="chrome"` 复用系统 Chrome。

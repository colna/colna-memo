---
title: 美博会 CIBE 小程序（逆向 + 参展商名单）
date: 2026-11-03
tags: [project, wechat, miniprogram, 美博会, 数据抓取]
---

# 美博会 CIBE 小程序

微信小程序「美博会 CIBE」，appid `wx3c59af19d3a05227`（v115，2.04MB wxapkg）。2026-11-03 从「能否看懂功能」出发，做到解包、功能盘点、全量展商名单抓取。

## 基本信息

- 技术栈：uni-app(Vue) 编译产物；后端 yudao/ruoyi-vue-pro 风格。
- API base：`https://mp.beautyisp.com/app-api`，上传 `https://mp.beautyisp.com/upload`；官网 chinainternationalbeauty.com。
- 页面：85 个；tabBar = 首页 / 美记 / 寻找展商 / 我的。
- 功能面：展会 / 展区 / 展商 / 展品、笔记社区、供需对接（找产品 / 合作 / 直播 / 广告位）、企业入驻与认证、实名认证、门票（购买 / 退款 / 发票 / 电子票 / 签到）、活动报名、收藏 / 消息 / 名片。核心接口前缀 `/ems/*`。

## 产物

| 产物 | 路径 |
|---|---|
| 展商名单与分析报告 | `/Users/colna/WORK/meibohui-72/第72届中国（广州）国际美博会-参展商名单与分析.md`（951KB / 4,171 行，名单 3,829 行） |
| 原始数据 | 同目录 `data/exhibitors.json`、`raw/page-*.json`（20 页 × 200） |
| 分类结果 | `data/classified.json` |
| 解包/解密脚本 | `/tmp/wxrev/decrypt.js`、`/tmp/wxrev/unpack.py`、解包目录 `/tmp/wxrev/unpacked`（**临时，重启即失**） |

## 接口口径坑（重要，报告已据此标注）

- `GET /app-api/ems/exhibitor/page?lang=0&pageNo=N&pageSize=200` **不按 `exhibitionId` 过滤**：传 14 / 15 / 999999 都返回同样 3,829 家 → 拿到的是「当前展商库全量」，不是第 72 届专属名单。
- 第 72 届的 `/ems/zone/*` 返回空。
- 列表接口不返回展位号，只有详情才有部分 `boothNo`。
- 分页 20 页 × 200，按 `id` 去重后 3,829 家。

## 分类结果（关键词 + 名称规则启发式，非官方口径）

护肤/功效护肤 897、大健康 498、医美/医疗器械 429、包装包材 382、美发/美睫/美甲 231、电商/服务 183、洗护 171、美容仪器 155；标签：OEM/ODM 565、品牌方 1,248、院线渠道 373、出口/外贸 619。地域：广州 911、深圳 291、广东 253、佛山 132、东莞 111。

- **误差**：约 13.1% 落入其他/未分类；纯英文名、简介缺失、跨品类企业需人工复核。引用数字时必须带这句限定。

## 待办

- [ ] 若需「第 72 届专属」名单，得找带届次维度的接口（当前 page 接口无此过滤），或按展区/展馆接口重新抓。
- [ ] 分类可用 LLM 复核那 13.1%（当前纯关键词启发式）。

## 来源

2026-11-03 Daily 三条工作日志（15:06 可行性 / 15:20 解包 / 16:05 抓取），本人执行。取包与解密细节见 [wechat-miniprogram-wxapkg-unpack](../30-Resources/troubleshooting/wechat-miniprogram-wxapkg-unpack.md)。

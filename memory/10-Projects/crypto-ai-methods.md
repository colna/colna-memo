---
title: 「AI 炒币」方式全景 + 10 USDT 可行性（crypto_ai_methods）
date: 2026-11-04
tags: project, crypto, ai, trading, feasibility, binance, negative-result
---

# 「AI 炒币」方式全景 + 10 USDT 可行性

2026-11-04 立项的**独立支线**（与 `quant-trading` 的一次性翻倍线并列，不是同一条主线）：用户想知道「市面上的 AI 炒币都怎么做、10 USDT 能做哪一种」。

**结论（一句话）**：8 类互斥类别里，没有一类能在 10 USDT 上把成本摊平；4 类不可行（其中资金费 carry 的阻断依赖报告沿用的 2N/L=1 假设，**本库 14:03 已更正为 N + N÷L**，见下表注）、1 类被提币费劝退、剩下 3 类费后为负或只是 beta，跟单与卖课本质是「别人赚你的钱」。AI 的真实优势在信息结构化 / 风控纪律 / 监控告警 / 对账复盘，**不在方向预测**。核心结论不依赖任何社媒样本。

> 用法提醒（数据与评价函数绑定）：本页所有「不可行」都是在**10 USDT 本金 + VIP0 费率（现货 0.1%/边、合约 taker 0.05%/边）+ 2025-11~2026-10 单边下跌样本期**这三个前提下成立的。换本金、换费率等级、换行情窗口，部分结论会变（见「我认为可能错的地方」）。

## 交付物与指纹（2026-11-04 14:30 定稿，14:33 最后一处归因更正）

路径：`shared/crypto_ai_methods/`（Team Computer 共享目录：`/Users/colna/WORK/rakazo/data/desktop-computers/team-8f0a2a4300c87fca269da0a6e44db684/shared/crypto_ai_methods/`）

| 文件 | 事实 | 我的复核 |
|---|---|---|
| `REPORT.md` | **426 行**；SHA256 `9018a73a5a99e8a4eb9c36ccd1d7e2fe2db0009201a39d773927145719425627` | `wc -l` + `shasum` 一致 |
| `scripts/feasibility.py` → `feasibility.json` | SHA256 `a9cda7b3eba97c74db2529b165497bd6c9f691e2406929580a43ee20bd9908d9` | 连续跑两次，JSON 字节相同、stdout **IDENTICAL** |
| `scripts/power.py` → `power.json` | SHA256 `27a1270a811cbe990e49e59d34c2fd971fe2626832dfce03ac8bc3a8319ee05f`；纯解析式，无随机数 | 读 JSON 交叉核对关键值 |
| `scripts/verify.py` | 把报告里的关键数字回查 JSON | **27 PASS / 0 FAIL，rc=0** |
| `evidence/probe_log.txt` | `probe_utc=2026-11-04T06:27:04Z`，11 个探测 | 逐行核对 |
| `evidence/platform_fetches/*.html` + `SHA256.txt` | 三个页面正文存档 | 3/3 SHA256 与存档清单一致 |
| `evidence/waf_challenge_headers.txt` | 560 B，**只有响应头，没有挑战页正文** | 见「归因更正」 |

**确定性为什么成立**：`power.py` 是解析计算、`feasibility.py` 无随机数，且 JSON 内**不含 `generated_utc`**（时间戳只写在报告正文）→ 同一输入两次运行必须字节相同。这是本项目「同脚本同数据两次运行必须一致」的可执行版本。

输入（A 级，来自 `shared/binance_p0a/`）：`spot_ex.json` `656aa598…944cb`、`spot_px.json` `d0fd750b…06ad64`、`kl1m_btcusdt.csv` `820ba00b…61cece`。数据窗 2025-11-01T00:00Z ~ 2026-10-31T16:00Z（12 个月 1m，5 标的 × 524,160 = **2,620,800 根**）。

## 10 USDT 的硬约束（A 级：exchangeInfo，serverTime 2026-11-04T04:51:46Z）

| 标的 | 价格 | minNotional | minQty | 10 U 可容档数 |
|---|---|---|---|---|
| BTCUSDT | 77,220.15 | **5.00** | 0.00001 | **2** |
| ETHUSDT | 2,272.79 | **5.00** | 0.0001 | **2** |
| DOGEUSDT | 0.073960 | **1.00** | 1 | **10** |
| ADAUSDT | 0.187600 | **5.00** | 0.1 | **2** |
| XRPUSDT | 1.268500 | **5.00** | 0.1 | **2** |

→ 四分之四的主流标的只有 **2 个仓位格**；三角套利 3 腿 × 5 U = **15 U > 10 U**，名义上就出局。

## 网格：12 个月真实回放（B 级，含期末库存，单线）

主表（0.5% 间距，报告 §5.2 口径）：

| 标的 | cycles | 完美择时往返上界 | 单线净额 | 占 10 U | 持仓时间 |
|---|---|---|---|---|---|
| BTC 0.5% | 0 | 2,425 | **−1.453 U** | −14.53% | 99.42% |
| ETH 0.5% | 0 | 3,869 | **−2.060 U** | −20.60% | 99.43% |
| ADA 0.5% | 5 | 6,464 | **−3.319 U** | −33.19% | 99.60% |
| XRP 0.5% | 11 | 4,603 | −2.294 U | −22.94% | 99.40% |
| DOGE 0.5% | 4 | 4,982 | −0.580 U | −5.80% | 99.59% |

打平间距 0.2% 那一行（**另一组行，别和 0.5% 混**）：BTC −1.461 U / cycles **2** / 上界 10,155；ETH −2.059 / 7 / 14,661；ADA −3.397 / **10** / 23,403；XRP −2.451 / 20 / 16,831；DOGE −0.589 / 14 / 18,324。**上界与实际差 3~4 个数量级**，差额全部来自「买入后价格不回来」（库存风险），不是手续费。

- 完美择时上界（2 档都踩中每个往返）：BTC 0.5% → **+0.199 U/日**（年 +72.7 U）；实际单线 **−0.0040 U/日**，差约 50 倍。
- **0.2% 间距的上界恰好是 0**：10,155 个往返 × 5 U ×（0.2% − 2×0.1%）= 0.00 U —— 这是「打平间距 = 2×费率」的实证，且**与行情无关**。

## 八类策略 × 10 USDT（结论表，非建议）

| 类别 | 10 U 下的判定 | 依据 |
|---|---|---|
| 现货网格 | 技术上能开，**费后为负**（−0.0040 U/日，12 个月 −14.5%）；99.4% 时间套牢 | B+A |
| DCA / 定投 | 能开，但收益 = beta（样本期 BTC −29.25% ~ ADA −68.34%），手续费 0.01 U 一次性 | A+B |
| 资金费 carry | **报告判定：不可行**——报告沿用 **2N 口径（隐含 L=1）**，BTC 单腿 5.4054 → 10.82 U > 10 U。⚠ **本库 14:03 已更正为「资本 = N + N÷L」**：L≥2 时本金够开（8.108 / 7.207），但 carry 线已于 13:58 **冻结归档**，且实测年化最好只有 +0.1736 U（不够 VPS）→ 本行只作「报告当时的判定」，**不是现状结论**，详见 `quant-trading.md`「G0-a 更正」 | A（费率）+ 本库更正 |
| 跨所价差 | 名义刚好 10 U，但提币 1 U = 本金 **10.2%** → 需 204 次 5bp 机会回本 → **被费用劝退** | D（提币费未核实） |
| 三角套利 | **不可行**：3 腿 15 U > 10 U | A（算术） |
| 做市 / HFT | **结构性不可行**：费率等级靠 30 天交易量，10 U 永远 VIP0 → maker 0.1% ≫ 点差 | A+D |
| 跟单 / 信号群 | 本质是「别人赚你的钱」：10 U 翻倍最多被分 ~1 U；分润比例 D 级未取得 | C/D |
| ML / LLM | 技术门槛 0，但收益上限被 fee drag 压死：每天 1 往返 = **年 73% 本金** | B |
| 链上 MEV / sniper | **不可行**：gas 与仓位无关，单笔 gas > 0.5 U 即吃 5% 本金 | D |
| 内容生意（卖课/返佣） | 不做交易，收入 = 订阅 × 转化 × 复购；对 follower 是高危 | 推演 |

> 行数 10 > 类别数 8：因为「统计套利」按机制拆成 3 个互斥子类（资本/费用/可行性完全不同）。

## 手续费底噪（所有类别的共同天花板）

| 动作 | 成本 | 占 10 U |
|---|---|---|
| 现货一次往返 | 0.0200 U | 0.20% |
| 永续一次开平 | 0.0100 U | 0.10% |
| 三角 3 腿 | 0.0300 U | 0.30% |
| 网格 1 cycle（0.5%） | 0.0700 U | 0.70% |
| 跨所往返 + 1 次提币 | **1.0200 U** | **10.20%** |
| **每天 1 次往返 × 一年** | **7.30 U** | **73%** |

BNB 抵扣 25% 后年 drag 从 73% 降到 **54.75%**（`power.json.annual_fee_drag`，已算好；注意这个数与 §10 里 BTC 10x 的 p=54.75% 纯属巧合，别混）。

## 「晒单有 alpha」为什么不成立（`power.py`，可复跑）

| 命题 | 数字 |
|---|---|
| 检测 1.29pp edge（α=0.05，power=0.8） | **11,791 笔**独立交易 = 每天 1 笔跑 **32.3 年** |
| 100 笔样本能分辨的最小 edge | **14.01pp**（打平线的 10 倍） |
| 90 笔（3 个月纸面） | 只能分辨 ~16pp |
| 50 组参数 × 500 笔，纯噪声下「最优那个」 | 看起来有 **+4.70pp** edge（多重比较） |
| 1000 个负期望账号（p=0.4871，50 笔/年）一年后仍盈利 | **42.76% → 428 个** |

→ 推论：任何「我 30 笔赚了 80%」的展示**不含信息**，既不能证明有 edge，也不能证明没有。要判真伪只能要原始逐笔 + 足够样本 + **预先声明**的假设。

## AI 真正有优势的环节（有本项目交付物为证）

对账校验（72/72 个数据文件 SHA256 与官方 CHECKSUM 一致）、回测基础设施（`gamble.py` 5.6 s/标的、同输入两次 SHA256 相同）、风控纪律（按规则给负值，没有一次「这次不一样」）、监控告警（HTTP 码 + 内容指纹自动取证）、信息结构化（分类学/清单可脚本生成可 diff）、口径管理（N=单腿名义、资金费只按永续腿、摊销 vs 结算）。

反过来**没有优势**：方向判断、择时、叙事定价、抢跑（延迟与资本的游戏）、以及任何需要「持续 ≥1.29pp 稳定优势」的事。

## 平台取证：0 个样本（明确失败，不硬凑）

`./scripts/probe.sh`（11 个探测，`evidence/probe_log.txt`）：X 首页 200/45,991 B、X 搜索 200/28,870 B、TikTok 首页 200/430,019 B（含 diggCount/playCount）、TikTok 搜索 200/362,072 B、抖音首页与搜索 **000**、`api.binance.com` / `fapi.binance.com` / `www.binance.com/en/fee/schedule` 全 **000**、`data.binance.vision` 200/2,712 B、`cn.bing.com/search` 200/102,272 B。

三个平台的**失败模式各不相同**（正文存档 + SHA256：`evidence/platform_fetches/SHA256.txt`，我 3/3 复核一致）：

| 页面 | 字节 | 指纹 | playCount | 性质 |
|---|---|---|---|---|
| `douyin_search_aichao.html` | **72,914** | `_$jsvmprt` 命中 1，`<body></body>` 空 | **0** | HTTP 200 但正文为空 + JS VM 反爬壳 |
| `tiktok_search_ai_crypto.html` | **361,423** | `login` 93 次（不分大小写） | **0** | 未登录壳 + 登录引导，客户端渲染 |
| `x_search_btc.html` | **28,811** | 0 个 `viewCount/likeCount` | — | React 壳，数据走 GraphQL 需登录 cookie |

→ **抖音不是「连不上」，是「返回 200 的 72 KB 反爬壳」**。两种说法的结论一样（0 样本），但错误归因会把下一步带偏：要解决的是**登录态/签名**，不是网络。

搜索引擎也是降级的（`scripts/search.py`，证据 `evidence/search_degraded.txt`）：`xyzzyqwerty 987654` 返回 DeepSeek 官网、`"AI炒币" 骗局` 返回 AI 工具集/IBM/Kimi → 多词中文语义检索本节点基本不可用，只有单词/专名有效。**所以「改用二手报道」这条路也走不通**，报告给 0 样本并说明替代方案（`§3.2` 变现结构解剖）。

## 归因更正：`202` + WAF ≠ 地域封锁（本次唯一新事实）

- 旧判定：`www.binance.bh` 三个 URL「451/000 → geo-block」。
- 实测：**HTTP/2 202**，响应头 `server: CloudFront`、`x-amzn-waf-action: challenge`、`content-length: 2108`（2 KB JS 挑战页，**不是费率表**）。原始响应头存 `evidence/waf_challenge_headers.txt`。
- headless Chromium（`--dump-dom --virtual-time-budget=20000`）连打 37 s 未产出 DOM，按时间盒放弃、`pkill` 清理、临时脚本已删。**结论：不是地域封锁，是 AWS WAF 挑战，headless 未能突破。**
- 两种根因的解法完全不同：`451`（`testnet.binance.vision` 那种，body 有 `restricted location` 条款）要换落地地区；`202 + challenge` 要过 JS 挑战（真实浏览器会话 / cookie），换 IP 也没用。**库里已有的 451 记录仍然是真 451**，别拿这条去否定它。
- 我 2026-11-04 14:35 独立重试同一 URL：仍 **202 + `x-amzn-waf-action: challenge`**，但这次 `content-length: 0`（正文为空）→ 挑战页正文**不稳定**，不可依赖。

## 未取得 / 未核实（不得当实测引用）

- 费率表（0.1% / 0.05%）是**公开口径的常识值**，本轮未直连复核；若账户是 VIP1+ 或开 BNB 抵扣，§5 的手续费按比例下降。
- `futures exchangeInfo` 与 `leverageBracket` 未取得 → 永续侧 minNotional 与 MMR 分档仍是假设（沿用 `quant-trading` 的标注）。
- 跨所价差、链上 gas、BTC 实际点差：D 级，无数据。
- 跟单最小资金 / 分润比例：需登录，未取得。
- 平台内容样本：**0**（见上）。

## 报告作者自标的「可能错的地方」（照录，防止后人当铁论）

1. 「完美择时上界」用 ZigZag 计数，**忽略同一时刻多档同时触发的资金占用** → 对多档偏乐观；对单线实测偏保守（单线只有 2 档中的 1 档在工作）。
2. 「12 个月单边下跌」只是一个样本期，网格在震荡市会更好；但**「0.2% 上界恰好为 0」与行情无关**，所以「震荡市能覆盖成本」不成立。
3. 幸存者计算用「单笔 1:1 盈亏」简化，真实肥尾会让「仍盈利」比例略降。
4. 8 类分类学是作者的切分，边界有争议（例：「AI 选币 + 手动下单」）；换切分不影响结论。

## 别再试（省一次踩坑）

- `www.binance.bh` 抓费率表：202 + WAF 挑战，headless `--dump-dom` 连打 37 s 出 0 字节 → 别再试；要数据只能走真实浏览器会话或用已落盘快照。
- 抖音/TikTok/X 搜索页抓指标字段：200 但 0 字段（反爬壳 / 未登录引导 / React 壳）→ 命中「200 + 字节数看着正常」时先数目标字段，别数 HTTP 码。
- 本节点多词中文语义检索：只按第一个词匹配，无意义串也能返回 DeepSeek → 不能当检索证据。

## 复跑

```bash
cd shared/crypto_ai_methods
python3 scripts/feasibility.py      # ~1.3 s -> feasibility.json
python3 scripts/power.py            # ~0.1 s  -> power.json
python3 scripts/verify.py           # 必须 rc=0，27/27 PASS
./scripts/probe.sh                  # ~40 s，会写 evidence/probe_log.txt（会覆盖旧日志，先备份）
cp feasibility.json /tmp/a.json && python3 scripts/feasibility.py >/dev/null && shasum -a 256 /tmp/a.json feasibility.json
```

## 相关

- [quant-trading.md](quant-trading.md)（同用户、同 10 USDT，但**目标不同**：那条是「一次性翻倍」；本页是「方式全景 + 可行性」，两条线的结论不要互相引用当证据）
- `shared/binance_p0a/REPORT_GAMBLE.md`（方向预测负期望的实测来源之一，§6.1 引用了它 4 行）
- [troubleshooting/waf-202-challenge-not-success.md](../30-Resources/troubleshooting/waf-202-challenge-not-success.md)（`202` + `x-amzn-waf-action` 判读、`grep` rc=1 陷阱）
- [troubleshooting/binance-min-order-and-capital.md](../30-Resources/troubleshooting/binance-min-order-and-capital.md)（minNotional 与资本口径）
- [troubleshooting/binance-leverage-fee-and-alpha.md](../30-Resources/troubleshooting/binance-leverage-fee-and-alpha.md)（手续费 ∝ L、打平所需 alpha）

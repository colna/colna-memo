---
title: HTTP 202 不是成功：CloudFront + x-amzn-waf-action: challenge 才是真因（附 grep rc=1 陷阱）
date: 2026-11-04
tags: troubleshooting, http, waf, cloudfront, scraping, curl, grep
---

# HTTP 202 不是成功：CloudFront + `x-amzn-waf-action: challenge` 才是真因

来源：2026-11-04「AI 炒币方式全景」取证（`shared/crypto_ai_methods/`，数据分析师交付；响应头 `evidence/waf_challenge_headers.txt`，我 14:35 独立重试同一 URL 复核）。适用面：任何走 CloudFront 前置 WAF 的站点（AWS WAF 默认配置即此指纹），不止币安。

## 症状（两个典型误判）

1. **把 202 当成功**：`curl` 退出码 0、HTTP 202、`content-length: 2108` → 以为页面拿到了，解析出 0 个字段，于是在「解析器」上折腾半小时。202 是 **Accepted**，不是 200 OK。
2. **把「连不上 / 拿不到数据」一律归因为地域封锁**：本例原先写成「451/000 → geo-block」，实际是 WAF 挑战。方向错了，后面所有补救（换 IP、换地区、换 DNS）都无效。

## 判据（看响应头，不看状态码语义）

```
HTTP/2 202
server: CloudFront
content-length: 2108
x-amzn-waf-action: challenge
cache-control: no-store, max-age=0
x-cache: Error from cloudfront
```

**看到 `x-amzn-waf-action: challenge` 就到此为止**：这是 AWS WAF 的 JS 挑战页，正文是 2 KB 脚本（会 `document.cookie` 写入 `aws-waf-token`），没有业务内容。

与 `451` 的区别（**必须分开归因**）：

| 现象 | 根因 | 解法 |
|---|---|---|
| `451` + body 含 `restricted location` | 地域/合规封锁（业务侧拒绝） | 换落地地区 |
| `202` + `x-amzn-waf-action: challenge` | WAF 挑战未通过 | 真实浏览器会话跑完 JS、拿 cookie 复用；换 IP 无用 |
| `000` + 无响应 | DNS / 连接 / TLS 层失败 | 查解析与链路（见 `quant-trading.md` 的 DNS 污染证据） |
| `200` + 字节数正常但目标字段 0 | 反爬壳 / 客户端渲染 / 未登录 | 数**字段**，不是数状态码（见下） |

## 修法

1. **永远打印响应头**：`curl -D headers.txt -o body.html`，先读 `x-amzn-waf-action` / `server` / `content-length`，再谈解析。
2. **先验证「拿到的是不是想要的东西」**：`grep -c '<目标字段>' body.html`，字段数 0 就是没拿到，别进入解析阶段。
3. **headless 抓 WAF 站要有时间盒**：本例 `--dump-dom --virtual-time-budget=20000` 连打 37 s、产出 0 字节；定好上限就停，并 `pkill` 清理残留 Chrome 进程（本例临时脚本用完已删）。
4. **别把「202 + challenge」写成「站点挂了」或「被墙了」**：这是三种不同故障（WAF / 地域 / 链路），写错会让下一个人重跑一遍同样的死路。
5. **`content-length` 也不稳定**：我 14:35 重试同一 URL 仍是 `202 + challenge`，但这次 `content-length: 0`（连挑战页正文都没回）→ **不要依赖挑战页正文做任何判据**，只依赖响应头。
6. **抓不到就存「失败证据」而不是空文件**：本例存响应头（560 B）+ 三个平台正文 SHA256 清单，报告里明写「0 样本」。这比硬凑二手报道有价值。

## 附：`grep -c` 的 rc=1 陷阱（同一批踩到的）

```
$ grep -c "RENDER_DATA|__INITIAL|播放" dy2.html
0
$ echo $?
1
```

- `grep` 退出码 **1 = 没匹配**（不是出错，2 才是出错）。`0` 是打印出来的计数，不是退出码。
- 在 `set -e` 脚本里这一句会**中断整个脚本**；写法改成 `n=$(grep -c ... || true)` 或 `grep -q ...; rc=$?`。
- 更关键的是语义：这里的 rc=1 **正是结论本身** —— 「HTTP 200 + 72,914 字节，但零个指标字段」。日志里只写「rc=1」会被读成「抓取失败」，只写「200」会被读成「成功」，两者都错。

## 附：用「官方域名族」核对钓鱼链接（判据待验，别当事实用）

思路：AWS WAF 挑战页通常带 `awsWafCookieDomainList = [...]`，列出该站点授权的 cookie 域；把它和搜索结果里的「币安官网入口」域名比对。

**但在本批交付物里这条无法验证，我明确记录**：

- `grep -r awsWafCookieDomainList shared/crypto_ai_methods/` → **0 命中**（报告正文没有，evidence 里也没有）；
- 挑战页正文**未存档**（`evidence/waf_challenge_headers.txt` 只有响应头），且我重试时 `content-length: 0` → 拿不到正文；
- 所以「`home.cc-binance.sh.cn` / `binance-cc.cn` / `web-binance-zh.cn` 不在官方域名列表」这句**没有证据支撑**，不要引用。

**目前能站住的那半条**（`REPORT.md:203`）：搜索「binance」时首页结果里出现多个非官方域名，标题都写「币安官网入口」；报告**不判定其性质**（可能是 SEO 联盟页，也可能是钓鱼），只给硬规则 —— **只认自己手输的官方域名 + 官方 App，任何从搜索/私信/群聊点进去的「官网入口」都不进**。这条不依赖任何域名清单。

## 沉淀（一句话）

判读抓取结果要三看：**状态码 + 响应头（`x-amzn-waf-action`）+ 目标字段计数**；只看看其中一个，就会把 WAF 挑战读成成功、把「零字段」读成「抓取失败」。

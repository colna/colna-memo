---
title: 闲鱼（goofish）item detail 逆向取数
date: 2026-11-04
tags: troubleshooting, xianyu, goofish, mtop, scraping, cash7d
---

# 闲鱼（goofish）item detail 逆向取数

> 来源：2026-11-04 17:15 数据分析师全量执行 + 我在 `shared/cash7d/` 的现场复跑（`verify_xy_delivery.py` 7/7、`verify_xy_api.py --gate` exit 1、`verify_v3.py` 39/39）。脚本与原始响应都在 `…/team-8f0a2a43…/shared/cash7d/`（`run_full_xy.py`、`replay_xianyu_detail.py`、`tools_v3/js/xy_detail_fetch.js`），本库只记方法与坑，不复制数据。

## 接口与签名（实证）

- 端点：`POST https://h5api.m.goofish.com/h5/mtop.taobao.idle.pc.detail/1.0/`，query 携带 `jsv=2.7.2 / appKey / t / sign / api=mtop.taobao.idle.pc.detail / v=1.0 / type=originaljson / dataType=json / sessionOption=AutoLoginOnly`，body 为 `data=<urlencoded {"itemId":"…"}>`。
- 签名：`sign = md5(token + "&" + t + "&" + appKey + "&" + data)`
  - `appKey = 12574478`
  - `token` = cookie `_m_h5_tk` 的值**前 32 位**（`_m_h5_tk=<token>_<ts>`，按下划线切）
  - `t` = `Date.now()` 毫秒字符串；`data` 必须与 body 里那份**完全一致**
- 必须在**已登录**的 goofish 页里发（`credentials: 'include'`），token 与 cookie 是同一会话，否则签名对但业务侧不认。
- 返回取值：`data.itemDO.soldPrice`（现价）/ `originalPrice`（原价）/ `wantCnt`+`wantCntUnit`（想要数）/ `browseCnt`（浏览数）/ `cpvLabels[propertyName='预计工期'].valueName` / `title` / `desc` 等。

## 护栏（照抄，别放松）

- 只读 detail，**不调任何写接口**；不注册、不投标、不私聊、不下单、不关注。
- 间隔 **1.2s/次**（DOM 复读 1.5s），单轮硬上限 **200 次**（`round.json` 记账，`--reset-round` 需人工）。
- 熔断：**连续 2 次** `FAIL_SYS_USER_VALIDATE` / `RGV587` / 403 / 验证码 → 立即停（exit 3），不要自动重试硬闯。
- 实操量级参考：148 条 detail + 23 次商品页只读 + 20 次 DOM 复读 → punish/403/验证码 **0/0/0**，p50 **157ms**、max 2,316ms。

## 坑 1：新标签页首访必命中一次 punish

- **现象**：在**新开的**标签页首次发请求 → `FAIL_SYS_USER_VALIDATE` + `RGV587_ERROR::SM::哎哟喂,被挤爆啦,请稍后重试`（响应体是 `_____tmd_____/punish?...action=captcha` 的 HTML）。
- **根因**：cookie / 风控指纹在首访时未「暖」到位（实证 `raw_v3/xianyu_api/cap_detail_1057937931438.json`：#1 punish、同 `t` 重试 #14 放行）。
- **修法**：**只在已登录的既有 goofish 标签页里发**（经 CDP `localhost:9222` 复用该页），不复用/不新建标签页；万一命中，同 `t` 重试一次即可，但**别把重试当常规路径**。

## 坑 2：商品页主价拆行 + 推荐位 → 抽取伪影

- **现象**：用 `¥\s*(\d+)` 抓商品页得到 79.20 / 2.00 / 8.50，而 API `soldPrice` 是 88 / 5 / 10。
- **根因**：详情页主价是**拆行渲染**（`¥` 与数字分行），且页面底部「为你推荐」也带价格与 `N人想要`，首个匹配常落到推荐位。
- **修法**：价格证据**只用搜索页卡片 baseline**，不用商品页；同页 `原价` 字段 3/3 等于 API `originalPrice`（360 / 9.99 / 666），可当交叉旁证但不当主价。

## 坑 3：`browseCnt` 不可比

- **现象**：同一 item 3 分钟内 `browseCnt` 38873 → 38876（+3），而 `price`/`wantCnt` 未变；DOM 侧还常显示「1万浏览」。
- **根因**：浏览数是实时累计且展示端四舍五入到「万」。
- **修法**：门槛/核对**只比 price 与 wantCnt**，`browseCnt` 只做量级参考，不进比对、不进分母。

## 坑 4：`wantCnt` 单调不减 → 两个时刻的严格相等门槛必 FAIL

- **现象**：148 条里 93 条有 DOM 基线，78 严格相等、15 条正漂移（`+1…+5`，最大相对 1.43%）；严格 gate 判 **FAIL**。
- **根因**：想要数是实时累计（**单调不减**），基线 16:47 与 API 17:19 相隔约 35 分钟；15/15 现读 DOM 后**全部等于 API**，负 Δ 0 条。所以 FAIL 来自**判据设计**（拿两个时刻做严格相等），不是取数错。
- **修法**：分两层——① 保留 v1 的 FAIL 作为正式结论（**不许事后调参把 FAIL 改写成 PASS**）；② 另立带时点的判据：price 严格相等 + `wantCnt` **单调不减**（任何负 Δ 即 FAIL）+ 现读 DOM 逐条等于 API，并写明覆盖分母。规则**先落盘再跑**。

## 坑 5：`cpvLabels[预计工期]` 覆盖只有 46.6%

- **现象**：148 条里只有 **69 条**有 `预计工期`（1天内 18 / 1-5天 40 / 其他 11 / 无 79）。
- **根因**：类目/卖家未填该属性，API 就没有该字段；148/148 均 `SUCCESS::调用成功`，不是抓取失败。
- **修法**：工期类结论只能在**子集**上排序，不能覆盖全量；缺失机制（卖家未填 vs 类目无此属性）不做因果解读。

## 坑 6：团队有两个 `shared/cash7d`，读到 `homes/` 那份等于读旧镜像

**症状**：同一脚本在不同时间报的「文件数 / 最新 mtime / gate 结果」互相打架，或算出来的数比上游摘要旧一截；日志类产物「凭空不见了」。

**两个目录**（同一账号 `colna`，都在 `…/rakazo/data/` 下）：

| 路径 | 角色 |
|---|---|
| `desktop-computers/team-8f0a2a4300c87fca269da0a6e44db684/shared/cash7d` | **活动目录**，所有写入落这里 |
| `homes/team-8f0a2a4300c87fca269da0a6e44db684/shared/cash7d` | **镜像**，只读用，带过滤器 |

**根因**：镜像侧有过滤器，**排除 `*.log` 与 `__pycache__/`**；而且镜像**持续刷新**，所以 inode 与文件数都是**移动靶**，不能当「是不是同一个目录」的判据。

**实测（2026-11-04 17:36–17:37，两次 `stat` 间隔 3 分钟）**：

- 活动目录：inode **123972662**、**547** 文件（两次一致）。
- 镜像：inode **124054170**、**528** 文件、**528 个文件的 mtime 全部 = 17:36:34**（随时被重写）。上游摘要给的 `124051099 / 521 / 17:27` 是更早一刻的快照，**已过期**。
- 差集：镜像少 **20** 个 = 11 个 `fetch_*.log` + `numbers_final.log` + `verify_v3.log` + `xy_collect.log` + `raw_v3/fetch_fl_v3.log` + `raw_v3/fetch_parallel.log` + `raw_v3/xianyu_api/gate_dryrun.log` + 3 个 `__pycache__/*.pyc`；**多 1** 个 `selftest/tools/ocr`。

**修法**：

1. 路径**固定写 `desktop-computers/…` 绝对路径**；`homes/…` 只当只读镜像，**不在上面判「有没有新产物」，也不复用它算数**。
2. 判「产物新不新」看**文件 mtime**，不看目录 inode / 文件总数（镜像会重写）。
3. 任何「日志全不全」的核对**必须在活动目录做**——镜像里天然没有 `*.log`（例：`gate_dryrun.log` 只在活动目录存在）。

## 工具适用性：REA 不可用于抽 mtop endpoint（负向结论，2026-11-04 17:27–17:31 实跑）

**结论**：REA 6.7.0 **不能**用作闲鱼 mtop 链路的独立验证源——它抽不到 endpoint。同输入下 `grep` 能拿到的 15（宽松 21）个 API id，REA 抽到 **0**。其他 route/URL 型 bundle 仍可用。

- **输入**：`https://www.goofish.com/` 首页里的 3 个静态 bundle（`g.alicdn.com/idle-pc/xy-site/0.0.179/js/{p_layout,p_index,main}.js`，共 **2,194,823 B**：1,365,072 / 425,937 / 403,814）。命令 `rea analyze-javascript-application ./app --format json --max-semantic-nodes 200000` → **exit 0**，29.4s，输出 **140,168,529 B**（140 MB），`parse_failures 0`、`visited_ast_nodes 648,123`、`modules 1,284`、`findings 4,617`。（对照：我更早一次限节点跑 3 JS / 302k AST / 713 findings / endpoint 0，结论一致。）
- **真值（本地 grep，0 平台请求）**：`api:"mtop.*"` 字面量调用点 **23 处**（`p_layout` 19 / `p_index` 4 / `main` 0），union distinct **15 个**（14 + 2，重叠 1）；放宽到任意 `mtop.*` 字面量 → distinct **30**，剔除 `mtop.config.*` / `mtop.request` / `mtop.middlewares` 后 **21 个真实 API id**。典型形态：`(0,D.G)({api:"mtop.taobao.idlehome.home.webpc.feed", v:"1.0", data:{itemId:"", pageSize:30}})`。
- **REA 召回**：证据体里 `mtop.taobao` **0** 次、`h5api` **0** 次、`idlehome` **0** 次，`mtop` 仅 **31** 次且**全部**出现在 `limitations`；endpoint 节点虽有 45 个，但清一色是 route/network 碎片（`route: /, account, account/api, *`；`network: a, b, c, q, ee, eE, hover, spm…`）。→ **recall 0/15（宽松 0/21）**。
- **根因**：REA 的 endpoint 抽取走 property-path / route 启发式，**不覆盖「字符串字面量作对象字段、由库函数消费」的 mtop 形态**。同一次运行的 coverage 是 `status=partial, truncated=true`，`omitted_unknown_occurrences`：`call-flow/dynamic-call 35,872`、`object-flow/ambiguous-target 32,131`、`object-flow/dynamic-property 5,521`、`data-flow/dynamic-scope 3`、`timer 81`（每文件节点上限 20,000）——mtop 调用点正好落在被省掉的 dynamic-call 里。REA 自己给的是：`Static read/write property /lib/mtop/… has unknown-coverage presence; its receiver is unresolved or not a retained container`。
- **纪律**：全程只读本地文件，**0 次 mtop/h5api 请求**、未登录、未装依赖、未跑 `rea setup`、未写 memory、未改 prereg。原始证据：`…/shared/cash7d/REA_FEASIBILITY.md` §4（四个数）/ §5（grep 对照）/ §7（产物 `/tmp/rea_xy/app/*`、`/tmp/rea_xy/rea_out.json`）。
- **复用方式**：判定一个 bundle 能否用 REA 当验证源，先看 API 名是「完整 URL/路径字面量」还是「mtop 式字段拼接」——后者直接放弃 REA，改用 grep + 运行时抓包。

## 复跑（0 请求，可复核每个数字）

```bash
cd …/shared/cash7d
python3 verify_xy_delivery.py     # 7/7 PASS，重算全部数字，exit 0
python3 verify_xy_api.py --gate   # 重放 v1 门槛，FAIL exit 1（15 条全为正 Δ）
python3 verify_v3.py              # 39/39 PASS，exit 0
python3 replay_xianyu_detail.py --item-id 687464624280   # 1 请求，单条 exit 0
```

## 合规

只读公开面 + 已登录会话内的自有账号视图；不做写操作、不绕验证码、不批量转存原始页面。用于自家 7 天现金池（cash7d）的机会评估，不与 X/Twitter 逆向那条线互相引用（后者已被明确否决，见 `10-Projects/x-operator.md`）。

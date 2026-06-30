---
title: 阿里云 DMS 线上库查询操作手册(给 Claude / MetaBot)
date: 2026-06-30
tags: [dms, aliyun, postgres, presence-io, query, manual, reference]
---

# 阿里云 DMS 线上库查询操作手册

给 Claude / MetaBot 用的 presence-io 线上 PostgreSQL 查询脚本化手册。复制本节即可让任意 agent 直接上手跑数。

---

## 1. 接入参数(固定)

通过阿里云 **DMS Enterprise `ExecuteScript` OpenAPI** 跑 SQL,**不是**直连数据库。

| 参数 | 值 |
|---|---|
| Endpoint | `dms-enterprise.us-east-1.aliyuncs.com` |
| Region | `cn-shanghai` |
| Tid(租户) | `2915590` |
| DbId(库) | `80570568` |
| Logic | `false`(物理库,不是逻辑库) |
| CLI 二进制 | `./aliyun`(放 scratchpad,aliyun CLI v3) |

凭证走环境变量,**绝不写进任何会被提交的文件**:
```bash
export ALIBABA_CLOUD_ACCESS_KEY_ID="$AK"
export ALIBABA_CLOUD_ACCESS_KEY_SECRET="$SK"
```
> AK/SK 是生产凭证。只在 shell env / scratchpad 里用,用完即弃;粘贴过明文的 key 应尽快轮换。

---

## 2. 最小可用调用

```bash
./aliyun dms-enterprise ExecuteScript \
  --endpoint dms-enterprise.us-east-1.aliyuncs.com \
  --region cn-shanghai \
  --Tid 2915590 --DbId 80570568 --Logic false \
  --Script "SELECT count(*) FROM userinfo WHERE gender=2;"
```
返回 JSON:`.Results[0].Rows` 是行数组(每行是 `{列名: 值}` 的 dict,值都是字符串)。

---

## 3. ⚠️ 三条致命坑(必须记住)

### 坑 1:硬 200 行输出上限
`ExecuteScript` **每次最多返回 200 行**,超过静默截断,不报错。后果极坏:
- `WHERE id IN (...400 个 id...)` 会只回前 200 行 → 你以为某些用户"没有记录",其实是被截断了。
- 解决:**IN 列表分块 ≤150 个 id**;或对全量结果集做分页。

### 坑 2:OFFSET 分页会丢页
`LIMIT 200 OFFSET n` 在并发/排序不稳时会**静默丢整页**。
**改用 keyset 分页**(游标):
```sql
SELECT ... FROM t WHERE <filter> AND user_id > :last
ORDER BY user_id LIMIT 200;
-- 取本页 max(user_id) 作下次 :last,直到返回行数 < 200 停止
```
keyset 实测 27,091 用户 0 丢失;OFFSET 在同场景丢过页。

### 坑 3:聚合也受 200 行限
`GROUP BY user_id` 返回的是"每个用户一行",同样吃 200 上限。所以**分组查询也要按 ≤150 id 分块**,然后客户端合并。

---

## 4. 稳健调用封装(Python,实战验证)

要点:重试(DMS 偶发空响应)、`assert rows is not None`(区分"真没数据"和"调用失败")、≤150 分块、8 线程并发、失败块串行补跑。

```python
import os, json, subprocess, time
from concurrent.futures import ThreadPoolExecutor, as_completed
EP="dms-enterprise.us-east-1.aliyuncs.com"
env=dict(os.environ)
env["ALIBABA_CLOUD_ACCESS_KEY_ID"]=os.environ["AK"]
env["ALIBABA_CLOUD_ACCESS_KEY_SECRET"]=os.environ["SK"]

def call(sql, tries=8):
    """返回 Rows 列表;失败返回 None(调用方必须 assert 区分)。"""
    for _ in range(tries):
        p=subprocess.run(["./aliyun","dms-enterprise","ExecuteScript",
            "--endpoint",EP,"--region","cn-shanghai",
            "--Tid","2915590","--DbId","80570568","--Logic","false",
            "--Script",sql], capture_output=True, text=True, env=env)
        if p.stdout.strip():
            try: return json.loads(p.stdout,strict=False)["Results"][0].get("Rows") or []
            except Exception: pass
        time.sleep(1.3)
    return None  # 千万别把 None 当成空结果

# 分块 + 并发 + 失败补跑 模板
CH=150
def inlist(c): return ",".join("'%s'"%x for x in c)
def pull(chunk):
    sql="SELECT user_id uid, ... FROM t WHERE user_id IN (%s) GROUP BY user_id;"%inlist(chunk)
    return call(sql)

def run(ids):
    chunks=[ids[i:i+CH] for i in range(0,len(ids),CH)]
    res={}; fails=[]
    with ThreadPoolExecutor(max_workers=8) as ex:
        futs={ex.submit(pull,c):i for i,c in enumerate(chunks)}
        for fut in as_completed(futs):
            r=fut.result()
            if r is None: fails.append(futs[fut]); continue
            for row in r: res[str(row["uid"])]=row
    for i in fails:                      # 串行补跑失败块
        r=pull(chunks[i]); assert r is not None, "hard fail chunk %d"%i
        for row in r: res[str(row["uid"])]=row
    return res
```

keyset 全量分页模板:
```python
last=0; base={}
while True:
    r=call("SELECT user_id uid, ... FROM t WHERE <filter> AND user_id > %d "
           "ORDER BY user_id LIMIT 200;"%last)
    assert r is not None
    if not r: break
    for row in r: base[str(row["uid"])]=row
    last=max(int(x["uid"]) for x in r)
    if len(r)<200: break
```

---

## 5. 库表速查(presence-io PWA 业务,均 gender=2 为女)

| 表 | 关键列 | 备注 |
|---|---|---|
| `userinfo` | user_id, username, gender(2=女), app_name, regulation_status, deleted_at, cai_user_type, pending, sus_bot, condition | 用户主表 |
| `user_balance` | user_id, balance(varchar), frozen(varchar), balance_type('video'/'ai_persona'/'referral_bonus'), update_at | 一个用户每种 balance_type 一行;金额是字符串,用 `NULLIF(balance,'')::numeric` |
| `user_online_status_record` | user_id, updated_at(=最后在线) | 1 用户 1 行;**数字人无行** → 可当"真人活跃"过滤器 |
| `user_call_order` | female_user_id, order_type(VIDEO_CALL/MOCK_VIDEO/REFERRAL_BONUS), status(INIT/PAID/MISS_CALL), amount(varchar) | 通话订单,计费权威源 |
| `user_withdraw_task` | user_id, amount, status(FINISHED/FAILED/APPROVED), earning_type('video'/'ai_persona') | 提现;只有 FINISHED 是真出账 |
| `pwa_user_balance_change_history` | to_user_id, balance_change(varchar), change_type | 余额流水;**无 balance_type 列**;**不记提现** |
| `userinfo_pwa_club_mapping` | user_id | 公会主播映射;**不在此表=散户/retail** |

### 用户状态字段语义(gender=2 实测分布)
- `regulation_status`:**0=正常**;2、6=封禁(ban);4=异常。锁正常用户用 `=0`。
- `deleted_at`:NULL=在用;非 NULL=已软删除。**与 regulation 独立**(有人 status=0 但已删除)。
- `cai_user_type`:**1=真人(UserType.User)**;**3=数字人(UserType.DigitalHuman/"dh")**。排 DH 用 `<>3`。
- `pending` / `sus_bot` / `condition`:`condition` 是质量分(0、50–95),**不是**账号状态,别拿来当封禁判据。

### "干净活跃散户女"标准过滤(本次定位用的口径)
```sql
u.gender=2
AND o.updated_at >= '<N个月前>'           -- 活跃窗口,o=user_online_status_record
AND u.regulation_status = 0               -- 未封禁
AND u.deleted_at IS NULL                  -- 未删除
AND u.cai_user_type <> 3                  -- 非数字人
AND NOT EXISTS (SELECT 1 FROM userinfo_pwa_club_mapping m WHERE m.user_id=o.user_id)  -- 散户
```

---

## 6. 金额列处理铁律

所有金额是 **varchar**,可能为空串。永远这样转:
```sql
NULLIF(amount,'')::numeric        -- 空串→NULL,避免 ::numeric 报错
ROUND(SUM(NULLIF(amount,'')::numeric), 2)
```

---

## 7. 排错速记

| 现象 | 原因 / 修法 |
|---|---|
| 某些 id "查不到记录" | 多半是 200 行截断 → IN 列表分块 ≤150 |
| 全量统计数对不上 | OFFSET 丢页 → 换 keyset 分页 |
| `::numeric` 报 invalid input | 金额空串 → `NULLIF(x,'')::numeric` |
| stdout 为空 | DMS 偶发抖动 → 重试(模板已含 8 次) |
| 把 None 当空结果 → 过滤集偏小 | 所有查询加 `assert rows is not None` |

相关:[Grafana/Loki 查询手册](grafana-loki-query-manual.md) · [PWA 女钱包计费业务知识](pwa-female-wallet-billing.md)

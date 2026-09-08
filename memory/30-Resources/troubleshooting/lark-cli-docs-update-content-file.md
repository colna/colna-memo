---
title: lark-cli docs +update 写文件内容的两个坑
date: 2026-09-08
tags: [lark-cli, 飞书文档, docs-update, 静默失败]
---

# lark-cli `docs +update` 写文件内容的两个坑

## 坑一：`--content ./file.md` 把路径当字面文本写进去

```bash
# ❌ 文档里会出现一行字：./part-0.md
lark-cli docs +update --doc "<url>" --command append \
  --doc-format markdown --content ./part-0.md --as user

# ✅ 用 @ 前缀才会读文件
lark-cli docs +update --doc "<url>" --command append \
  --doc-format markdown --content @part-0.md --as user
```

**报的是 `success`**，所以不看文档不会发现。我一次写了 8 片，文档里就是 8 行文件名。

`@` 后面**必须是相对路径**，而且相对于**当前工作目录**：

```
--content: invalid file path "/abs/path/x.md": --file must be a relative path
within the current directory (hint: cd to the target directory first)
```

所以要先 `cd` 到文件所在目录再执行。注意 Claude Code 的 Bash 每次调用后 cwd 会重置，
`cd A && lark-cli ...` 要写在同一条命令里。

## 坑二：单次写入有体积上限，超了不报错

35277 字符一次 `overwrite` 的返回：

```json
{ "ok": true, "data": { "result": "partial_success", "warnings": [
  "degrade_code=1011,msg=Instruction produced no document changes..." ] } }
```

`ok: true`、`revision_id` 还涨了，但**文档一个字没变**。那句 warning 说的是「指令没有
产生任何改动，可能内容与当前文档相同，或格式不符合预期」——实际原因是太大。

**分片写**：先 `overwrite` 写头部，再逐片 `append`。实测 **5400 字符一片稳定成功**，
43 片连写全部 success（总量 206K）。

## 写完一定要验

```bash
lark-cli docs +fetch --doc "<url>" --doc-format markdown --as user | python3 -c "
import json,sys
c=json.load(sys.stdin)['data']['document']['content']
print('总长', len(c))
for l in c.split(chr(10)):
    if l.startswith('#'): print(' ', l[:60])"
```

只看 `result: success` 不够——上面两个坑都会返回 success 或 ok:true。

---
title: sitin-next build/deploy 踩坑
tags: [sitin-next, build, deploy]
---


## 本地 build 验证别用 tail 截断输出(2026-08-11)
- **坑**:改 app-social-proxy-server 后用 `tsc --noEmit 2>&1 | tail -15` 验证,顶部 admin.controller 的 TS 报错被截掉,只看到无关 spec 报错 → 误判通过 → deploy 时 nest build 才炸(TS2339 platform not on OneClickBody)。
- **根因**:pre-commit 只 lint 不 tsc;`tail` 丢掉了真正的编译错误。
- **修法**:验证编译用 `pnpm --filter <pkg> build` 看 exit code,或 `grep -E "error TS|Found [0-9]+ error"`(别 tail)。deploy 用的是 `prisma generate && nest build`,本地对齐这个。
- **另坑**:DTO/接口新增字段要同步声明,body.xxx 用到的字段必须在对应 interface 里(OneClickBody vs CreateTodoBody 是不同接口,别看错行)。

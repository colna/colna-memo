---
title: sitin-next build/deploy 踩坑
tags: [sitin-next, build, deploy]
---


## 本地 build 验证别用 tail 截断输出(2026-08-11)
- **坑**:改 app-social-proxy-server 后用 `tsc --noEmit 2>&1 | tail -15` 验证,顶部 admin.controller 的 TS 报错被截掉,只看到无关 spec 报错 → 误判通过 → deploy 时 nest build 才炸(TS2339 platform not on OneClickBody)。
- **根因**:pre-commit 只 lint 不 tsc;`tail` 丢掉了真正的编译错误。
- **修法**:验证编译用 `pnpm --filter <pkg> build` 看 exit code,或 `grep -E "error TS|Found [0-9]+ error"`(别 tail)。deploy 用的是 `prisma generate && nest build`,本地对齐这个。
- **另坑**:DTO/接口新增字段要同步声明,body.xxx 用到的字段必须在对应 interface 里(OneClickBody vs CreateTodoBody 是不同接口,别看错行)。

## Vercel 构建 ~6min 慢在哪：子模块指向内网 GitLab，clone 阶段等超时(2026-10-08)
- **现象**:app-pwa 的 Vercel 部署总时长 ~6min;拆解 = **clone 4m33s(占 76%)** + 缓存恢复/install×2 ~20s + turbo 6 包全 cache hit + `tsc + vite build:dev` 43s + 产物发布 ~11s。构建本身不慢,日志有 `Warning: Failed to fetch one or more git submodules`。
- **根因**:`.gitmodules` 里 `packages/business-pwa-proto/proto` → `git@gitlab.sitinai.com:packages/proto.git`(SSH);该域名**公网 DNS 解析到私网 IP `10.8.0.155`**(本地 VPN 下是 fake-IP 198.18.0.95;必须用 Cloudflare/Google DoH 查才看到真实公网记录)→ Vercel(AWS 公网)连私网地址,SSH 无短超时,只能等 TCP 超时后放弃。构建**不需要**该子模块(proto 生成物 `business-pwa-proto/src/gen` 已入库,`vercel:build` 只用 src/gen)。
- **可选修法**(本地 git 实测语义):`.gitmodules` 给该子模块加 `update = none` →
  | 命令 | 行为 |
  |---|---|
  | `git clone --recurse-submodules` | `Skipping submodule '...'`(不拉、不超时) |
  | `git submodule update --init` | 同上跳过 |
  | `git submodule update --init --checkout` | 显式覆盖,正常拉取 |
  影响全组递归 clone/init 默认跳过 proto,需团队拍板 + 部署验证(warning 消失、clone 缩短)。
- **别踩**:Vercel 项目设置**没有**子模块开关(官方 Git settings 只有 LFS / Deploy Hooks / Verified Commits);不要改/删子模块 URL(本地 proto 工作流依赖它)。

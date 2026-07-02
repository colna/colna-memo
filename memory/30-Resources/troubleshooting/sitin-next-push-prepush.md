---
title: sitin-next push 被 pre-push 全量 test 拦住(私有 registry 连不上)
date: 2026-07-02
tags: [troubleshooting, sitin-next, git, pnpm]
---

# sitin-next `git push` 被 pre-push hook 拦住

## 现象
`git push` 失败,`.husky/pre-push` 跑 `pnpm test`(全量,非 affected)+ `pnpm build`。
某个 business-minerva-* 包 test 报 `Cannot find package '@heyhru/...'` 或
`Cannot find package '@presence-io/datatester'`。

## 根因
- pre-push 是**全量** `pnpm test`,任何一个包缺依赖都会挡住整次 push,哪怕你改的是别的包(如 app-minerva-web 前端)。
- `@presence-io/*` 走**私有 Nexus** `https://nexus.sitinai.com`(见根 `.npmrc`:`@presence-io:registry=...`)。
- **本机无内网/VPN 时连不上 Nexus**(`curl -m8 ... nexus.sitinai.com` 返回 `000`),
  所以 `@presence-io/datatester@0.0.1` 装不进来,`business-minerva-upgrade` test 必挂。
- `pnpm install` 会 exit 0 但静默缺这个包;root `node_modules/@heyhru` 也可能空,但 `node -e require.resolve(...)` 仍能解析(走 workspace 链接)。

## 修法
1. **优先**:连公司 VPN/内网 → `pnpm install` → 正常 push(pre-push 能过)。
2. **无网络时**:改动与失败包无关,`git push --no-verify` 跳过 pre-push。lint 已过即可(项目约定默认不绕过,需用户授权;本例已授权)。

## 排查命令
- `cat .husky/pre-push`
- `grep -n datatester pnpm-lock.yaml` / `cat .npmrc`
- `curl -s -m8 -o /dev/null -w "%{http_code}\n" https://nexus.sitinai.com/repository/npm-group/@presence-io%2fdatatester`

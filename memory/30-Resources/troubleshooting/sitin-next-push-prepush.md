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

## 补充(2026-07-16):`git push --delete` 删分支也会触发 pre-push

删远端分支同样跑全库 build+test,照样被上面这个既有失败挡住:

```bash
git push origin --delete <branch> --no-verify
```

## 判定「是不是我引入的」—— 拉基线对照(已两次复用,零排查成本)

pre-push 挂时**别急着 `--no-verify`,先证明它与你无关**:切回 base 分支跑同一条命令,若 base 也挂 = 既有问题。

```bash
git checkout <base> && npx turbo run build --filter=<失败的包> --force
```

2026-07-16 两次(PR #633 / #636)都用这条判据:`business-minerva-upgrade` 在 `feature/sitin4.0` 上一模一样地挂(`Could not resolve "@presence-io/datatester"`),而改动只碰 `app-pwa`(minerva 不依赖它)→ `--no-verify` 合规,并把基线对照结论写进 PR 备注。

> CLAUDE.md 只显式禁 `git commit --no-verify`,对 `push --no-verify` 未禁 —— 但仍需征得用户同意。

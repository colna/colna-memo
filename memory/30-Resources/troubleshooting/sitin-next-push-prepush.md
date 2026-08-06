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

## 2026-07-27 全量 hook 被无关包拖累 → 授权 --no-verify

- sitin-next 的 pre-commit 跑**全量 turbo lint**、pre-push 跑**全量 build/test**。单包改动常被**无关包既有失败**挡下:实测 `business-minerva-users` 的 `no-useless-assignment` lint error(#679 引入)、`business-minerva-upgrade#build` 失败。
- 处理:**本包自己** `pnpm --filter <包> lint` + `tsc --noEmit -p` 验证通过后,`--no-verify` 绕过整体 hook(需用户授权;CLAUDE.md 默认禁止)。本轮多个 PR(#730/731/732/733/734/741/743)都这么走。
- **shell 坑**:切分支命令若用 `&&` 串联且首条走了管道(`git checkout ... | tail`),管道会**吞掉非零退出码** → checkout 失败仍继续;再叠加无 `cd` 前缀 **cwd 漂移**,一次误在 colna-memo 建了 sitin-next 的分支。→ 切分支命令**带绝对 `cd` 前缀**、**别把会失败的 `git checkout` 接管道**。

## 2026-08-06:`business-pwa-proto` dist 过期(切分支/pull gen 后本地 tsc 一堆假错)

- **现象**:改完 app-pwa 单独跑 `pnpm --filter @heyhru/app-pwa exec tsc --noEmit`,报一堆 `Module has no exported member 'TaskType'/'ConversationState'/'HasContactCard'`、`Property 'cardType' does not exist` —— **不是自己的改动**。
- **根因**:`business-pwa-proto` 的 **`dist`(编译产物)gitignored、不随切分支/pull 更新**,而 `src/gen`(checkin)变了 → app-pwa 引用的 `dist` 类型过期。**一天内切分支/pull gen 反复触发**(sp-snapchat↔sitin4.1、用户 push 新 gen)。
- **修法**:`pnpm --filter @heyhru/business-pwa-proto build`(tsup 秒级)重建 dist → tsc 干净。**pre-push 的 `turbo build --affected` 会先构建依赖**,所以本地这堆假错**线上不挂**,不用慌;只是本地单独 tsc 会看到。
- **判定**:报错文件/行是不是自己碰的?proto/gen 类型的成员缺失、cardType 缺属性 → 十有八九是 dist 过期,先 rebuild proto 再看。剩 `web-util-media`(GiftBubble/ChatFooter)同理,别的包 dist 过期同样 rebuild 或交给 pre-push。

## 2026-08-06:commitlint `subject-case` 拦大写英文词开头

- **现象**:`git commit` 过了 pre-commit(lint+circular)却在 commit-msg 报 `found 1 problems`,`npx commitlint` 显示 `subject must not be sentence-case, start-case, pascal-case, upper-case [subject-case]`。
- **根因**:subject 以**大写英文词开头**(如 `fix(app-pwa): CE 完单...`,`CE` 触发 upper-case)。中文开头 / 小写英文开头都 OK。
- **修法**:subject 首词用中文或小写(`fix(app-pwa): 完单按...`)。body 里的大写词(IG/SC/CE)不受限。

---
title: sitin-next
date: 2026-06-25
tags: project, sitin, monorepo, nextjs, pnpm, turborepo
---

# sitin-next

Sitin 平台主 monorepo,pnpm workspace + Turborepo 管理。

## 基本信息

- 仓库:`git@github-colna:presence-io/sitin-next.git`
- 路径:`/Users/user/Dev2/zhangzheng/sitin-next`
- 集成分支:`personal/zz/sitin4`(app-pwa 功能分支从这里切、PR base 也指这里)
- 环境:Node.js >= 20,pnpm >= 10

## 技术栈

monorepo(pnpm workspace + Turborepo + lerna),含多个可部署应用与共享包(`packages/`)。带自有 `CLAUDE.md` 与 `skills-lock.json`。

## 常用命令

```bash
pnpm install   # 首次或 lockfile 变更后
pnpm build     # 构建所有包
pnpm dev       # 启动所有开发服务
pnpm lint      # 提交前必须通过
pnpm test      # 运行测试
```

## 已完成需求

### 脚本错误日志 — 流式 CSV 导出 + 日期筛选(2026-06-30)

- **页面**:Minerva admin `/social-proxy/script-errors`(脚本执行错误日志)。
- **需求**:①修大数据量导出报错(旧实现 `error-logs?pageSize=140909` 一次性 `findMany` 拉爆内存,domSnapshot 巨大;140,949 行 / 全量 ≈1.2GB);②表格加日期范围筛选,默认最近 7 天;③导出跟随所选日期;④`target_html` 不能去掉(调试用);⑤前端加进度 Modal(MB 计数 + 可取消)。
- **后端**(`app-social-proxy-server`):新增 `GET script/error-logs/export`(必须声明在 `error-logs/:id` 之前,否则被 `:id` 捕获)。`streamCsv()` 游标分页(`orderBy [createdAt desc, id desc]` + `cursor/skip:1`),每批 2000 条,边取边写 Express 响应流,内存恒定、任意体量可导。BOM + `\r\n`,`target_html` 保留在 CSV。
- **前端**(`app-minerva-web`):`RangePicker` 默认近 7 天;导出用 File System Access API(`showSaveFilePicker` → `createWritable`)+ fetch reader 直接流式写盘,浏览器内存也恒定;非 Chromium 走 blob fallback。进度按字节计(流响应无 Content-Length,行数因 target_html 内嵌换行不可靠),Modal 显示已写入 MB + 取消(AbortController → `writable.abort()` 丢弃半成品文件)。
- **数据边界**:导出放 social-proxy-server 而非 minerva-server —— `script_error_log` 表只在 social-proxy 的 Prisma;minerva-server 是 BFF,`/api/social-proxy/*` 用 `@fastify/http-proxy` 流式透传(`reply.from` 不缓冲),故流式 CSV 经它透传内存依旧恒定。放 minerva-server 反而要多一跳或跨库,更糟。
- **PR / 分支**:
  - 特性分支 `personal/zz/script-errors-export`(从 `feature/admin@261ef60b` 切),2 个提交 `d81bd05b`(后端)+ `f2a3b11a`(前端)。
  - **PR #491** → base `feature/admin`:<https://github.com/presence-io/sitin-next/pull/491>
  - 上测试环境:cherry-pick 这 2 个提交到 `release/test-admin` 并 push(`c71e9276..b31e31c0`)。**不用 merge**:该分支基于旧的 261ef60b,feature/admin 从未合进 release/test-admin(分叉点 3d3de7e0),merge 会拖入整段 feature/admin delta 并产生回退型冲突(如 `guild-queue.ts` 会用旧版覆盖新 import 破坏构建)。

### 社媒交换「授权证据」修复 — 手动用户名落名片不再误置授权态（2026-10-09/10）

- **问题**：4.3 手动用户名路径（MyPicks 未授权接受交换）`saveContactCard` 落名片后，冷启动 `hasContactCard` 回填把未授权用户误判"已授权"→ 弹 Reconnect 抽屉 + MyPicks 三守卫失效（用户名弹窗跳过 / 自动关注 / 偷拉小崽）。
- **修复**（前端过渡；后端名片 source 字段待补）：`insStore.authEvidenceByPlatform`（仅授权回调写入）+ 回填改 `hasContactCard && 证据` + merge 存量迁移。
- **分支/提交**：`personal/zz/no-auth` `68d91457c`；已合 `release/test-pwa`（merge `2ebfda055`），push 完成。
- **文档**：`packages/app-pwa/docs/社媒交换逻辑调整-女端技术方案.md`（A2/风险表/Q5/状态行）已同步；踩坑沉淀见知识库 `troubleshooting/sitin4.1-ce-social-auth.md`。

### IG 发帖脚本（createPost：图 / 视频 / 图+视频 carousel）+ Insights 抓取 — 直调私有接口方案（2026-10-12；10-13 更名，10-14 pipeline 化，10-15 shortCode/postUrl）

- **背景**：SP 脚本新增「不点页面、直调 IG web 私有接口」发照片/视频帖（Reels），并抓取帖子 Insights；替代脆弱的 DOM 点击方案（接口方案不受页面改版影响）。
- **脚本**（`packages/app-ins-scripts`）：`createPost({files, caption, postKey})` 按素材自动分派——1 视频 → Reels 四步 + 转码轮询（GET 初始化 → `rupload_igvideo` → 封面抽帧上传 → `configure_to_clips`，202 轮询 2.5s）；1 图 → 照片两步（`rupload_igphoto` → `media/configure`）；2+ → carousel（逐项分片拉取 → magic bytes 嗅探 → 图/视频各自独立 `upload_id` 上传 → `configure_sidecar` 202 轮询）。素材经分片接口 + 原生 `requestHttp` 拉取（IG 页面 CSP 拦第三方 fetch），**分片失败自动退避重试（最多 3 次，含首片）**。`postKey` 原样透传；**result 返回 `mediaId/mediaPk/shortCode/postUrl/type`**（shortCode 优先取 configure 响应 `media.code`，兜底 `pkToShortcode` 大数编码——见 `troubleshooting/instagram-pk-shortcode.md`）。**10-14 起两个方法均改用 `SP.pipeline` 编排**（断点存 stateKey，页面重载由 recovery 重调续跑；`getMediaInsightsResume` = 再调主方法；机制见 `troubleshooting/sp-pipeline-resume.md`）；`getMediaInsights` 兼容中/英文界面，**帖子已删除（404）→ 成功 + `data:{"message":"帖子不存在"}`**，完成后统一跳转收件箱 `navigateToInbox`。错误码含 `MEDIA_LOAD_FAILED` / `IMAGE_UPLOAD_FAILED` / `PIPELINE_STALE`。
- **服务端**（`app-social-proxy-server`）：新增 `GET /script/media-chunk?url=&offset=&size=`（base64 文本分片，默认 256KB/上限 512KB，按 url 缓存 + 并发去重 + FIFO 淘汰 3 条）；⚠️ 无鉴权 + 任意 URL 拉取（SSRF 面），对外暴露前需评估。
- **分支/PR**：**#1227 已合并进 `feature/sp`**（`4a570c29c`）；后续 pipeline 改造等经 GitHub #1246 合并后异常消失（merge commit 不在任何分支，疑似合并后被 force-push 回退）→ **GitLab MR !1156 已合入 `feature/sp`**（`0bfa56c20`，`personal/zz/ins-pipeline`，5 commits：pipeline 化 / 分片重试 / 删除帖空结果+收件箱 / 页面不存在 message / shortCode+postUrl）。收尾互踩修复（`6825aab7f` 续跑自检重导航 + `a24aaa553` 完成不跳收件箱）搬到新分支 `personal/zz/ins-post-fixes`，提 **MR !1175**（base `feature/sp`）：<https://gitlab.sitinai.com/frontend/sitin-next/-/merge_requests/1175>。**仓库已迁移 GitLab**（`git@gitlab.sitinai.com:frontend/sitin-next.git`，本机 SSH key 已注册；**约定：以后所有 fetch/push/MR 一律走 GitLab，不再用 GitHub**）。功能同步 GraceChat assets（`haven/src/main/assets/scripts/instagram/`）+ 容器 app（`social-proxy-scripts-container-app` 两处 bundle），均未提交、需重建/热更生效。
- **待办**：MR !1175 review 合并；`release/test-sp` 的 retry 提交 `f9b3eaf37` **本地未推**（部署测试环境时推）；**真机 media-chunk `HTTP -1` 排查**（原生 OkHttp `SSLHandshakeException: connection closed` 时段性出现——浏览器对比 / 换网络 / VPN 检查 / 服务端网关日志；本机 GitHub 通道同样 TLS 断连佐证链路问题）；真机验证删除帖/收件箱/shortCode；carousel 视频 child `rupload_igvideo` params 补抓；`upload-all.mjs` 上传需有效 `--token`（默认 super_admin 凭据已失效）；工作区 `upload-all.mjs` 有未提交改动（默认 baseUrl dev→prod，外部改动）待确认处理。
- **沉淀**：`30-Resources/instagram-create-post-api.md`（照片/视频/carousel 全链路、headers、CSP、转码轮询）。

## 进行中 / 待收口

### IG API 收敛：insGraphql + comment/like（2026-10-26）

- **分支 / MR**：`feature/sp-ins-comment-like-api` → `feature/sp`，MR !1266；提交 `a5fd42c91`（commentPost/likePost API + pipeline）与 `43c8ac616`（getMediaInsights/toProfessionalAccount 复用 insGraphql）。
- **实现**：新增 `actions/insGraphql.js` 统一页面 context、GraphQL/REST、headers 与错误；commentPost/likePost 使用真实 mutation，旧 DOM 版保留为 `*Dom.js` 不参与构建。
- **验证**：lint/circular/build 通过；真实页面已验证点赞/取消、评论/删除、Insights 20 项、personal→creator 转换，调用均不改变页面 URL。
- **待收口**：MR !1266 已合并到 `feature/sp`（merge `ced8f5469`，2026-10-27）；Android automation/login/bootstrap 资产已同步但未 commit/push；sitin-next `build.mjs`、Android 相关改动未提交。
- **测试环境**：`release/test-sp` 已重置为 `feature/sp` 并合入 getMediaInsights API 改动（merge `cd7a3be82`），旧 tip 保留在 `backup/release-test-sp-20261026-before-reset`；部署后需复测 media-chunk 与 Story `isBake:false`。
- **2026-10-27 追加**：`followUserById` 改为 GraphQL `usePolarisFollowUserFollowMutation`（doc `27767812149509802`，`target_user_id`）+ pipeline；用户名→pk/关注态复用 `getProfileInfo`；`insGraphql` 增加「真实请求模板优先 + `for (;;);` 前缀剥离」；`executeAction` 把 `followUserById` 加入 `SESSION_ONLY_ACTIONS`。真机（`ins-script-device-test`，账号 `xuzaixing2026`）已验证：`getProfileInfo` 模板路径、`followUserById` 关注 `@instagram` 成功、二次调用 `alreadyFollowing:true`；随后账号触发 IG 人机验证，`@instagram` 关注尚未取关，需完成 CAPTCHA 后恢复。sitin-next 相关提交已全部合入 `feature/sp`（MR !1266，merge `ced8f5469`）；Android assets 已同步（5 个变更）但未 commit/push。

## 备注

- 详细应用清单 / 线上域名见仓库 README「可部署应用」表。
- 项目自带 `docs/` 与 `deploy/`。

## 进行中（2026-11-02）PWA 冷启动补 IG 授权兜底

- **背景**：冷启动 `checkSocialProxyAbnormal()` 有执行但没进 `checkSocialProxyPageAbnormal`，`abnormalByPlatform` 恒 `null`、也不弹授权悬浮窗。根因是解锁前在原生 Earn 绑定 IG 时，sticky `InsStateEvent` 被 `EarnMainActivity.onInsState()` 无条件 `removeStickyEvent` 消费掉，PWA 启动后收不到 `finishPWAInsTask` → `authEvidenceByPlatform` 为空 → 卡在门 B。取证与判据见 [sitin-pwa-sp-abnormal-null-on-coldstart](../30-Resources/troubleshooting/sitin-pwa-sp-abnormal-null-on-coldstart.md)。
- **改动**：`packages/app-pwa/src/hooks/useInsTaskInit.ts`（+31/−6）。`!igAuthed && (hasBoundInstagram() || igCard === true)` 时补探一次；`await waitForBridge()` 防 -500；只有 `code===0 && type===0` 才补写 `setPlatformAuth` / `markAuthEvidence` / `setPlatformLoggedIn` / `setPlatformAbnormal(false)`，未授权且探测失败一律不写（避免手动用户名用户误弹 Reconnect）；兜底不置 `socialProxyChecking`。
- **状态**：分支 `fix/pwa-coldstart-auth-fallback`（从 `feature/pwa` `845a2ed7f` 切出），commit `272ad2b7a`，已 push，MR [!1357](https://gitlab.sitinai.com/frontend/sitin-next/-/merge_requests/1357) → `feature/pwa`，`opened` / `can_be_merged`。eslint、`tsc -b`、`pnpm --filter @heyhru/app-pwa build` 均通过。
- **未决**：线上实际卡门 A 还是门 B 判不了，需按日志对号 `[INS_BRIDGE] setPlatformAuth:`（`insStore.ts:97`）与 `[INS_BRIDGE] setPlatformAbnormal:`（`insStore.ts:125`）——有前无后 = 门 B，两条都无 = 非 APK / 门 A。native 侧更根治的修法是不在 PWA 未 attach 时移除 sticky，或 attach 后补投。

## 进行中（2026-11-03）earning-ai 三分支集成 + 悬浮窗授权修复

### 分支与 MR 现状

| 分支 | tip | 去向 | 状态 |
|---|---|---|---|
| `feat/earning-ai` | `5a386f14f` | MR [!1360](https://gitlab.sitinai.com/frontend/sitin-next/-/merge_requests/1360) → `feature/pwa` | opened / can_be_merged / 无冲突 |
| `release/test-pwa2` | `de5afdff5` | 已 push（`4bb38cfec..de5afdff5`） | 本地与远端 `0 0` |
| `fix/pwa-coldstart-auth-fallback` | `272ad2b7a` | MR [!1357](https://gitlab.sitinai.com/frontend/sitin-next/-/merge_requests/1357) → `feature/pwa` | opened，待评审 |
| `release/test-pwa` | `42be0162c` | — | 10:27 撤销 earning-ai 合并，未 push 过 |

- `feat/earning-ai` 由 `origin/feat/gxy/pwa-native-bridge`（`eea7346ae`，已含 `feature/pwa` `845a2ed7f`）切出，再合 `origin/feat/pwa-byteplus-app-id-url`（`ab2920d33`）与 `fix/pwa-coldstart-auth-fallback`（`272ad2b7a`）→ merge `4da195319`、`ff8b4322e`。以 gxy 为基少一次大 merge，只处理 byteplus 与 fix。
- 之后两个修复提交直接落该分支：`f46469c37`（版本门→端能力门，+8/−1）、`5a386f14f`（回前台补拉小崽，+42/−1）。
- **撤销记录**：`release/test-pwa` 上误建的 merge `af23d240c` 已 `git reset --hard origin/release/test-pwa` 撤销（仍在 reflog 可恢复）；该 merge 从未 push。

### push 被拒的标准处理

12:52 推 `release/test-pwa2` 被拒：远端已前进 11 个提交（`fix/pwa-online-hotfix-1103` + profile 修复）。做法：`git fetch` → 确认 merge-base 仍是自己上次推送的 `ff8b4322e`（证明远端没回退/改历史）→ 本地仅 `f46469c37` 独有 → `git merge --no-ff FETCH_HEAD` 干净合并为 `4bb38cfec` → 再 push。**不在未确认 merge-base 前 rebase 或 force**。

### 代码改动（web）

- `packages/app-pwa/src/hooks/useInsTaskInit.ts` —— `initInsFollowedUsers` 版本门改能力门（`hasNativeMethod("checkSocialProxyPageAbnormal") && hasNativeMethod("startSocialProxyRobot") || version>=1.34`）；新增 `visibilitychange` 回前台补拉 `startRobot`（`floatingRetryRef` 防并发、`miniShown` 防重复、延迟 1.2s）。
- Android `earning-ai-android` `release/v1.0.0`（`57c78f0`）本地 debug 配置**未提交**：`LocalTestConf.pwaHost` → `http://localhost:3000`、`app/src/debug/AndroidManifest.xml` 加 `usesCleartextTraffic="true"`。
- 坑与根因已沉淀：[pwa-host-version-gate](../30-Resources/troubleshooting/pwa-host-version-gate.md)、[pwa-localhost-port-conflict](../30-Resources/troubleshooting/pwa-localhost-port-conflict.md)（adb reverse 双端端口不同）。

### 未决

- [ ] MR !1357 / !1360 评审合并。
- [ ] PWA 冷启动实际卡门 A 还是门 B：对号 `[INS_BRIDGE] setPlatformAuth:`（`insStore.ts:97`）与 `[INS_BRIDGE] setPlatformAbnormal:`（`insStore.ts:125`）。
- [ ] `bridge.ts` 的 `checkInsPageAbnormal` 二级降级在端能力侧新旧 APK 是否存在（2026-11-02 21:15 专家回报被截断，无结论）。
- [ ] 权限授予后「Allow System Access」弹窗不自动关闭（建议 `InsAuthPermissionModal.tsx:133` granted 后自动 close）。

## 进行中（2026-11-04）IG clickMute 改纯 API 静音

- **背景**：旧 `packages/app-ins-scripts/src/instagram/methods/clickMute.js` 走 DOM 流程（主页「发消息」→ 会话信息 → 开关 → 时长弹窗），依赖 3 段易变 DOM 且必须进聊天页；CE 交换后立刻静音时页面状态不稳定。改纯接口后不依赖导航，直接用后端已下发的 handle。
- **实现**（分支 `feature/sp-ins-click-mute-api`）：`web_profile_info` 取 `userId` → `direct_v2/threads/get_by_participants` 取/建 `threadId` → `POST threads/{id}/mute/` → 回读 `thread.muted` 校验。入参兼容后端 `MUTE_USER` 下发的 `manSocialId` / `userIgHandle` / `manSocialNickname`；无参时回退解析当前 profile URL 的 username。已把 `clickMute` 加入 `executeAction.js` 的 `SESSION_ONLY_ACTIONS`。
- **分支 / MR**：`4fad0451b feat(app-ins-scripts): clickMute 改为纯 API 静音` + `8890e2328 chore: 忽略 colna fastembed 缓存` → **MR [!1381](https://gitlab.sitinai.com/frontend/sitin-next/-/merge_requests/1381)**（→ `feature/sp`，push option 建 MR，`remove_source_branch`），已 push。
- **验证（已做）**：`node scripts/build.mjs` 通过；Node + `vm` 模拟 `insGraphql.rest` 跑 **7 个用例**全过（正常静音 / 已静音短路 / 校验仍 false / mute 接口失败 / 用户不存在 / 无会话 / 非 profile 页传参）。未改 `dist`（gitignore）。
- **未验 / 待确认**：
  - `get_by_participants` 在**无历史会话**时是否自动建 thread——私有接口未文档化，需真机或网页实测。
  - `thread.muted` 回读字段若变更，会走 `verified:null` 的成功分支（静默降级）。
  - **真机联调仍被 IG 风控阻塞**：`update_risky_contactpoint`，MR 描述已注明。
  - `packages/business-pwa-proto/proto` 子模块指针未提交，未纳入该 MR。
- **踩坑**：首次 push 被 pre-push 全量 hook 拦（`business-minerva-ug-core` 缺依赖）→ `pnpm install --frozen-lockfile` 后 71 tasks 全过，详见 [sitin-next-push-prepush](../30-Resources/troubleshooting/sitin-next-push-prepush.md)。

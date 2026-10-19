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

## 备注

- 详细应用清单 / 线上域名见仓库 README「可部署应用」表。
- 项目自带 `docs/` 与 `deploy/`。

---
title: aichat-hindsight 去掉 Nacos 通道并清理公司基础设施
date: 2026-11-26
tags:
  - aichat-hindsight
  - nacos
  - local-chat
  - deepseek
  - hindsight
  - architecture
---

# aichat-hindsight：去掉 Nacos 通道并清理公司基础设施

> 状态：技术方案成稿（只出方案，未改 `aichat-hindsight` 代码，未 commit/push，未写飞书）。
> 依据：本机仓库 `/Users/colna/WORK/aichat-hindsight`，`main` @ `8f86340`（`origin` → `https://github.com/colna/aichat-hindsight`）。工作区另有未提交改动 `src/main/resources/chat-config.jsonc`，本方案不触及。
> 知识库检索：今日 Daily 与 `30-Resources/troubleshooting/aichat-v2-hindsight.md` 记录了本地 DeepSeek + Hindsight 接入与踩坑；**未查到「彻底删除 Nacos」的既有决策文档**。以下以现码为准，既有笔记仅作背景。

## 1. 结论摘要

用户已确认本地聊天不依赖 Nacos，要求把通道彻底拿掉，并删掉不需要的公司基础设施，保持纯净代码。目标形态仍是：本机 DeepSeek + Hindsight + Redis + Agent Loop + `chat-config.jsonc`。

**必须在本轮清理落地（不可只关开关）：**

- 删除整个 Nacos 通道：`nacos-client` / `nacos-client-mse-extension` 依赖、`NacosConfigLoader`、`AppConfig.resolveAppConfig` 的 Nacos 拉取、`Application` 里所有 `nacosLoader` 监听与 `aichat_v2_router_profiles` 加载、硬编码默认地址 `api.nacos.archat.us`、全部 `NACOS_*` / `RAM_ROLE_NAME` 环境变量、`application.yaml` 的 `infra.nacos` 明文凭证、文档与注释里把 Nacos 当作主配置源的叙述。
- **不要留下** `if (nacosLoader != null)` / `NACOS_DISABLED` 空壳分支。
- 配置只走 `LocalEnv` / `.env` / 代码默认值 / `chat-config.jsonc`。
- DeepSeek 本地 provider 改成**正式路径**，不再写成「Nacos 路由为空才 fallback」。
- Redis 只走 env URI（经 `LocalEnv`，从而也能读仓库根目录 `.env`），删除 `RedisProvider.fromNacosConfig`。
- 热更新：完全放弃 Nacos listener。第一期以进程重启为准；可选二期再做 `chat-config.jsonc` 文件重读，不是本轮阻塞项。

**建议同步清掉、否则会留下走不到的公司死代码（见第 8 节，均有现码依据）：** Tianshu 同步/轮询客户端、BytePlus AB SDK 与事件上报配置通道、只从 Nacos 取密钥/开关的 trace-viewer 推送、公司 Kafka 消费者/生产者及 GCP Kafka 鉴权依赖、只为 Nacos 热更新存在的 autocorrect overlay 监听。

**必须保留：** `io.presence` 包名与 `sitin.*` protobuf/gRPC 协议包名；`LocalEnv`、`.env.example`、`scripts/local-chat.sh`、`chat-config.jsonc`、Hindsight hooks、Agent Loop / agentkit、classpath PromptRouter。

**回归底线：** `./scripts/local-chat.sh` 在不连 Nacos 的情况下仍能启动，本地对话主路径可用（`POST /api/agent/run` + DeepSeek + Redis 历史 + Hindsight retain/recall/reflect）。

## 2. 范围与硬约束

| 约束 | 说明 |
|------|------|
| 本方案阶段 | 只出方案。落地由后续 Coder 执行；落地时默认也不自动 commit/push，除非用户之后明确要求。 |
| 只动 aichat-hindsight | 不要改 Onia、sitin-next、aichat-v2 或其他业务仓。 |
| 远程 | `origin` 已指向 `colna/aichat-hindsight`；`presence-io` 的 push 已禁用。绝对不要推到 `presence-io/aichat-v2`。 |
| 包名 | 不改 `io.presence` 与 `sitin.*` protobuf/gRPC 协议包名。 |
| 密钥 | 不读、不回显真实 `.env`。文档只列键名。`application.yaml` 里现有的 Nacos 明文账密必须删除，但不得把旧值抄进新文档或新配置。 |
| 性能数字 | 不编造 QPS、延迟、容量、可用性。下文不引用无来源的运行指标。 |

## 3. 现码调研结论

### 3.1 当前配置解析顺序（`resolveAppConfig`）

见 `src/main/kotlin/io/presence/aichat/config/AppConfig.kt`（约 L698–L1040）与 `NacosConfigLoader.kt`。

1. 若 `LocalEnv.flag("NACOS_DISABLED")` 或 `NACOS_SERVER_ADDR=off`：跳过 Nacos，打日志 `[Nacos] skipped ... using env fallbacks`，`nacosLoader` 保持 `null`。
2. 否则：`NacosConfigLoader.create()` 连接默认地址 `api.nacos.archat.us`（可被 `NACOS_SERVER_ADDR` 覆盖），按 `NACOS_NAMESPACE` / `ENV` 映射到 `dora-*` namespace，用 `NACOS_USERNAME`/`NACOS_PASSWORD` 或 `RAM_ROLE_NAME`（阿里云 MSE）鉴权。凭证还会从 `application.yaml` 的 `infra.nacos.username/password` 传入（`Application.kt` L205–L208）。
3. 一次拉取并监听：
   - `global_db_config`：Redis（含 CA/SSL 的 `fromNacosConfig`）、Kafka `bootstrap_servers`、Postgres、RAG Postgres。
   - `aichat_v2_feature_flags`：大量 FeatureFlags、Tianshu `tianshu_base_url` / `tianshu_active`，以及 ConfigHolder 里一串子对象（chase / compass / recovery 除外的多数热更新项）。
   - `common_config`：LLM 密钥（`ai_config`）、BytePlus AB（`byteplus_ab` + `byteplus_key_config`）、`obs_config`、`byteplus_event_config`。
4. 另在 `Application.kt` 用 `nacosLoader?.let` / `nacosLoader?.addListener` 加载并热更新：`common_config`（AB SDK 客户端）、`recovery_config`、`aichat-autocorrect-overlay`、`aichat_v2_router_profiles`（LLM providers + profiles + arena）。
5. Nacos 跳过或失败后，env 兜底目前**不统一**：
   - Redis / Kafka / Postgres：`System.getenv("REDIS_URI")` 等，**不走 `LocalEnv`**，因此未先 `source .env` 或未由 `local-chat.sh` 导出时，即使仓库里有 `.env` 也读不到 Redis URI。
   - DeepSeek / Hindsight：已走 `LocalEnv`（`getenv` 优先，然后向上查找 `.env`）。
   - FeatureFlags：仅部分键支持 `System.getenv` 覆盖，且有的只能把已是 `true` 的默认值关成 `false`。
   - Tianshu / BytePlus AB / obs_config / router profiles / overlay / recovery_config：**没有 env 入口**。Nacos 关闭后这些就是默认值或直接不启动。

`scripts/local-chat.sh` 与 `.env.example` 默认 `NACOS_DISABLED=true`，并导出 `REDIS_URI` / `AICHAT_REDIS_URI`、DeepSeek、Hindsight。这是今天能跑起来的原因，不是「Nacos 通道已删除」。

### 3.2 DeepSeek 仍是附属 fallback

`src/main/kotlin/io/presence/aichat/llm/LocalDeepSeekFallback.kt` 与 `Application.kt` L475–L526：

- 仅当 `nacosLoader != null` 时从 Nacos 拉 `aichat_v2_router_profiles` 并注册 listener。
- 然后无论 Nacos 是否存在，都调用 `LocalDeepSeekFallback.maybeApply`。
- `maybeApply` 在 `router.clients` 与 `router.profiles` **都非空**时直接 return（日志：`Nacos profiles present — skip local DeepSeek fallback`）。
- 只有「Nacos 没配上 provider」且 `DEEPSEEK_API_KEY` 非空时，才注册 `deepseek-local`（OpenAI 兼容，`deepseekThinkingOff=true`），并把它同时写入 HEAVY / LIGHT / IMAGE 三个 weight。

`docs/local-chat.md` 也写明：「DeepSeek provider 只在 Nacos router profiles 为空时作为本地 fallback 注册」。这正是要改掉的语义。

另外，`application.yaml` 仍把默认 LLM 写成 `defaultProvider: airouter`、`defaultModel: gemini-2.5-flash`，与本地主路径不一致，应一并改为 DeepSeek。

### 3.3 本地对话主路径（必须保住）

启动：`scripts/local-chat.sh` → 检查 JDK 21、导出 `.env`、要求 `DEEPSEEK_API_KEY`、默认跳过 Nacos、检查/拉起本机 Redis 6379、检查 Hindsight `:8888`、`./gradlew run`。

运行时（均已有现码，且不依赖 Nacos）：

- Agent Loop + `POST /api/agent/run`（见 `docs/local-chat.md`、`scripts/local-chat-repl.sh`）。
- Prompt：`PromptModuleLoader` / `ChatConfig` 从 classpath `chat-config.jsonc` 加载（`promptModules`、personas、emojiPolicies、lifeEvents、autocorrect、archetypes 等）。Tianshu `baseUrl` 为空时根本不启动 poll，live prompt 已经走 classpath。
- LLM：上述 DeepSeek fallback（本轮要改成正式路径）。
- 短期历史：Redis（`REDIS_URI` / `AICHAT_REDIS_URI`）。
- 长期记忆：`HindsightConfig.fromEnv()` + `HindsightRecallHook`（pre-loop）+ `HindsightRetainHook`（post-loop，异步 retain，每 N 次 reflect）。`HINDSIGHT_ENABLED=true` 才启用，fail-open。

### 3.4 离开 Nacos 后，五项重点模块的实际可达性

| 模块 | 离开 Nacos 后能否走到 | 依据 |
|------|------------------------|------|
| Tianshu 同步 | **走不到。** `TianshuConfig.baseUrl` 默认空串，只从 Nacos `aichat_v2_feature_flags.tianshu_base_url` 读取，**无 env 覆盖**。`Application.kt` L629：`baseUrl.isNotBlank()` 才注册 `TianshuPromptClient` / `TianshuRecipeClient`。 | `AppConfig.kt` L37–L55、L830–L841；`Application.kt` L609–L657；仓库根目录 `tianshu-sync/`、`scripts/sync_prompts_to_tianshu.py` 是公司控制面工具，不在 local-chat 启动链上。 |
| BytePlus AB / 事件上报 | **走不到有效路径。** `ABTestConfig.enabled` 默认 `false`，apps 为空；凭证只从 Nacos `common_config.byteplus_ab` + `byteplus_key_config` 解析。`ABTestResolver` 在 `enabled=false` 时直接返回 `defaultValue` / `status=disabled`。`EventReporter.create` 在 `enabled=false` 或无 endpoint/无 apps 时返回 `NOOP`。 | `ABTestConfig.kt`；`ABTestResolver.kt` L104、L152、L215；`EventReporter.kt` L27–L30；`AppConfig.kt` L90–L111、L861–L887。依赖 `com.bytedance:datatester-java-sdk:2.0.26`。 |
| 只从 Nacos 取密钥/开关的观测推送 | **HttpObsReporter / company trace-viewer 走不到。** `ObsConfig.enabled` 默认 `false`，`traceViewerEndpoint` 默认空，只从 Nacos `common_config.obs_config` 填充，无 env。 | `AppConfig.kt` L113–L135；`Application.kt` L225–L239。注意：`otelEnabled` **默认 true**（不依赖 Nacos），`GET /metrics` Prometheus 也不依赖 Nacos。这两项与「Nacos 取密钥的推送」要分开处理。 |
| 公司 Kafka 消费者 | **走不到。** `kafkaBootstrap` 只来自 Nacos `global_db_config.kafka.bootstrap_servers` 或 `KAFKA_BOOTSTRAP`。`local-chat.sh` / `.env.example` **不设**该变量。`Application.kt` 里 `RiskEventConsumer`、`MsgIngestedConsumer`、`MaleDhReplyConsumer`、`TypingService`、`ChatRoundProducer`、`TimEventProducer` 全部在 `kafkaBootstrap != null` 时才创建。`docker-compose.yml` 明确省略 Kafka，并注明客户端按 GCP SASL_SSL / OAUTHBEARER 鉴权，本地明文 broker 也无法满足。 | `AppConfig.kt` L752–L755、L959–L961；`Application.kt` L868、L881、L1124、L1175、L1186、L1198；`docker-compose.yml` L12–L14；`build.gradle.kts` L180–L181（`kafka-clients` + `managed-kafka-auth-login-handler`）。 |
| 只为 Nacos 热更新存在的 overlay | **Nacos overlay 走不到；基线词典仍在。** `Application.kt` L273–L281 仅在 `nacosLoader != null` 时拉取/监听 dataId `aichat-autocorrect-overlay` 并 `InputMethodFormatterHook.applyOverlay`。Hook 本身仍挂在 post-loop **最后一位**，使用 JAR / `chat-config.jsonc` 的 `autocorrect` 基线。 | `format/InputMethodFormatter.kt` L32；`hooks/InputMethodFormatterHook.kt` L47；`Application.kt` L813。 |

### 3.5 一个额外的现码事实（本地启动副作用）

Nacos 关闭后，若干 FeatureFlags **默认仍为 true**，且 local-chat 会配上 Redis，于是会实际注册后台任务，并尝试连公司 gRPC 默认主机（失败则 warn）：

- `proactiveHourlyEnabled=true`、`proactiveMinuteEnabled=true`、`videoTipsEnabled=true`、`videoTipsConsumerEnabled=true`、`hangUpGateEnabled=true`（`FeatureFlags` 默认值，`AppConfig.kt` L169–L244）。有 Redis 时，`Application.kt` 会注册 proactive scheduler / follow-up executor / VideoTips（Kafka 消费者仍因无 bootstrap 跳过）。
- MsgCenter / ContactExchange / LiveService / ChatService / Checker / HangUp 等 gRPC 客户端在启动时用 **Kubernetes 服务名默认值** 建连，例如 `aichat-msgcenter-grpc:10086`、`contact-exchange-api-grpc:9096`、`sitins-checker-eval-grpc:10086`，未设置对应 `*_HOST` 也会 `try` 一次，失败打 warn（`Application.kt` L345–L438、L949–L986）。

这不影响「主路径能聊」，但与「纯净本地项目」不一致，建议在落地时把本地默认姿态改成「未显式配置则不建连、公司向开关默认关」，见第 8.3 节与风险。

## 4. 迁移后的配置解析顺序

删除 Nacos 后，`resolveAppConfig`（建议改名语义为「解析本地配置」，函数名可保留）只做：

```
LocalEnv / 进程环境 / 仓库 .env
    ↓
代码内安全默认值（FeatureFlags、ObsConfig 等）
    ↓
chat-config.jsonc（classpath；prompt / persona / autocorrect / archetypes 等）
    ↓
缺失的基础设施保持关闭或内存回退（Redis 未配 → 现有 in-memory；Kafka 不再存在）
```

具体规则：

1. **统一读配置入口为 `LocalEnv`**（已有实现：`getenv` 非空优先，否则解析 `AICHAT_DOTENV` 或从 `user.dir` 向上最多 6 层找 `.env`；不打日志打值）。Redis / FeatureFlags / Postgres 今天走 `System.getenv` 的地方都改过来，避免 `local-chat.sh` 与直接 `./gradlew run` 行为分叉。
2. **不再出现 Nacos 分支**，包括 `nacosDisabled`、`nacosLoader` 字段、`if (nacosLoader != null)`。
3. **不把公司集群地址写进默认值**（包括 `api.nacos.archat.us`、gRPC 的 k8s 主机名若按第 8.3 节处理）。
4. `chat-config.jsonc` 继续做 prompt 与静态内容真源；第一期不必把 FeatureFlags 或 LLM router 强行塞进该文件。若二期要可编辑开关，可新增顶层分区（例如 `featureFlags` / `llmProviders`），但那是增强，不是删除 Nacos 的前置条件。

## 5. DeepSeek 改为正式路径

建议（实现细节可在落地时微调，语义必须如下）：

1. 将 `LocalDeepSeekFallback` 重命名为更中性的名字（例如 `LocalDeepSeekProvider` 或 `DeepSeekLocalBootstrap`），删除「Nacos-less fallback」「production composition stays unchanged」这类注释与日志。
2. 在 `Application` 装配 `LLMRouterImpl` 时**直接**调用正式注册函数：用 `LocalEnv` 读取 `DEEPSEEK_API_KEY` / `DEEPSEEK_BASE_URL`（默认 `https://api.deepseek.com`）/ `DEEPSEEK_MODEL`（默认 `deepseek-flash`），构建 `deepseek-local` OpenAI 兼容客户端，写入 HEAVY / LIGHT / IMAGE。保留现有 `deepseekThinkingOff=true`（官方 DeepSeek 拒绝 tools + thinking 同时开启，这是本地 Agent Loop 能跑的现码原因，不是 Nacos 逻辑）。
3. 若 `DEEPSEEK_API_KEY` 为空：启动期明确报错/警告（可沿用今天「local chat will fail at the LLM boundary」的精神），**不要**再提 Nacos。
4. 删除 `aichat_v2_router_profiles` 的加载与 listener。第一期不保留「文件版 Nacos profiles」兼容层，除非落地时发现测试强依赖该 JSON 形状（见 `LLMRouterProfilesTest.kt`：测试若只测 merge/parse，应改成读夹具，而不是保留 Nacos 加载）。
5. 同步修改 `application.yaml` 的 `aichat.llm` / `aichat.agent` 默认 provider/model 叙述，以及 README / CLAUDE.md / `docs/local-chat.md` / `docs/architecture.md` 中「Nacos 热更新 profiles」的说法。
6. `ArenaRouter` 依赖 AB。AB 关闭后它应自然变成直通或等价于内层 router。第一期以「不挡主路径」为准，不必在同一次改动里删除 Arena 全部代码（见暂留项）。

## 6. Redis 只走 env

1. 删除 `RedisProvider.fromNacosConfig`（含 PEM CA、以及 `EnvUtils.isLocalEnv()` 时设置 JVM 全局 SOCKS `127.0.0.1:12348` 走跳板机的逻辑——这是公司本地连集群的残留，正式本地路径不应再触发）。
2. 只保留 `fromUri`。`resolveAppConfig` 用 `LocalEnv.get("REDIS_URI")` / `AICHAT_REDIS_URI` / `AICHAT_REDIS_READ_URI`。未设置则该 provider 为 `null`，沿用现有「in-memory / 无专用 Redis」语义，**不要在代码里偷偷默认 `redis://localhost:6379`**（以免单测 `AppConfig(redisProvider=null)` 行为被改变）。本地默认值继续写在 `.env.example` 与 `local-chat.sh`。
3. `PostgresProvider.fromNacosConfig` 同样删除；若保留 Postgres，只留 `fromEnv()`，并改为 `LocalEnv` 读 `PG_URL` / `PG_USERNAME` / `PG_PASSWORD`。`local-chat.sh` 当前不起 Postgres，`ragEnabled` 默认 `false`，因此 Postgres 不是主路径必需。

## 7. 热更新：完全放弃及替代

Nacos listener 覆盖的 dataId 至少包括：`aichat_v2_feature_flags`、`common_config`、`recovery_config`、`aichat_v2_router_profiles`、`aichat-autocorrect-overlay`。全部删除，不留空 listener。

`ConfigHolder`（`AtomicReference` 容器 + 解析函数）**建议保留**：它同时是启动期写入的进程内配置面，很多 hook 从这里读 chase/compass/flags。只是不再被 Nacos 回调更新。启动时用默认值 + env 覆盖写入一次即可。

替代方式：

| 方案 | 何时用 |
|------|--------|
| **进程重启**（第一期唯一承诺） | 改 `.env`、改 FeatureFlags、换 DeepSeek model/key、改 `chat-config.jsonc` 后重启 `local-chat.sh`。 |
| 可选二期：重读 `chat-config.jsonc` | 若本地调 prompt 很频繁，可加一个调试接口或文件监视，调用已有的 `PromptModuleLoader` / `PromptRegistry.replaceHotModules`。**不是删除 Nacos 的前置。** |
| 不引入新的远程配置中心 | 不要用另一套云配置替换 Nacos。 |

## 8. 建议删除 / 建议保留 / 建议暂留待确认

判定规则：离开 Nacos 后若某模块仍能靠 env / `chat-config` 工作，**且属于本地对话主路径**，必须保留。现码无法确定是否走得到的，标「建议暂留待确认」，不写成必删。

### 8.1 建议删除（必删 + 强烈建议同期清掉的死代码）

**A. Nacos 通道本身（必删）**

| 项 | 现码依据 |
|----|----------|
| `NacosConfigLoader.kt` 整文件 | 唯一 SDK 封装；硬编码 `DEFAULT_SERVER_ADDR = "api.nacos.archat.us"`；`NACOS_SERVER_ADDR` / `NACOS_USERNAME` / `NACOS_PASSWORD` / `NACOS_NAMESPACE` / `RAM_ROLE_NAME`；`ENV`→`dora-*` namespace 映射。 |
| `AppConfig.nacosLoader` 字段及 `resolveAppConfig` 的 Nacos 拉取/listener 注册整段 | `AppConfig.kt` L34、L718–L938。 |
| `Application.kt` 所有 `nacosLoader` 用法 | L205–L208 读 yaml 账密；L249–L281 AB/recovery/overlay listener；L485–L523 router profiles 加载与热更新。删后不要留 `?.addListener` 空壳。 |
| Gradle：`nacosVersion`、`nacos-client`、`nacos-client-mse-extension`、`-Pcloud=aliyun` 仅为 MSE 引入的分支 | `build.gradle.kts` L61、L72–L73、L161–L171。 |
| `application.yaml` 的 `infra.nacos` 整段 | 含明文 username/password，既是通道残留也是密钥风险。 |
| `NACOS_DISABLED`、全部 `NACOS_*`、文档把 Nacos 当主配置源的叙述 | `.env.example` L28–L29、`scripts/local-chat.sh` L51、`README.md` 架构图与 dataId 表、`CLAUDE.md` Environment / Config resolution、`docs/local-chat.md`、`docker-compose.yml` 注释、`docs/architecture.md` 等。 |
| `RedisProvider.fromNacosConfig` / `PostgresProvider.fromNacosConfig` | 只为 Nacos JSON 形状存在。 |

**B. Tianshu（建议删除集成，保留 classpath prompt）**

离开 Nacos 后 poll 客户端不会启动（无 `tianshu_base_url` env）。本地 live prompt 已经来自 `chat-config.jsonc`。Tianshu 是公司 prompt 控制面，不属于本地主路径。

建议删除：`TianshuPromptClient.kt`、`TianshuRecipeClient.kt`、`TianshuLiveState.kt` 及其单测（`TianshuPromptClientTest.kt`、`TianshuRecipeClientTest.kt`、`TianshuShadowDiffIntegrationTest.kt`）；`Application.kt` 中 `tianshuConfig.baseUrl` 装配块；`TianshuConfig` 数据类；`PromptRouterHook` 里 serve-Tianshu / shadow-diff 分支（简化为只走 classpath）；仓库根 `tianshu-sync/`；`scripts/sync_prompts_to_tianshu.py`。

**必须保留：** `PromptModuleLoader`、`PromptRegistry`、`PromptRouter`、`chat-config.jsonc`、非 Tianshu 的 prompt hooks。

`PromptSignalRoutes` / `AgentkitCatalogRoutes` 注释写明「供 tianshu control plane 轮询」。若落地时确认没有本地调试价值，一并删除；若 agentkit 本地排障还要用 catalog，可暂留路由、去掉 Tianshu 措辞。标入第 8.3 节由落地时快速确认。

**C. BytePlus AB 配置通道与 SDK（建议删除实现，第一期可留 no-op 门面）**

凭证与总开关只来自 Nacos。本地路径上所有 `resolveExperiment(...)` 已回退到 `defaultValue`。建议删除：`com.bytedance:datatester-java-sdk` 及为它引入的专用依赖（以 `build.gradle.kts` L173–L177 为准，注意 `protobuf-java` / `caffeine` / `httpclient5` 是否还被别处使用，删除前要核对，不能误删）；`BytePlusEventReporter.kt`；`common_config` 解析 AB / event 的代码；`ABTestMain.kt`（若仅为手工打 BytePlus 的入口）。

为避免一次 PR 改几百处 hook 调用点，**第一期建议保留 `ABTestResolver` 作为永久 no-op**（`enabled` 恒 false，只返回默认值），让预/后置 hook 继续编译。第二期再视需要拆除 AB 分支。不要把「保留 no-op 门面」写成继续依赖 BytePlus 网络或密钥。

**D. 公司 Kafka（建议删除）**

本地主路径是 HTTP Agent Loop，不经过 TIM/msgcenter/Kafka。无 bootstrap 时整条链本来就不装配。建议删除消费者/生产者实现与依赖：`kafka/RiskEventConsumer.kt`、`ingest/MsgIngestedConsumer.kt`、`maldh/MaleDhReplyConsumer.kt`、`delivery/TypingService.kt` 中的 KafkaProducer 路径、`delivery/TimEventProducer.kt`、`chatround/ChatRoundProducer.kt`；`org.apache.kafka:kafka-clients`、`com.google.cloud.hosted.kafka:managed-kafka-auth-login-handler`；仅为后者存在的 `https://packages.confluent.io/maven/` 仓库（若删除后无其他包需要）。`KAFKA_BOOTSTRAP` 与相关 feature 开关（`chatRoundEnabled`、`timEventBroadcastEnabled`、`maleDhReplyEnabled`、`videoTipsConsumerEnabled`）一并从配置面移除或变为无操作。

Male-DH / ChatRound / RiskEvent 的**非 Kafka 领域代码**是否整包删除，见第 8.3 节（避免把「删 Kafka 通道」扩大成未经确认的产品功能删除）。

**E. Nacos-only overlay 监听（建议删除监听，保留 Hook + 基线）**

删除 `Application.kt` 对 `aichat-autocorrect-overlay` 的 get/listen，以及仅服务于该 dataId 的注释。保留 `InputMethodFormatterHook` 与 `chat-config.jsonc` 的 `autocorrect` 基线（它在本地主路径 post-loop 上）。若以后要本地增补词表，再加本地文件即可，不必预留 Nacos 形状。

**F. 只从 Nacos 取开关的观测推送（建议删除推送，保留本地可观察性）**

删除 `obs_config` 解析、`HttpObsReporter` 指向公司 aichat-trace-viewer 的推送装配（或让工厂永远走 NOOP，并删除只为该推送存在的代码）。保留：`GET /metrics`、`observability-sdk:metrics` 的进程内池指标。OTel 手动 span 改为 **env 显式开启**（今天 `otelEnabled` 默认 true，不依赖 Nacos；本地可保留默认开或改成默认关，但必须不再写「由 Nacos obs_config 控制」）。

### 8.2 建议保留（本地对话主路径或硬约束）

| 项 | 理由 / 依据 |
|----|-------------|
| 包名 `io.presence.*`、协议包 `sitin.*` | 用户已接受先保留。 |
| `LocalEnv`、`.env`（gitignore）、`.env.example` | 正式配置入口。 |
| `scripts/local-chat.sh`、`scripts/local-chat-repl.sh`、`docs/local-chat.md` | 本地启动与对话方式；需改掉 Nacos 措辞与 `NACOS_DISABLED`。 |
| `chat-config.jsonc` + `ChatConfig` / `PromptModuleLoader` / `PromptRouter`（classpath） | 本地 prompt 真源，已不依赖 Nacos。 |
| DeepSeek 客户端与 router 装配（改成正式路径后） | 本地 LLM。`DeepSeekClient.kt` / `LLMClientFactory` 保留。 |
| Hindsight：`HindsightClient`、`HindsightConfig.fromEnv`、`HindsightRecallHook`、`HindsightRetainHook` | `Application.kt` L716–L757、L797；`LocalEnv` 的 `HINDSIGHT_*`。 |
| Agent Loop、`agentkit` 模块、`POST /api/agent/run` | 本地对话核心。 |
| Redis `fromUri` + Lettuce | 短期历史等；由 env 配置。 |
| `ConfigHolder` 进程内持有者（去掉 Nacos 更新） | 被大量 hook 读取；启动写一次即可。 |
| `FeatureFlags` 数据结构 | 改数据源，不先拆类型；但默认值建议按第 8.3 / 第 13 节调整。 |
| `InputMethodFormatterHook` + autocorrect 基线 | 在主路径 post-loop。 |
| `docker-compose.yml` 的 Redis（及按需的 pgvector） | 本地基础设施。第一期主路径不必强制起 Postgres。 |
| AIPolice 等已在 Agent Loop 内、且可用默认值/本地 LLM 工作的安全钩子 | 属于对话主路径；其中 BytePlus 分桶在 AB no-op 后走默认/force 开关（`aiPoliceRepetitionForceV2` 默认 true，不依赖 Nacos）。 |

### 8.3 建议暂留待确认（现码无法判定为「主路径必需」或「删除不会误伤」）

这些离开 Nacos 后大多是 fail-open / 跳过 / 打 warn，不阻塞开聊，但删除面大、与 hook 链耦合深。**不要在「去掉 Nacos」的必做范围里整包删除**，除非用户另行确认。

| 项 | 现码依据 | 为什么暂留 |
|----|----------|------------|
| 公司 gRPC 客户端及 sitin SDK 依赖（MsgCenter 历史/RAG/HangUp、ContactExchange、LiveService、ChatService、Checker、Aichatv2 SyncReply server） | `Application.kt` 启动即用 k8s 默认主机建连；协议包名按约束保留 | 主路径 HTTP 对话不需要它们；但 ConversationContextHook 等仍持有 client 引用，fail 后为 null。建议**至少改成「仅当显式设置 `*_HOST` 才建连」**，整包删除等确认。公司 Maven（GAR `asia-maven.pkg.dev/heyhru-server/presence-server`、Nexus `nexus.sitinai.com`）在仍编译 sitin SDK 时也暂留。 |
| 主动消息 / chase / recovery / VideoTips / selfie / male-DH 业务域（Kafka 除外） | FeatureFlags 部分默认 true，有 Redis 就会注册 scheduler；业务本身面向 PWA/TIM | 不是「用 DeepSeek 聊一局」所必需，但代码量大、和 hook/dispatch 交织。建议第一期把本地默认开关改为 false，代码暂留。 |
| Arena / 多 provider 客户端（Gemini、Claude、Bedrock、Grok、Vertex、AIRouter） | `application.yaml` 仍默认 airouter；Nacos profiles 删除后本地不会注册它们，除非另配 | 以后若要在本地加第二模型，这些客户端有用。第一期改默认即可，不强制删客户端文件。 |
| `ABTestResolver` 调用点、各 `*AbKeys.kt` | AB 关闭后已是默认值 | 与 hook 编译耦合；第一期 no-op，第二期再清。 |
| RAG + Postgres/pgvector | `ragEnabled` 默认 false；`fromEnv` 需 `PG_URL`；compose 提供了 pgvector | 可选本地能力，不是当前主路径。 |
| TIM 回调、SocialProxy、Offline ticket（luncharity） | ticket 配置只从 Nacos `offline_ticket_config` 热加载，默认 enabled=false 且 baseUrl 空则关 | 离开 Nacos 后基本关着；是否整包删除待确认。 |
| e2e 目录（含连公司 dev Kafka 的用例） | `e2e/environment.md` 写死公司 broker 与 VPN | 与本地主路径无关；删除 Kafka 后这些脚本必然失效。可随 Kafka 删或另开清理。 |
| `observability-sdk` 其余部分、OTel API | `/metrics` 对本地排障有用 | 保留 metrics；events 推送见 8.1 F。 |
| 文档 `docs/archive/**` 里的历史 Nacos 设计 | 例如 `docs/archive/nacos-sdk-migration.md` | 历史档案可不改正文；不要让现行 README/CLAUDE 继续把 Nacos 当主路径。 |
| `PromptSignalRoutes` / `AgentkitCatalogRoutes` | 注释称供 Tianshu 轮询 | 见 8.1 B，落地时确认本地调试价值。 |

## 9. 删除文件 / 模块清单

### 9.1 必删

- `src/main/kotlin/io/presence/aichat/config/NacosConfigLoader.kt`

### 9.2 建议随 Nacos 清理一起删除（Tianshu / BytePlus 推送 / Kafka 通道 / 控制面工具）

- `src/main/kotlin/io/presence/aichat/prompt/TianshuPromptClient.kt`
- `src/main/kotlin/io/presence/aichat/prompt/TianshuRecipeClient.kt`
- `src/main/kotlin/io/presence/aichat/prompt/TianshuLiveState.kt`
- `src/test/kotlin/io/presence/aichat/TianshuPromptClientTest.kt`
- `src/test/kotlin/io/presence/aichat/TianshuRecipeClientTest.kt`
- `src/test/kotlin/io/presence/aichat/TianshuShadowDiffIntegrationTest.kt`
- `tianshu-sync/`（整目录，含 `tianshu_client.py`、`README.md`）
- `scripts/sync_prompts_to_tianshu.py`
- `src/main/kotlin/io/presence/aichat/tracking/BytePlusEventReporter.kt`
- `src/main/kotlin/io/presence/aichat/abtest/ABTestMain.kt`（若确认只是 BytePlus 手工入口）
- `src/main/kotlin/io/presence/aichat/kafka/RiskEventConsumer.kt`
- `src/main/kotlin/io/presence/aichat/ingest/MsgIngestedConsumer.kt`
- `src/main/kotlin/io/presence/aichat/maldh/MaleDhReplyConsumer.kt`
- `src/main/kotlin/io/presence/aichat/delivery/TimEventProducer.kt`
- `src/main/kotlin/io/presence/aichat/chatround/ChatRoundProducer.kt`
- 以及 `TypingService` 中创建 `KafkaProducer` 的实现（文件是否整删视 Typing 是否还有非 Kafka 用途，落地时先读再定）
- `src/main/kotlin/io/presence/aichat/obs/HttpObsReporter.kt`（若确认只服务于 Nacos `obs_config` 的 viewer 推送）

配套依赖删除见 8.1 与第 10 节。相关单测（`src/test/kotlin/io/presence/aichat/kafka/` 等）随实现删除或改为不再编译。

### 9.3 不删、只改内容的文件（必改）

- `src/main/kotlin/io/presence/aichat/config/AppConfig.kt`
- `src/main/kotlin/io/presence/aichat/Application.kt`
- `src/main/kotlin/io/presence/aichat/config/RedisProvider.kt`
- `src/main/kotlin/io/presence/aichat/config/PostgresProvider.kt`（若保留 Postgres）
- `src/main/kotlin/io/presence/aichat/config/ConfigHolder.kt`（去掉 Nacos 措辞与「由 listener 更新」的前提，保留解析函数供启动调用）
- `src/main/kotlin/io/presence/aichat/llm/LocalDeepSeekFallback.kt`（重命名 + 正式路径）
- `build.gradle.kts`、`src/main/resources/application.yaml`
- `.env.example`、`scripts/local-chat.sh`、`docker-compose.yml` 注释
- `README.md`、`CLAUDE.md`、`docs/local-chat.md`、`docs/architecture.md`，以及其他现行文档里把 Nacos 当作主配置源的段落（至少：`docs/runtime-reference.md`、`docs/integration-tests.md`、`docs/ab-experiment-guide.md` 的「如何配 Nacos」操作步骤应改为「已移除」或移入 archive）

## 10. 关键代码改动要点（给落地用，仍不是本轮改代码）

1. `resolveAppConfig` 去掉全部 Nacos 参数与分支；函数签名不再接收 nacos username/password。
2. `AppConfig` 去掉 `nacosLoader`；`tianshuConfig` / `abTestConfig` / `bytePlusEventConfig` 若对应模块按 8.1 删除，则一并去掉字段。
3. FeatureFlags 启动赋值：默认值 + `LocalEnv` 覆盖。建议同时把**公司向、且当前默认 true** 的开关改为本地默认 false：`proactiveHourlyEnabled`、`proactiveMinuteEnabled`、`videoTipsEnabled`、`videoTipsConsumerEnabled`。`hangUpGateEnabled` 在没有 msgcenter 时本就 fail-open，可默认 false 以免尝试建连。这是行为变化，须在回归说明里写明：「本地不再默认启动小时/分钟主动消息扫描」。
4. gRPC 客户端：若暂留代码，改为「环境变量显式给出 HOST 才 `ManagedChannel`」，去掉 k8s 默认主机名。
5. 单测：凡构造 `AppConfig(..., nacosLoader=...)` 或依赖 Nacos 文本的夹具要改。`EndToEndTest` / `ProactiveDebugRoutesTest` 已用 `kafkaBootstrap=null`，应继续可编译。
6. 不要改 `chat-config.jsonc` 的 prompt 内容（工作区已有无关本地改动）。最多在二期加分区，第一期不必动。

## 11. 迁移后配置来源表

| 配置项 | 迁移后的唯一来源 | 默认 / 缺失时 | 备注 |
|--------|------------------|---------------|------|
| `DEEPSEEK_API_KEY` | `LocalEnv` / `.env` | 无密钥则不注册 LLM，启动或首轮调用失败应有明确日志 | 正式路径，必填（相对本地聊天） |
| `DEEPSEEK_BASE_URL` | `LocalEnv` | `https://api.deepseek.com` | 与现码一致 |
| `DEEPSEEK_MODEL` | `LocalEnv` | `deepseek-flash` | 与现码一致 |
| `REDIS_URI` | `LocalEnv` | 不设则无 shared Redis（in-memory 回退） | `.env.example` / `local-chat.sh` 写 `redis://localhost:6379` |
| `AICHAT_REDIS_URI` | `LocalEnv` | 不设则回退到 shared Redis | 同上 |
| `AICHAT_REDIS_READ_URI` | `LocalEnv` | 不设则读走 primary | 本地通常不需要 |
| `HINDSIGHT_ENABLED` | `LocalEnv` | 非 true 则不装配 hooks | 主路径应保持 true |
| `HINDSIGHT_BASE_URL` | `LocalEnv` | `http://localhost:8888`（脚本层） | 与现码 `fromEnv` 一致 |
| `HINDSIGHT_BANK_SCOPE` | `LocalEnv` | `user` | |
| `HINDSIGHT_REFLECT_EVERY` | `LocalEnv` | `8` | |
| Prompt / persona / autocorrect / archetypes | classpath `chat-config.jsonc` | 解析失败按现有 fail 日志 | 改完重启（或二期热重读） |
| FeatureFlags | 代码默认值 + `LocalEnv` 覆盖 | 见第 10 节建议的本地默认 | 不再读 `aichat_v2_feature_flags` |
| LLM router profiles | 正式 DeepSeek 注册 | 无第二来源（第一期） | 不再读 `aichat_v2_router_profiles` |
| `GOOGLE_API_KEY` / `SERPER_API_KEY` / `WEB_SEARCH_ENABLED` | `LocalEnv`（可选） | 空则相关能力关 | 非主路径必需 |
| `PG_URL` / `PG_USERNAME` / `PG_PASSWORD` | `LocalEnv`（可选） | 不设则无 Postgres | 非主路径必需 |
| Tianshu | （删除后无） | — | 原仅 Nacos |
| BytePlus AB / 事件 | （删除实现后无；resolver no-op） | 一律默认值 / NOOP | 原仅 Nacos |
| Kafka bootstrap | （删除后无） | — | 不再认 `KAFKA_BOOTSTRAP` |
| Overlay | 无远程源 | JAR / `chat-config.jsonc` 基线 | 原 dataId `aichat-autocorrect-overlay` |
| Obs viewer 推送 | （删除后无） | NOOP | 原仅 Nacos `obs_config` |
| OTel 手动 span / `/metrics` | 代码默认或 env | 本地可继续暴露 `:8080/metrics` | 不依赖 Nacos |
| `NACOS_*` / `RAM_ROLE_NAME` / `infra.nacos` | **删除，不再识别** | — | |

## 12. 建议落地分期

在规范模式下由 Coder 执行；每期都要能编译，主路径可启动。

**第一期（必须，对应「去掉 Nacos 通道 + DeepSeek 正式路径」）**

- 删除第 9.1 节与 8.1 A、第 6 节 Redis/Postgres 的 Nacos 路径、第 5 节 DeepSeek 正式化、文档与 `.env.example` / `local-chat.sh`、`application.yaml` 凭证。
- FeatureFlags / gRPC 按第 10 节调整默认姿态（显式 HOST 才建连；公司向开关默认关）。
- 回归第 14 节。

**第二期（强烈建议紧随其后，对应「不要留下走不到的死代码」）**

- 8.1 B–F：Tianshu、BytePlus SDK 与事件上报、Kafka 通道、overlay 监听、obs viewer 推送。
- `ABTestResolver` 保留 no-op，除非同期有余力开始拆调用点。

**第三期（需用户确认）**

- 8.3 的 gRPC 整包、主动消息/chase/recovery 等业务域、多余 LLM 客户端、e2e 公司套件、文档 archive。

不要把第三期塞进「去掉 Nacos」的必通过标准，以免审查范围膨胀、误删主路径钩子。

## 13. 风险

1. **默认开关变化：** 今天本地只要 Redis 起来，小时/分钟主动扫描和 VideoTips 会按默认 true 注册。改成默认 false 后，行为与「现在的 Nacos-less 启动」不完全相同。这是有意收紧，应在发布说明里写清；若有人依赖本地主动消息，可用 env 再打开。
2. **编译扇出：** 删除 Tianshu / Kafka / BytePlus SDK 会碰到大量 import。必须分期或先留门面，避免一次改动无法通过 `./gradlew test`。
3. **Redis 读取路径变更：** 把 `System.getenv` 改成 `LocalEnv` 会使「未导出环境变量、但目录里有 `.env`」时也能连上 Redis。这是修复，但要确认测试若不该连真实 Redis，就不要在测试工作目录放 `.env`。
4. **不要默认写死 localhost Redis：** 以免单测或无 Redis 启动路径被破坏。
5. **`application.yaml` 明文凭证：** 删除时不要把旧密码复制到 memo、commit message 或新文件。若该文件历史已进 git，清理只保证工作树不再含明文；是否改写 git 历史不在本方案范围。
6. **工作区已有 `chat-config.jsonc` 未提交修改：** 落地时不要夹带进 Nacos 清理提交。
7. **包名与 sitin 依赖暂留：** 删除 Nacos 后，构建仍可能需要公司 Nexus/GAR 才能解析部分 proto SDK。这不阻碍运行时去 Nacos，但「完全离线构建」不是本方案承诺（见假设）。
8. **文档残留：** archive 与部分专题设计文档仍会提到 Nacos。现行入口文档必须改准，否则后续 agent 会按旧 CLAUDE.md 再去连 Nacos。

## 14. 回归标准

以下均在**不设置任何 `NACOS_*`、不存在 `NacosConfigLoader`、进程不尝试连接 `api.nacos.archat.us`** 的前提下进行。

1. `./scripts/local-chat.sh` 能启动 Ktor `:8080`。脚本不再导出 `NACOS_DISABLED`。本机 6379 Redis 与 Hindsight `:8888` 的预检查行为保持。
2. 启动日志应出现：Redis 经 URI 连接、Hindsight enabled、DeepSeek **正式** provider 注册（日志不得再写 `fallback` / `Nacos profiles present` / `[Nacos] skipped` / `[Nacos] create`）。
3. 启动日志不得出现向 `api.nacos.archat.us` 或 Nacos namespace `dora-*` 的拉取。建议用 `rg` 确认源码与新日志格式已无这些字符串。
4. 按 `docs/local-chat.md` 或 `./scripts/local-chat-repl.sh` 完成至少两轮对话：`POST /api/agent/run` 返回成功；同一 `conversationId` + `userId` 能延续 Redis 历史；Hindsight bank 能 retain，并可在后续轮次 recall（具体断言方式沿用现有本地验证习惯，不编造指标）。
5. `DEEPSEEK_API_KEY` 缺失时，启动或首轮有明确失败，而不是去连 Nacos。
6. `./gradlew test`（或至少 `compileKotlin` + 受影响模块测试）通过。删除模块后，对应测试应删除或改写，不得留编译失败。
7. 若执行了第二期：进程在未设 Kafka / Tianshu / BytePlus 时，不创建 KafkaConsumer、不轮询 Tianshu、不初始化 DataTester SDK。

## 15. 仍开放的假设

1. 本项目将作为个人本机助手长期使用，**不会再接入公司 Nacos / Tianshu / BytePlus / 公司 Kafka**。若该假设不成立，本方案的第二期删除不可执行。
2. 本地主路径定义为：DeepSeek + Hindsight + Redis + Agent Loop + `chat-config.jsonc` + `scripts/local-chat.sh`。PWA/TIM/主动消息不是当前主路径。
3. 保留 `io.presence` 与 `sitin.*` 包名，等于构建期仍可能依赖公司 Maven 拉 proto 工件；「纯净」指运行时配置与死代码，不自动等于完全去公司仓库。
4. `chat-config.jsonc` 继续作为 prompt 真源，第一期不把 LLM provider 配置搬进该文件。
5. `ABTestResolver` 第一期以 no-op 门面保留，不视为「还在用 BytePlus」。
6. 主动消息等业务代码第一期只关默认开关、不整包删除，除非用户在审查后把暂留项改成「确认删除」。
7. 热更新第一期只承诺重启生效。
8. 本方案不涉及把仓库改名为其他包名、不涉及推送 `presence-io/aichat-v2`、不涉及改其他仓。
9. 未对运行中的本机进程做性能测量；方案不含 SLA/QPS 数字。

## 16. 调研范围（便于复查）

在 `/Users/colna/WORK/aichat-hindsight` 检索并阅读了至少以下符号与入口：`nacos` / `nacos-client` / `mse-extension` / `NacosConfigLoader` / `NACOS_` / `api.nacos.archat.us` / `aichat_v2_router_profiles` / `LocalDeepSeekFallback` / `resolveAppConfig` / `LocalEnv` / `Tianshu` / `BytePlus` / `datatester` / `Kafka` / `overlay` / `aichat-autocorrect-overlay` / `obs_config` / `scripts/local-chat.sh` / `.env.example` / `chat-config.jsonc` / Hindsight hooks / `Application` 启动链 / `build.gradle.kts` 依赖。未读取真实 `.env` 内容。

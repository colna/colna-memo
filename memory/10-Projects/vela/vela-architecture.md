---
title: Vela 技术架构（MVP）
date: 2026-11-19
tags: project, vela, architecture, rust, tts, voice-clone, mock-live, monorepo
---

# Vela 技术架构（MVP）

本文是 **Vela**（AI 语音助播）的技术架构成稿，对齐产品真源 [vela-prd.md](vela-prd.md)。本阶段**只出方案，不开发、不建 GitHub 仓库、不改 Onia、不写飞书**。立场：clean-room，禁止复用竞品视觉、品牌、文案、密钥、接口或协议实现。不编造 QPS、延迟、RTF、GPU 实测或容量数字；需要数字处一律写「待自测」。不得把「AI 自动助播已生产稳定」写成既成事实。

文档只维护在 colna-memo，不使用飞书。

## 0. 已定决策与假设

### 已定决策

| 项 | 决策 |
|----|------|
| 仓库 | **新建独立 monorepo**，命名约定 `github.com/colna/vela`（私有；本阶段不创建）。**不并入** `onia`。 |
| 服务端 | Rust，`axum` + `tokio` + `sqlx`；业务主库**仅 PostgreSQL**；schema 只走 `sqlx migrate`；首期**单进程**部署。 |
| 后台 | `apps/admin`：Umi 4 + React 19 + antd；直连服务端 HTTP，无 BFF。 |
| 官网 | `apps/web`：Vite + React 19；介绍型，**不发任务、不开播、不做账号内业务**。 |
| Agent | monorepo 内预留 `apps/script-agent`（名称可微调）；本阶段**只边界与契约，不上线话术改写**。 |
| 客户端 | **不做** Flutter / App。 |
| 工具链 | `just` + Cargo + pnpm（风格对齐 Onia，业务逻辑不复制）。 |
| 直播侧 | **Mock 适配器**；不接真实抖音 / 其他真实直播开放平台。 |
| TTS | **可替换 adapter**；VoiceStudio（`https://github.com/colna/VoiceStudio`，上游 AGPL-3.0）仅为候选实现，不得绑死。 |
| 计费 | 首期单租户、无卡密、无真实支付；可有本地「无限额度」开发开关并标明非计费。 |

### 本阶段默认假设（实现前可改，须明示）

1. 账号体系**暂与 Onia 独立**（邮箱/密码最小登录）；是否日后统一 SSO 见文末待确认。数据模型可预留 `tenant_id`，首期不做商户后台。
2. 播放端 MVP 默认：**后台浏览器页内 Web Audio 出声**；独立播放页 / 桌面播放器为可选增强，不做 App。
3. 模式 2 某句 TTS/播放失败：默认**停在该句**，供运营重试或显式跳过；**不**默认自动跳过全部失败句（与 PRD 一致；若产品改口再改状态机）。
4. 首期默认**开播时现合成（流式）**；「试听预合成」可作为音库试听短句能力，不强制整表预合成。
5. 媒体（参考音频、合成缓存）走**对象存储**（本地联调可用 MinIO / 本地目录适配；生产选型待定）。参考音频**不出竞品、不引用竞品 CDN**。
6. 性能数字（TTFA、RTF、长稳、并发路数）一律 **待自测**；公开 benchmarks 为空时不得写入承诺值。

## 1. 系统上下文与边界

```
┌────────────┐   HTTPS    ┌──────────────────────────────┐
│ apps/web   │ ─────────► │ 静态介绍（可同域或 CDN）        │
│ 官网       │            │ 无业务写接口                     │
└────────────┘            └──────────────────────────────┘

┌────────────┐   HTTPS    ┌──────────────────────────────┐
│ apps/admin │ ─────────► │ apps/server（vela-server）     │
│ 后台       │◄── WS ──── │  账号 / 音库 / 话术 / 会话     │
│ 播放器     │  队列推送   │  播放队列编排 / Mock 直播      │
└────────────┘            │  TTS adapter 调用             │
                          └──────────────┬───────────────┘
                                         │ adapter 契约
                    ┌────────────────────┼────────────────────┐
                    ▼                    ▼                    ▼
           ┌────────────────┐   ┌────────────────┐   ┌────────────────┐
           │ TtsAdapter     │   │ LiveAdapter    │   │ ScriptAgent    │
           │ VoiceStudio*   │   │ Mock（MVP）    │   │ （预留，未上线） │
           │ 或其他引擎     │   │ 真实平台=P3    │   │ 话术改写契约    │
           └────────────────┘   └────────────────┘   └────────────────┘
```

\* VoiceStudio 进程通常独立部署（Docker，常见端口 `3900`，`OMNIVOICE_API_KEY`），由 server 侧 adapter 访问；**不要**把 STT 的 speech-platform / `/ws/transcribe` 误写成 TTS。

### 边界表

| 组件 | 做什么 | 不做什么 |
|------|--------|----------|
| 官网 | 产品介绍、能力边界声明、登录入口链接 | 建音色、开播、发任务、持有业务 token 调写接口 |
| 后台 | 登录后全部业务：音库/话术/模式控制台/队列/Mock 开停播；页内播放 | 声称「已连接抖音」；上线未命名的其余模式完整编排；卡密发售 |
| server | 元数据与会话权威、队列状态机、调用 adapter、审计 | 直接依赖某个 TTS 厂商 SDK 散落业务层；持有竞品密钥；代打真实平台私有协议 |
| TTS adapter | `synthesize_stream` / `clone_voice` / `health` 统一契约 | 模式编排、弹幕、话术改写、切段混音 |
| Mock 直播 | `start` / `stop` / 会话状态；可注入假事件（P1 弹幕） | 真实开放平台 OAuth、真实推流、真实弹幕 SDK |
| Script Agent | 预留 HTTP 契约与空壳目录 | 本阶段不做验收、不进主路径 |

## 2. Monorepo 目录建议与职责

新建仓库 `vela`（与 Onia 平行，不共享业务 crate）：

```
vela/
  apps/
    server/           # axum 二进制入口（薄壳，逻辑在 crates）
    admin/            # Umi 4 + React 19 + antd
    web/              # Vite + React 19 介绍站
    script-agent/     # 预留：话术改写 Agent（本阶段空壳 + README 契约）
  crates/
    vela-protocol/    # 唯一契约真源（可生成 packages/protocol-ts）
    vela-server/      # HTTP/WS 路由、鉴权、组装
    vela-domain/      # 音库、话术、会话、队列状态机（纯领域，无具体 TTS SDK）
    vela-tts/         # TtsAdapter trait + VoiceStudio 实现 + MockTts
    vela-live/        # LiveAdapter trait + MockLive +（P3）真实平台占位
    vela-storage/     # 对象存储抽象（本地/S3 兼容）
    vela-auth/        # 最小会话/密码哈希（独立账号假设）
  packages/
    protocol-ts/      # 由协议生成，admin 消费
  docker/
    docker-compose.yml   # postgres +（可选）minio +（可选）voicestudio
  justfile
  Cargo.toml          # workspace
  pnpm-workspace.yaml
  README.md
```

职责约束：

- `vela-domain` **不得** `use` 具体引擎 HTTP 客户端；只依赖 `TtsAdapter` / `LiveAdapter` trait（定义可放在 `vela-tts` / `vela-live`，由 server 注入）。
- `apps/admin` 只通过生成的协议类型与公开 API 通信；禁止另写一套字段名。
- `apps/script-agent` 即使空壳，也不得与 `vela-server` 揉成同一二进制职责。

## 3. 核心数据模型（逻辑）

首期单租户；表可带可空 `tenant_id` 供后续，不做多商户后台。下列为逻辑模型，非锁死 SQL 字段名。

### 3.1 用户与鉴权

| 实体 | 要点 |
|------|------|
| `users` | id、email、password_hash、created_at；**假设与 Onia 账号独立** |
| `sessions` | opaque token（只存哈希）、user_id、expires_at；token **不进 URL query** |

### 3.2 音库 / 音色

| 实体 | 要点 |
|------|------|
| `voices` | id、user_id、name、source=`clone`\|`preset`\|`design`、status=`pending`\|`ready`\|`failed`、provider_voice_id（引擎侧 id，可空）、error_message、rights_ack_at（授权勾选时间）、created_at |
| `voice_assets` | voice_id、kind=`reference`\|`preview`、object_key、content_type、duration_ms、bytes |

参考音频只存对象存储 key；克隆时由 server 经 adapter 提交给引擎，**不**把竞品样本或 URL 当输入。

### 3.3 话术

| 实体 | 要点 |
|------|------|
| `script_books` | id、user_id、title、mode_hint=`sequential`\|`assist`\|`any` |
| `script_lines` | id、book_id、ordinal（有序）、text、enabled |

模式 2 依赖 `ordinal` 严格有序；模式 3 可复用同一书或独立助播集，点选时按 line_id 入队。

### 3.4 直播位（Mock）与会话

| 实体 | 要点 |
|------|------|
| `live_slots` | id、user_id、title、adapter=`mock`、status=`idle`\|`live` |
| `live_sessions` | id、slot_id、mode=`2`\|`3`、voice_id、script_book_id、state=`starting`\|`running`\|`paused`\|`stopping`\|`stopped`\|`failed`、started_at、ended_at |

界面文案与 API 枚举均使用 Mock；禁止出现「已连接抖音」等真实平台文案。

### 3.5 播放任务 / 队列

| 实体 | 要点 |
|------|------|
| `play_items` | id、session_id、line_id（可空）、ordinal_in_session、text_snapshot、state=`queued`\|`synthesizing`\|`ready`\|`playing`\|`played`\|`failed`\|`skipped`、error_code、retry_count |
| `play_audio_parts`（可选） | play_item_id、seq、object_key 或仅内存/短时缓存指针；流式场景可先不落库全量 PCM，只记元数据与可重放策略 |

**假设**：热路径上的 PCM 块优先经 **WebSocket 推到后台播放器**；对象存储用于参考音频、失败排查切片与（可选）预合成试听，不强制每句落全量文件。

## 4. 模式运行时链路

### 4.1 共同管线

```
话术文本 → Session 编排 → TtsAdapter.synthesize_stream
        → 音频块入播放通道 → Admin 播放器消费 → 播完 ACK
        → 编排推进下一句 / 等待点选
```

MockLive：`start_session` 时 `LiveAdapter.start(slot)` → `live`；`stop` 时反向。直播适配器**不负责**出声，只负责「场次状态」与未来事件源。

### 4.2 模式 2 · 逐句读

1. 运营选音色、有序话术本、Mock 直播位 → `POST sessions`（mode=2）→ Mock `start`。
2. 编排器取 `ordinal` 最小的未完成句 → 创建 `play_item`（queued）。
3. 调用 `synthesize_stream(text, voice_ref)`；首包到达后 item → synthesizing/ready，WS 向该会话订阅者推 `audio_chunk` / `item_state`。
4. 播放器开始出声 → playing；`playback_ended` ACK 后 → played，再取下一句。
5. **控制**：pause（停取下一句且通知播放器暂停）、resume、skip_current（当前 → skipped，取下一）、stop（会话 stopped，队列归档/清空策略见实现，须可测）。
6. **失败（默认）**：TTS 或播放失败 → item=`failed`，会话停在该句（可理解为 `paused_on_failure`），等待 `retry_item` 或 `skip_current`。**不**自动吞掉后续整表。

### 4.3 模式 3 · 助播

1. 进入 Mock 直播中（mode=3），不自动扫表。
2. 运营点选一条 → 创建 `play_item` 追加到队列尾。
3. 若当前无 playing，立即合成并播；若正在播，仅入队，播完自动取队首。
4. 本阶段**不做**主轨真人 + 辅轨 AI 的闪避/混音；「插播感」仅由队列先后满足。
5. 失败：该条 `failed`，默认不阻断用户继续点选下一条（与模式 2「停表」不同）；是否自动重试标为**待确认**，实现先做手动重试。

### 4.4 失败策略占位

| 场景 | MVP 默认 | 待确认替代 |
|------|----------|------------|
| 模式 2 TTS 失败 | 停住该句，重试/跳过 | 自动跳过并记日志 |
| 模式 2 播放器断开 | 会话标 paused，保留队列 | 自动结束会话 |
| 模式 3 单条失败 | 条目标失败，可继续点选 | 自动重试 N 次 |
| 引擎不可用 | 开播前 health 检查失败则拒绝 start | 降级非流式整段（UI 必须标明非流式） |

## 5. 流式 TTS 集成契约

### 5.1 Adapter 接口（逻辑）

```rust
// 示意，非最终 API
#[async_trait]
trait TtsAdapter: Send + Sync {
    async fn health(&self) -> Result<TtsHealth>;
    async fn clone_voice(&self, req: CloneVoiceRequest) -> Result<CloneVoiceResult>;
    async fn synthesize_stream(
        &self,
        req: SynthesizeRequest,
    ) -> Result<Box<Stream<Item = Result<AudioEvent>>>>;
}

enum AudioEvent {
    Start { sample_rate: u32, channels: u16, format: AudioFormat }, // 如 Pcm16
    Chunk { bytes: Bytes },
    Done { duration_ms: u64, metrics: Option<TtsMetrics> }, // metrics 仅观测，不写进产品承诺
    Error { code: String, message: String },
}
```

业务层只依赖上述事件；**禁止**在 domain 里写死 `/ws/tts` 路径。

### 5.2 VoiceStudio 候选映射（实现细节，可替换）

| Vela 契约 | VoiceStudio（公开信息） |
|-----------|-------------------------|
| 流式合成 | WebSocket `ws://<host>:3900/ws/tts`；客户端发 JSON（text、voice profile_id、speed 等）→ 收 PCM16 块 → JSON `done` / `error`；可有 `start` 元数据（sample_rate 等） |
| 鉴权 | 非 loopback 需 `OMNIVOICE_API_KEY`；浏览器场景优先 **WS ticket / Header**，**避免**把长期 key 放进 query（竞品反例：token 入 query） |
| 克隆 | 经其 HTTP API 创建 voice profile（具体路径以所选版本文档为准，经 adapter 封装） |
| 部署 | Docker 常见 `-p 127.0.0.1:3900:3900`，`-e OMNIVOICE_API_KEY=...` |
| 非 TTS | `/ws/transcribe` 等为 STT，**禁止**接到合成链路 |

**假设**：server 与 VoiceStudio 同私网或本机；由 server 持有 key，**不**把引擎管理员 key 下发给浏览器。浏览器只连 Vela server 的会话 WS。

### 5.3 预合成 vs 现合成

| 策略 | MVP 建议 |
|------|----------|
| 开播主路径 | **现合成 + 流式**（边生成边播） |
| 音库试听 | 短句合成，可短时缓存 |
| 整表预合成 | 非必须；作 P1 优化项（降低开播瞬时负载） |

若引擎降级为非流式整段返回：UI **必须**标明「非流式」，不得假装边生成边播。

## 6. 声音克隆上传与存储

1. 后台上传参考音频 → server 校验类型/大小/时长上限（具体上限实现时定，失败原因明确返回）。
2. 写入对象存储，创建 `voices`=`pending` + `voice_assets`。
3. **权利人授权**：创建前强制勾选「拥有权利或已获授权」；记录 `rights_ack_at` 与用户 id；提供违规处理入口（文案级，法务另议）。
4. 异步任务调用 `TtsAdapter.clone_voice` → 成功写 `provider_voice_id`、status=`ready`；失败写 error。
5. 试听：短文本 `synthesize_stream` 或非流式，结果可短缓存。
6. **禁止**：用竞品站点音频 URL 直接当参考；禁止把未授权第三方音色样本打进引擎。

## 7. Mock 直播适配器接口

```rust
#[async_trait]
trait LiveAdapter: Send + Sync {
    fn id(&self) -> &'static str; // "mock"
    async fn start(&self, slot: &LiveSlot, session: &LiveSession) -> Result<LiveHandle>;
    async fn stop(&self, handle: &LiveHandle) -> Result<()>;
    async fn health(&self) -> Result<()>;
    // P1: fn subscribe_events(&self, handle) -> Stream<LiveEvent>  // 假弹幕等
}

enum LiveEvent {
    Comment { id: String, user: String, text: String, at: DateTime<Utc> }, // Mock
    // P3: 真实平台事件另行映射，不复用竞品协议字段名
}
```

MVP：`MockLiveAdapter` 只切换 slot/session 状态并打审计日志。日后真实平台实现同一 trait，**替换注入**即可；真实接入前单独合规评估（P3）。

## 8. 后台、官网与鉴权最小集

### 官网 `apps/web`

- 静态页：定位、MVP 能力、**明确写本阶段 Mock / 未接真实抖音**、进入后台链接。
- 无登录态业务 API。

### 后台 `apps/admin`

建议导航（未开放项勿做成假可用）：

- 登录
- 音库
- 话术
- 直播控制台（模式 2 / 3、队列、Mock 开停播、播放器）
- 设置（密码；开发额度开关）

### 鉴权

- HTTPS（生产）；本地可 http，文档标明。
- `Authorization: Bearer <session_token>`；**token 不进 query**。
- 会话 WS：先 HTTP 换短时 `ws_ticket` 或同 origin cookie + Origin 校验；禁止长期密码/API key 进 WS URL。
- 密码哈希（如 argon2）；无第三方登录矩阵。
- CORS 显式允许 admin origin。

## 9. 分期技术债

| 分期 | 内容 | 技术要点 |
|------|------|----------|
| **MVP** | 模式 2/3、克隆、TTS 流式、音库/话术/队列、Mock 直播、官网+后台+server、adapter 边界 | 本文范围 |
| **P1** | 碎片切段；主辅插播 | 自建 **ffmpeg + 更完整音频队列**；非 VoiceStudio 能力 |
| **P1** | Mock 弹幕驱动触发 | `LiveAdapter` 事件流 → 规则 → 入队助播句 |
| **P2** | 其余模式（另 5 种**不命名**，待完整逆向报告） | 扩展会话状态机 |
| **P2** | 话术改写 Agent | `apps/script-agent` 实现契约；默认人工确认 |
| **P2** | 二创（重组/变声） | 独立音频处理管线 |
| **P3** | 真实直播平台 + App | 合规评估；新 LiveAdapter；另议移动端 |
| 待定 | 多租户 / 卡密 / 货盘 / 算力账单 | 首期不做 |

## 10. 安全与合规

1. **传输**：生产全站 HTTPS；关闭明文 token 下发到 query。
2. **协议**：自有 JSON schema / 枚举；**禁止** `eval` 远端脚本或竞品式动态执行。
3. **密钥**：不硬编码任何平台签名 Key；引擎 key、对象存储密钥仅环境变量 / 密钥管理。
4. **AGPL**：VoiceStudio 应用 AGPL-3.0；商用 SaaS 须 Pro 授权或合规评估——文档阶段标 **未清关**。
5. **模型许可**：官方清单未见 `commercial_use: true` 时，默认 OmniVoice 等为非商用；商用前换引擎或取得授权。
6. **音色授权**：上传勾选 + 审计字段 + 处理入口。
7. **clean-room**：不依赖竞品 API 路径、WS 字段或密钥；完整逆向报告未读到时不猜协议。
8. **对外口径**：不得宣称已商用清关、已接抖音、或助播已生产稳定。

## 11. 本地联调、发布顺序与回退

### 本地联调（建议顺序）

1. PostgreSQL +（可选）MinIO。
2. （可选）VoiceStudio Docker stable 于 `127.0.0.1:3900` + `OMNIVOICE_API_KEY`；先用 `MockTtsAdapter` 也可跑通队列 UI。
3. `vela-server` 配置 adapter=`mock_tts` 或 `voicestudio`。
4. admin 登录 → 音库（Mock 克隆或真克隆）→ 话术 → 模式 2 Mock 开播 → 听流式播放。
5. 模式 3 点选助播。
6. 自测清单（数字不入库承诺）：10s 中文口播 TTFA/RTF、数小时稳定性、克隆主观相似度——**待自测**。

### 发布顺序（有代码之后；本阶段不执行）

1. 迁移 forward-only。
2. server → admin → web。
3. TTS 引擎独立进程，先于或并行于 server，失败时 server health 显式降级。

### 回退

- 应用：保留上一版本产物，反向切换二进制/静态资源。
- 迁移：只向前兼容；破坏性变更用扩展列 + 双读，避免硬 down migrate。
- 引擎：adapter 配置回切 `mock_tts` 或上一引擎；会话中失败走第 4 节策略，不静默空播。

## 12. 与 Onia 的关系（避免混仓）

| 项 | 关系 |
|----|------|
| 仓库 | 独立 `vela`，不并入 `onia` |
| 风格 | monorepo 分层、Rust/axum/sqlx、admin/web 技术选型对齐 |
| 业务 | **不复制** Onia Signal/Todo/Behavior、设备 WebView、Instagram 管线 |
| 账号 | 默认独立（待确认是否日后 SSO） |
| 文档 | `memory/10-Projects/vela/`，与 onia 目录分离 |

## 13. 待确认问题（对齐 PRD）

1. 播放端是否接受「仅后台浏览器出声」作为 MVP，还是必须另做独立播放页/桌面播放器？
2. 模式 2 失败句：维持「停住等重试/跳过」，还是改为「失败自动跳过并记日志」？
3. 首期是否需要「试听预合成」整表能力，还是开播现合成 + 音库短句试听即可？
4. 登录是否必须对接 Onia 同一账号体系，还是 Vela 独立账号？（本文暂按独立）
5. 完整逆向报告恢复后，是否立即补全另外 5 种模式的分期命名与优先级？
6. 商用路径优先级：先 VoiceStudio Pro + 可商用模型，还是一开始就选其他可商用 TTS？

## 14. 非目标再声明

本文是技术方案，**不是**开工指令。本阶段用户明确先不开发。交付理解应限于：评审用架构、adapter 边界、Mock 闭环设计与分期技术债。不是可上线 SaaS，不是已接抖音的无人直播工具，也不是对播搭子的协议级克隆。

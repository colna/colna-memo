---
title: sitin4.0 异常任务 / 探针(探真)/ 真人接管 后端契约
date: 2026-07-08
tags: [sitin4.0, app-pwa, 探真, probe, anomaly, 真人接管, 消息审核, 后端契约]
---

# sitin4.0 异常任务 / 探针(探真)/ 真人接管 后端全链路契约

2026-07-08 从飞书 wiki《sitin.ai-4.0》(node `LXCxwMdHbiTcC8keI2jc2bMDnmg`,顶层需求《SITIN 4.0 - 纯真不穿》`MCpwwzZvviKByjk7kBhczdPVnzf`)13 篇技术文档梳理。PWA 前端接入方案见 [[pwa-verify-tasks]]。

## 一、整体链路

```
checker 打分(横向/纵向,暴露面/觉察面)→ 红/黄/绿(产出到 db)
  ↓ 高风险
aichat 停止代回(女用户在 chat 页工作台时不主动回复)→ sender 组合红黄绿+内容下发
  ↓ 后端建「异常任务」AnomalyTaskItem,可附 hasProbe
IM 全局推送(付费版,管理员账号,不轮询)→ 前端刷 /anomaly/list
  ↓ 前端按优先级处理;探针独立弹窗
用户操作(选建议/文字/语音/图片/探真自拍)→ 内容审核 → 发出
  ↓ 后端收 TIM AfterSendMsg 回调
清除 taskId(+hasProbe)→ 推送更新 → 前端加钱动画 → 下一个
```

三段式(ai穿帮风险打分器,owner 刘锴):
- **checker**:只 check(横/纵向),可离线验证、实时产出,不依赖其他服务,结果落 db。参考 `github.com/presence-io/aichat-flywheel`。具体维度/阈值在《风险打分器架构设计》`LO6bwmYWwiD5Eyke4TYcaInMnzd`(未抽)。
- **aichat**(v1/v2/v3):只生成内容;女用户在工作台时不主动回复,交 sender。
- **sender**:红=下发自定义消息(不发 aichat 内容);黄/绿=aichat 消息+flash 备选,下发对应状态。

## 二、AnomalyTaskItem(前端契约,proto messaging_api 已生成)

```ts
interface AnomalyTaskItem {
  conversationId: string; targetUserId: number;
  status: "red"|"yellow"|"green";   // 持久,只升不降(绿→黄→红,不回退)
  taskId?: string;                  // 待处理任务唯一标识;所有本项目发出消息的 cloudCustomData 必带
  suggestions?: SuggestionSegment[][]; promptText?: string;
  timeoutSeconds?: number;          // 红/黄默认 180
  rewardCents?: { text?; voice?; image?; video? };  // 各类型奖励(¢),仅前端展示,实际加钱后端算
  reasons?: string[];               // AI 检测原因 "Story changed: Tampa → Phoenix"
  briefingSummary?: string;         // 概要+风险原因 ≤100字
  // ── 探针(探真)── 附加属性,依附 taskId,不独立
  hasProbe?: boolean;
  probeType?: "voice"|"image";      // proto: ANOMALY_PROBE_TYPE_VOICE=1 / IMAGE=2
  probePrompt?: string;
  probeRewardCents?: number;
  probeTimeoutSeconds?: number;     // 默认 180
  // ── 结束对话/拉黑 ──
  endChatEnabled?: boolean; endChatReason?: "pornographic"|"abusive"|"inactive";
  endChatDeductCents?: number; totalEarnedCents?: number;
  videoTipsEnabled?: boolean;       // 视频 Tips 卡
  inputMode?: "voice"|"text";       // 按 pair 记忆,后端持久化,默认 voice
}
// SuggestionSegment { intervalMs; text }  // 0=立即发
```

## 三、探针(探真)规则

- 前端**只看 `hasProbe` 决定弹窗、不区分红黄绿**;按 `probeType` 弹拍照/录音窗;弹窗**不改会话状态**,180s。
- 绑定具体会话,**需跳到对应聊天页后才开始处理和计时**;触发条件后端判定,前端不关心。
- 处理流程:列表发现 hasProbe → 按优先级跳会话 → 弹窗展示 `probePrompt` + 启 `probeTimeoutSeconds` 倒计时 → **完成**(发语音/图片 → 后端 IM 回调清整条任务 → 推送 → 加钱动画 → 关窗 → 下一个)/ **超时**(自动关窗 → 继续处理当前消息 → 超时计时重来)/ **杀进程重进**(拉列表仍在 → 重弹,可接受)。
- 探针回复消息**保留聊天记录、双方可见**;**无独立清除接口**,靠后端 IM `AfterSendMsg` 回调统一清。
- **红色状态用户主动发图免审,但探针图片仍需审核**;其余消息发送后走审核。

## 四、消息审核服务端(女用户消息审核,owner 钱文锦)

新建 `chat-service-api` gRPC(Spring Boot 4.0 + Java 21,gRPC 8080),proto `chat/api/chat_api.proto` → Java artifact `ai.sitin.chat:chat-grpc`。三模块:`-api` + `-infra` + `-worker`(Kafka 消费写库)。

| RPC | 依赖 | 说明 |
|-----|------|------|
| `GetUploadCredential` | Aliyun OSS | presigned V4,有效期 300s(最长7天),客户端直传 PUT。`FileType` UNSPECIFIED0/VOICE1/IMAGE2/VIDEO3;图片 jpg/png/webp(bmp→INVALID_ARGUMENT)。出参 `presigned_url/cdn_url/object_key/expires_in` |
| `AuditText` | Gemini `gemini-3.1-flash-lite-preview`,LLMScheduleClient | 服务端用 user_id+target_id 构 session_key,从 aichat-v2 PG `tim_msg_record` 取**最近10条**上下文。入参 `user_id/text/target_id` |
| `AuditVoice` | Gemini(音频直审,不转文字) | 服务端从 OSS 下载音频 → Gemini 理解。入参 `user_id/voice_url/target_id` |
| `AuditImage`(探针自拍) | **数美 Shumei** HTTP(非 LLM) | 人脸检测+性别验证,`ImageAuditService.java`,参考 dora `UserScreenshotDetectService.shumeiTargetLabelResult()`,accessKey/appId/eventId 从 Nacos。入参 `user_id/image_url/target_id` |

- 出参 `AuditResponse{ passed, violation_category, violation_message }`。**审核不通过≠错误**(passed=false 正常返回)。
- **ViolationCategory**:`NONE=0`、`FINANCIAL_INCENTIVE=1`、`AI_DISCLOSURE=2`、`PLATFORM_DEFAMATION=3`、`GIBBERISH_AND_SPAM=4`、`NO_HUMAN_FACE=5`、`NOT_FEMALE=6`(1-4 文字/语音内容,5/6 探针专属)。与 PWA `chatModerationApi.ts` 完全一致。
- mock 先行 proto id:**23000** 获取上传信息 / **23001** 文字审核 / **23002** 语音审核 / **23003** 自拍审核。
- 系统级错误走 gRPC Status:`INVALID_ARGUMENT/PERMISSION_DENIED(封禁)/NOT_FOUND(voice_url过期)/RESOURCE_EXHAUSTED(限频)/INTERNAL(Gemini/数美超时)/UNAVAILABLE`。fail-open/closed **待定**。
- **审核记录表** `chat_audit_record`(sitin-server PG,按天 RANGE 分区):`user_id/target_id/session_key/audit_type(TEXT/VOICE/IMAGE)/content/success/passed/violation_category/violation_message/error_message/latency_ms/created_at`。写路径:审核完 → Kafka → `chat-service-worker` 异步写库。
- 双数据源:主库 sitin-server PG(`chat_audit_record` 读写,prod `10.51.1.4:5432/archat`);第二数据源 aichat-v2 PG **只读** `tim_msg_record`(`from_user_id/to_user_id/dh_user_id/msg_desc/msg_time`)。
- **待确认**:`tim_msg_record` 未来改从「消息中心」取(替换第二数据源方案)。审核细节另见《消息内容审核》`IVQlwDHG4icQCikt0MqcSYiJnbe`(未抽)。

## 五、真人接管 红黄绿状态机(owner 耿学岩)

| 状态 | 风险 | Header | 用户操作 | 锁定 | 超时 |
|------|------|--------|----------|------|------|
| 🔴红 | 高,强制干预 | COPILOT | 手动 文字/语音/**图片** 回复 或 End chat | 仅输入区,不能切会话/返列表 | 180s |
| 🟡黄 | 中,引导 | COPILOT | 点建议/文字/语音(**不能发图**) | 建议栏+输入框,不能切会话/返列表 | 180s |
| 🟢绿 | 低,静默 | AUTO | 前端自动发第1条,无需操作 | 整页不可点 | 无 |

- 状态只升不降;免费用户绿灯不进列表、后端直接代回;女用户不在页时绿色后端代回不建任务。
- **优先级** `Red>Yellow>Green`,同级 **hasProbe 优先**,第二级时间倒序。chat 列表排序:未读优先 > 时间倒序 > 关系对程度。
- **cloudCustomData**(TIM 消息携带):`{ taskId(必), isProbeReply(必), isAiReply?, isLastSegment?, earnedCents? }`。`isAiReply` AI 代回写(渲染 "AI replied");用户手动点建议**不写**;`isLastSegment` 分段最后一条写(触发后端清任务);`earnedCents>0` 才显示 `+¢X.XX`。
- **超时**:详情页曝光开始计时;录音/视频/探针弹窗期间/审核中→暂停;取消录音→恢复;发送成功/视频通话当前对→销毁;探针弹窗关(未完成)→计时重来。超时 `POST /anomaly/timeout` 上报但**任务不移除,仍须完成**。
- **输入模式**按 pair 记忆(`inputMode`,默认 voice),切文字下次仍文字。图片仅红色可发。

### 后端接口
| 接口 | 用途 |
|------|------|
| `POST /anomaly/list` | 拉任务列表(传 conversationIds 返指定,不传返最近 limit=20,已按优先级排序)。Req `{userId, conversationIds?, limit?}` → `{tasks: AnomalyTaskItem[]}` |
| `POST /anomaly/timeout` | 超时上报 `{userId, targetUserId, level}` |
| `POST /anomaly/end-chat` | 结束对话/拉黑,扣收益 → `{deductedCents, remainingCount}`;每自然周上限 6 次 |
| `POST /anomaly/update-input-mode` | 更新 pair 输入模式,后端持久化 |

- 推送:IM 全局推送(管理员账号 `administrator_unread_disabled`),**不轮询**,丢失靠下次刷新/进页恢复。详见《IM 全局推送接入》`O9BQwt8PVigLlsk9fbLcWCnqnbh`。
- 清任务:后端收 TIM `AfterSendMsg` 回调移除 taskId+hasProbe。
- 状态恢复:刷新/启动 → `/anomaly/list {userId}` → 按 status 排序,**无时间窗限制**。
- 方案选型:从 TIM `CustomMark`(256B/需旗舰版/内容二次请求)改为**后端自维护任务列表**(无容量限制、不依赖 TIM 版本、直返完整数据)。

## 六、消息中心(TIM 消息唯一收口)

- **红蓝检查/探真审核/任务下发都不归它**,但探真真人消息进出要过它。
- 女方真人入站 `direction=0(IN)`;`dh_user_id`=DH/女方人格账号。
- 出站下发 `POST /messages/downlink`(调用方=任务中心),入参 `{conversationId, channel, from, to, dhUserId, contentType, content, replyToMessageId?, idempotencyKey(必)}` → 持久化+push TIM。**以女方身份代发须加 `ForbidAfterSendMsgCallback` 防回环**。探真通过后的真人自拍/语音/视频 note 即经此下发。
- `content_type`:0text 1image 2voice 3video 4custom 5gift 6system(探真自拍=image、语音note=voice)。
- 控制信令过滤:`Data.bizType = anomaly_task / global_broadcast`、管理员消息一律丢弃(不污染 history)。
- AI→真人→审核 的时序协调(latest-wins 取消在途、发出前 staleness 校验)由**真人接管/消息审核侧**定义,不在消息中心。

## 七、飞书文档地图 + owner

顶层需求《SITIN 4.0 - 纯真不穿》`MCpwwzZvviKByjk7kBhczdPVnzf`。子文档(space `7563852588778143772`):

| 文档 | node_token | owner |
|------|-----------|-------|
| ai穿帮风险打分器 | NkWxwtsJfiBkKekoUp1cw7rYnnc | 刘锴 |
| 真人接管&女用户接管 | RQThwZfijiZgoUkfTNFcJzinnch | 耿学岩 |
| 女用户消息审核服务端 | PhzLwCDeYi9gvrkkPm6cDaGan0c | 钱文锦 |
| 女用户消息内容拦截(探真审核真源) | Pi6pw3EG2im69fkm8Rcct16fn4b | 钱文锦 |
| 消息收益 | Hz2TwYYoviccCYk4uyDcftZpnbz | 沈硕 |
| 消息中心技术方案 | DyUdwkxbBiSjhkkiQEMcZ199nah | — |
| pwa调整 | MsJXwxR6kiOYrlk2C9McozCDnNe | 步川 |
| 女用户教学&激励 | A3T9ws4aZiaXP4k8n6XcN3vGnsh / 前端 Bzi6wEhXQifEa3krfKxco3OynKg | 步川 |
| 任务拆解 / mock先行(sheet) | ORZdw9XfriEjjMktVA8c3x1onJc / GNJuwOVwIiI06wkJB3uctgBlnWf | — |

**未抽(需要时补)**:《风险打分器架构设计》`LO6bwmYWwiD5Eyke4TYcaInMnzd`(红黄绿维度/阈值)、《消息内容审核》`IVQlwDHG4icQCikt0MqcSYiJnbe`、《IM 全局推送接入》`O9BQwt8PVigLlsk9fbLcWCnqnbh`。

抽取命令:`lark-cli docs +fetch --doc "https://presence.feishu.cn/wiki/<node_token>" --scope full --as user --json`(sheet 用 `wiki +node-get` 拿 obj_token 再走 sheets 命令)。

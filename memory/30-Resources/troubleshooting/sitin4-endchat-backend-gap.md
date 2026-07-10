---
title: sitin4 拉黑（End chat）：proto/文档是准，后端还是桩
date: 2026-07-09
tags: [troubleshooting, sitin4, sitin-server, app-pwa, 拉黑, 契约]
---

# 拉黑链路：契约齐了，后端没实现

**约定（用户 2026-07-09 明确）：以 proto 和文档为准。** 后端 `release/test` 上的桩实现不作为依据，
前端按契约写，后端补齐即通。

## 契约（三处一致）

**产品文档**（`wiki/MCpwwzZvviKByjk7kBhczdPVnzf`，「结束会话功能」）：

- **按钮出现时机**（三选一，后端判定）：
  1. 男用户命中过**色情**内容
  2. 男用户命中过**辱骂**内容意图识别
  3. 倒计时为 0 后在线累计**超过 30min 未处理**（可配置）
- **block 上限：每自然周 = 6；达到上限后按钮消失**
- **二次确认**，且**文案分两套**：
  - 色情/辱骂 → 「扣减 **x/2** 美金（从对方身上获得过 x 美金）」
  - 30min 未处理 → 「扣减 **x** 美金（从对方身上获得过 x 美金）」
- 点确认后弹出**扣减金额提示**、切换会话（若无未读，跳转至首个关系对）

**技术文档**（`sitin4.0-chat`，刘锴）：

> 拉黑（= 红色 End chat，`POST /anomaly/end-chat`）双副作用：① 该会话 `active` 任务全置 `cancelled`；
> ② 拉黑扣钱**经价格引擎扣减**（`endChatReason` 定比例）→ 消息金，返回 `deductedCents` + `remainingCount`
> （≤6/自然周，超限不再下发 `endChatEnabled`）

**proto**（`archat_api/messaging_api.proto`）：

```protobuf
// AnomalyTaskItem
bool  endChatEnabled     = 16;
AnomalyEndChatReason endChatReason = 17;  // Determines deduction ratio and confirm copy
int32 endChatDeductCents = 18;
int32 totalEarnedCents   = 19;

message AnomalyEndChatResponse { code; int32 deductedCents = 2; int32 remainingCount = 3; }

enum AnomalyEndChatReason { UNKNOWN=0; PORNOGRAPHIC=1; ABUSIVE=2; INACTIVE=3; }
```

三个 reason 正好对应文档的三个出现时机。

## 后端现状（`sitin-server` @ `release/test`，2026-07-09）

`chat-service-api/.../grpc/AnomalyServiceGrpcImpl.java:166` 的 `endChat` 是**桩**：

```java
responseObserver.onNext(EndChatResponse.newBuilder()
        .setDeductedCents(0)      // 写死
        .setRemainingCount(0)     // 写死
        .build());
```

它只：删任务行 → 复位 `autoReplyPaused/earningsPaused/distributionStopped` → 往 Kafka 发 `CHAT_ENDED`。

**缺口清单**（全部实测 grep 确认）：

| 缺什么 | 证据 |
|---|---|
| 扣款逻辑 | 全仓库 `deduct` 只命中那行 `setDeductedCents(0)` |
| payment 消费 `CHAT_ENDED` | `payment-service-worker` 只有 7 个 java 文件（CashOut/Payout 定时任务），**零 `@KafkaListener`** |
| `endChatEnabled` 下发 | 后端仓库**零命中** |
| `endChatDeductCents` 下发 | 后端仓库**零命中** |
| `endChatReason` 下发 | 后端仓库**零命中** |

按 `docs/sitin4/payment-service-worker.md`，`CHAT_ENDED → 执行扣款逻辑` 本该在 payment 侧；
设计原则也写明「任务中心不算收益，`rewardCents` 仅前端展示，实际由 payment 独立计算」。

## 前端按契约的落地（PR #575）

- 门：`onBlock={convData?.endChatEnabled ? handleBlock : undefined}` ——
  **后端不下发就不显示按钮**。这也是「绿/黄态是否显示」的正解：**不看红黄绿，看 `endChatEnabled`**。
- 接口：`anomalyEndChat(userId, targetUserId, conversationId)`，**不是** `blockHumanChatUser`
  （后者是 user_service 的通用屏蔽，不取消任务、不走价格引擎，响应只有 `code`+`message` —— 用它拉黑等于没人被扣钱）
- 确认弹窗显示 `convData.endChatDeductCents`（预估）；成功弹窗显示 `resp.deductedCents`（**实扣**）。**两者可以不同。**
- `remainingCount` 未展示（原型文案没有这句，不发明设计）。
- **未做**：按 `endChatReason` 分两套确认文案（x/2 vs x）。需先确认 `endChatDeductCents` 下发的是否已乘比例
  —— 若是，前端只需换措辞，不该重算金额。

## 待后端确认

1. `endChatDeductCents` 下发的是**已按 reason 乘过比例的最终值**，还是原始 `totalEarnedCents`？
2. `endChatEnabled` / `endChatReason` / `endChatDeductCents` 何时下发？
3. `CHAT_ENDED` 的扣款消费者何时补齐？
4. 6 次/周上限由后端「不再下发 `endChatEnabled`」实现，前端不数 —— 确认无误。

## 教训

- **「这是产品决定，我不做」≠「我不用查」**。我一度把 `blockUser` 留着，理由是「选哪个接口是产品决定」——
  决定早写在技术文档里。查文档比写注释把问题抛回去快得多。
- **飞书全局搜是最快路径**：`lark-cli drive +search --query "拉黑"` 一次命中三份权威文档。
- **后端没实现 ≠ 契约错**。以 proto + 文档为准写前端，后端补齐即通；反过来照着桩实现写，等后端一改就全错。

## 相关

- 后端全貌：[[../sitin-server-backend]]
- 前端 PR：#574（弹窗样式）、#575（链路 + 门）

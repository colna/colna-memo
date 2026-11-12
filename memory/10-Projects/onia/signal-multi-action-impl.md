---
title: Onia Signal 多动作串行与编辑页实现方案
date: 2026-11-12
tags: project, onia, social-proxy, architecture, signal, admin, pipeline
---

# Onia Signal 多动作串行与编辑页实现方案

本文是给后续实现用的技术方案。产品口径以 [signal-multi-action.md](signal-multi-action.md) 为准；动作名、params / 结果契约、脚本内 pipeline、单动作「发令」入口以 [instagram-actions-pipeline.md](instagram-actions-pipeline.md) 为准。对照本机仓库 `/Users/colna/WORK/onia`（含未提交改动）写成。界面以已确认的设计成稿 [Onia Signal 多动作串行设计成稿](https://cursor.com/canvas/store-bda2343d-69bf-4009-b7d8-4fc4d38d6d7c/6e27edf6-e421-4033-b130-b1f3f8b43810) 为准。**本文已确认可落地，供 Coder 按此实现；本文件仍不改 Onia 仓库、不写业务代码、不提交、不合入。**

## 结论

本阶段改变「一条 `MANUAL_ACTION` 只生成一种动作、一条 Todo」的约定，但**不**改变分层，也**不**把多动作塞进一次 `behavior.execute`：

1. 编辑页保存的是**不绑定设备的 Signal 定义**；「创建 Signal」改名为「发送 Signal」，负责选定义 + 设备下发，并仍可发送 `INBOX_ACTIVITY` / `PROFILE_SYNC_NEEDED`。「发令」页保留为单动作快捷入口，不改成流程图。
2. 一条含 N 步的已发送 Signal，由 `manual` 策略一次展开为 **N 条有序 Todo**，再各晋升为一次单动作 Behavior。串行由 Scheduler 门禁保证：第 k 步成功终态之前，不得晋升第 k+1 步。不以客户端 `executePipeline` 为前置，不改 `behavior.execute` 的四字段信封。
3. Signal 类型继续用 `MANUAL_ACTION`。旧 payload `{ "action", "params" }` 必须继续有效，并规范化为长度为 1 的序列。多动作用有序 `steps` 扩展同一 payload。
4. 同一条 Signal 内部即使动作类型相同，也不得用 `already_open` 把后一步吞掉。与其他 Signal 的同类未完成 Todo 冲突时，本步**等待**，不合并、不互相取消。失败则后续步骤**取消**（原因：前一步失败），不是再标失败；允许对失败步重试，不允许跳过。
5. `create_post` / `create_story` 可编排、可发送，但实打上传尚未打通。文档、界面、验收都不得写成已经能发帖；live 失败按该步最终失败处理，不得回落演示数据、不得误报成功。

## 假设

1. 活代码在 `/Users/colna/WORK/onia` 的未提交工作区之上继续，不要回退已有登录 / 注入 / 发令 / live 路径，不要用 box 上的旧脚手架。
2. 单动作契约（snake_case 动作名、表单字段、错误分类、单一 `social_proxy.js`、Flutter 按 `behavior_id` 做动作内部 pending / resume）仍按 [instagram-actions-pipeline.md](instagram-actions-pipeline.md) 执行，本阶段不重开一份动作表。
3. 后台已有「发令」页（`/commands`）和「创建 Signal」页（`/signals/create`）。本阶段改的是发送语义和新增编辑页，不是从零做发令。
4. 界面结构、文案和状态以已确认的设计成稿为准；本文锁数据模型、校验、管线语义和接口契约，不锁死画布组件库。布局稿是结构示意，实现跟已落地的 Ant Design 管理台。
5. 不编造 QPS、延迟、容量或可用性数字。下文若写「定义最多 20 步」，只是协议校验上限，避免误存超长链，不是容量承诺。
6. 本阶段不做并行 / 条件 / 循环 / 延时节点、步骤间变量引用、Agent、`send_reply`、脚本 OTA，也不打通发帖上传。

## 与既有文档的关系

- [signal-multi-action.md](signal-multi-action.md) 是本阶段产品真源。其中待确认项已按下方「实现拍板」锁定，并与设计成稿「假设说明」一致。
- 已确认设计成稿：[Onia Signal 多动作串行设计成稿](https://cursor.com/canvas/store-bda2343d-69bf-4009-b7d8-4fc4d38d6d7c/6e27edf6-e421-4033-b130-b1f3f8b43810)。实现不得偏离该稿的页面分工、单链拦截、追踪状态文案，以及 `create_post` / `create_story` 的「上传未打通」提示。
- [instagram-actions-pipeline.md](instagram-actions-pipeline.md) 对**单动作发令、脚本内部 pipeline、动作契约**仍然有效。其中「服务端不下发步骤数组 / 一次 Behavior 只跑一个动作」继续成立。被本阶段**覆盖**的只有：一条 `MANUAL_ACTION` 只产出一条 Todo；`manual` 包按 Todo 类型做 `already_open` 直接不建 Todo；「创建 Signal」页只承担来源事件、与发令页分工的旧表述。
- 架构里的分层、不直派 Behavior、离线不是失败、重试留在 Todo 状态机、获客包「有序列 Todo」的展开方式，均继续有效。本阶段只做手动 Signal 的有序展开，不改造获客包。

## 实现拍板（已确认）

下列条目原为产品文档第 11 节未决项，设计成稿按同一默认落到界面，现一并确认，Coder 按此实现，不要再改语义。

1. **权限**：不做更细角色。已登录后台即可编辑、发送、取消、重试。定义无「启用 / 停用」开关；删除用归档。
2. **发送前改参数**：发送页默认展示与成稿一致的「本次快照（只读预览）」。确认区提供「改本次参数」（展开或打开与编辑器相同的参数表单）和可选「写回定义」；未点「改本次参数」时保持只读。默认不写回定义。服务端仍以提交的快照为准做校验。不要把发送页改成整页动态动作表单。
3. **与其他 Signal 同类 Todo 冲突**：等待，不是失败，追踪必须写出原因（`wait_reason`）。本阶段不设等待超时；运营若认为等太久，可取消本条 Signal。
4. **从已发送实例反生成定义**：本阶段不做。只做定义另存。
5. **追踪**：在现有追踪页上按 `step_index` 展示动作名、参数摘要、状态、`wait_reason` / 取消原因、以及已有的 Behavior 结果；不重写整页。列表项补充派生的 Signal 总状态和步数。状态必须有文字，不只靠颜色。
6. **删除定义**：列表上的「删除」实现为归档（`archived_at`）。已被实例引用的定义可归档；已发出实例及其 payload 快照保留，追踪不受影响。

## 设计确认（界面约束）

导航顺序固定为：设备、Signal 编辑、发送 Signal、发令、追踪。旧「创建 Signal」路由跳到发送页。

**Signal 编辑**：进页先看定义列表（名称、步数、更新时间；操作：打开、另存、去发送、删除），再进画布。未保存不能发送；有未保存更改时，「保存并去发送」禁用。拖入节点后自动接入单链，落在两步之间则插入；连线由系统维护，不提供手绘分叉、汇合或环。可用「自动排成一条竖向链」收拾，排完仍要运营确认后保存。空画布、非单链或缺必填时，拦住保存为可发送定义和进入发送，并写明原因，不得静默改序。

成稿已出现的中文名必须沿用，并与发令旁注共用同一套：至少包括「获取资料 `get_profile`」「关注用户 `follow_user`」「打招呼 `send_greet`」「同步资料 `sync_profile`」「点赞 `like_post`」「发帖 `create_post`」「发快拍 `create_story`」。其余动作沿用发令页已有中文旁注；两边不得各写一套。

断点与键盘（跟成稿「断点与可访问」）：编辑画布以 ≥1280 为创作表面，三栏并列（面板、画布、参数）；992–1279 动作面板可折叠，参数改为右侧抽屉；<992 支持查看列表、发送与追踪，不作为编排主表面。面板可用回车把动作添加到链尾；选中节点后上下键调序、Delete 删除；拖拽必须有键盘等价操作。选中态要有焦点环。

**发送 Signal**：页内两个分段——发送定义、发送来源事件。成功后跳到既有追踪页并带上 `signal_id`。

**追踪文案**：整条 Signal 为进行中 / 成功 / 失败 / 已取消。步骤状态用文字：等待、执行中、成功、失败、已取消。等待要写清原因（等上一步成功、设备离线、与其他 Signal 的同类 Todo 冲突）。动作内部 pending 显示为执行中。失败步提供「重试失败的第 N 步」，不提供跳过。进行中可「取消整条 Signal」。

## 数据模型：定义与实例

定义不是 Signal，不进入管线，也不绑定设备。

### `signal_definitions`（新表）

| 字段 | 说明 |
|------|------|
| `id` | UUID |
| `user_id` | 所属后台用户 |
| `name` | 非空，Trim 后至少 1 个可见字符 |
| `steps` | JSONB，有序步骤快照：`[{ "ordinal": 1, "action": "get_profile", "params": { ... } }, ...]` |
| `archived_at` | 空表示可用；非空表示已归档，不得再被发送（打开编辑可另存） |
| `created_at` / `updated_at` | |

不存 `device_id`、`platform`、`account_id`。不把 `sessionid` 等禁字段写入 `params`。

保存校验（前后端都做，以后端为准）：

- 名称合法；`steps` 长度为 1–20。
- 必须是一条单链的序列化结果：`ordinal` 从 1 连续到 N，无缺口、无重复。画布上的分叉、环、孤立节点、空画布在保存或进入发送前拒绝，并返回原因，不得在服务端静默重排成运营没见过的顺序。自动排链只是编辑器的辅助，排完仍要运营确认后保存。
- 每个 `action` 必须在现有白名单内（与发令同一份，含 `pull_inbox` / `sync_profile`，不含 `send_reply`）。
- 每步 `params` 走既有 `validate_action_params`。身份字段仍由发送时写入，定义里不得出现、也不得被运营填写。
- 含 `create_post` / `create_story` 时允许保存，但接口可带 `warnings`（例如 `media_upload_not_ready`），供界面展示，不是校验失败。

### Signal 实例（沿用 `signals` 表）

发送时对当时的步骤和参数做一次快照，写入 `signals.payload`，并像现在一样写入 `device_id` 与绑定账号。之后再改定义，不影响已发出的实例。实例创建后 `device_id` 仍不可改。

`signals` 表本阶段不必加状态列。整条 Signal 的总状态由 Todos **派生**（见追踪）。

## 协议：`MANUAL_ACTION` payload 兼容

类型真源仍是 `crates/onia-protocol`。`ManualActionPayload` 扩展为两种合法形状，校验函数同时接受二者。

**旧形状（必须继续有效，发令页继续提交这种）：**

```json
{ "action": "follow_user", "params": { "target_handle": "alex" } }
```

规范化结果等价于 `steps: [{ "ordinal": 1, "action": "follow_user", "params": { ... } }]`。

**新形状（编辑后发送）：**

```json
{
  "definition_id": "可选 UUID",
  "definition_name": "可选名称，仅展示",
  "steps": [
    { "ordinal": 1, "action": "get_profile", "params": { "target_handle": "alex" } },
    { "ordinal": 2, "action": "follow_user", "params": { "target_handle": "alex" } }
  ]
}
```

规则：

- 出现非空 `steps` 时走新形状；**同时**再带顶层 `action` / `params` 则拒绝，避免歧义。
- `steps` 为空、缺 `ordinal`、序号不连续、动作未知、params 不合法，创建 Signal 失败，不落一条无 Todo 或半截 Todo 的事实。
- `definition_id` / `definition_name` 只是溯源元数据。定义已归档或已删时，只要快照本身合法，仍允许发送（发送页应提示定义已归档，但已填快照可继续发）。发令页不传这两个字段。
- `INBOX_ACTIVITY` / `PROFILE_SYNC_NEEDED` 的 payload 不变，不是 `steps`。

只增可缺省字段与新管理类型，不删旧字段，不升信封主版本 `v`。`just gen` 后生成物与协议一起交给 Git 助手，不要手改 `packages/protocol-ts`。

## Todo 补充字段与取消原因

新迁移（向前兼容）给 `todos` 增加：

| 字段 | 说明 |
|------|------|
| `step_index` | 从 1 开始。来源事件和旧数据视为 `1`。 |
| `wait_reason` | 可空。仅在 `pending` 时有意义：`predecessor`（等前一步成功）、`peer_conflict`（等其他 Signal 的同类 Todo）、`offline`（设备离线）。一旦晋升或进入终态，清掉。 |

新增 `CancelReason`：

- `predecessor_failed`：前一步最终失败，后续步取消。
- 运营取消整条 Signal，未完成步仍用已有的 `manual`。
- 换绑、缺能力、过期仍用现有原因。

不新增 Todo 状态枚举，等待一律保持 `pending` + `wait_reason`，避免看起来像卡住却没有原因。

`TodoPlan` 增加 `step_index`。追踪视图相应增加 `step_index`、`wait_reason`，以及派生的 Signal 总状态。

## 展开与串行

### Policy（纯函数，一次展开 N 条）

`manual` 包：

1. 只处理 `MANUAL_ACTION`。先把 payload 规范化为有序 `steps`（旧形状 → 1 步）。
2. **不再**用 `already_open` 丢弃计划。同一 Signal 内允许重复动作类型。
3. 为每一步产出一条 Todo：类型 / cap / 动作 仍 1:1；`params` 校验后写入，并补上发送时的 `platform`、`account_id`；`step_index = ordinal`；`playbook = manual`。
4. 创建 Signal 时就把 N 条 Todo 全部插入，状态为 `pending`。第 2 步及之后初始 `wait_reason = predecessor`。这样追踪从一开始就能看到整条链，而不是执行到才冒出来。

`INBOX_ACTIVITY` / `PROFILE_SYNC_NEEDED` 的 `already_open` **保留**：同类来源事件之间仍去重。它们与某条 `MANUAL_ACTION` 撞上相同动作时，不在 Policy 层合并或互取消，交给 Scheduler 等待。

### Scheduler（串行门禁，不改执行信封）

在现有设备钉死、能力检查、截止、离线等待之上，增加两个决定（建议作为 `PromotionDecision` 的新分支，由服务端填入 `TodoView`，Scheduler 仍无 I/O）：

- `WaitPredecessor`：同 Signal 中 `step_index = 当前 - 1` 的 Todo 尚未成功终态。
- `WaitPeerConflict`：同一设备上，**其他 Signal** 存在「已可执行或执行中」的同类动作 Todo（状态为无前置等待的 `pending`，或 `promoted` / `running`）。仍在等自己前一步的 Todo 不算冲突，以免一条尚未轮到的后续步把头堵死。

同一设备、同一动作同时只晋升一条。解除等待后按 `created_at` 先进先出，避免不确定抢跑。等待时更新 `wait_reason`，不改状态，不取消对方。

离线继续 `WaitOffline`，写 `wait_reason = offline`，不是失败。

`behavior.execute` 仍只带 `behavior_id`、`todo_id`、`action`、`params`。一次 Behavior 只跑一个动作。脚本内部跳转 / pending / resume 仍按上一轮方案，按 `behavior_id` 存在 Flutter。

### 服务端必须补上的衔接点

当前 `apply_behavior_result` 在**成功**时没有再调 `schedule_user`（失败自动重试才会）。单动作时不易暴露，多动作串行会让下一步永远停在「等前一步」。本阶段必须：

- 任一步成功、最终失败、被取消、运营重试之后，都再次 `schedule_user`。
- hello / 重连后继续走现有 `schedule_user`，以便离线恢复和冲突解除后晋升。

## 失败、取消与重试

失败分类不新增：`session` / `platform_risk` / `network` / `script` / `host`。动作内部 pending 不是失败，该步保持 `running`，后续不得开始。

自动重试沿用现有 `should_retry`（`session` / `platform_risk` / `script` 不自动重试；`network` / `host` 在次数内可重试）。未到最终失败前，不得取消后续步。

第 k 步最终失败（含重试用尽，或 live 下尚未打通的 `create_post` / `create_story` 失败）：

1. 该 Todo 为 `failed`。
2. 同 Signal 中 `step_index > k` 且尚未终态的 Todo，转为 `cancelled`，`cancel_reason = predecessor_failed`，**不得**标成 `failed`，也不得下发 Behavior。
3. 整条 Signal 派生为失败。

运营取消尚未全部完成的 Signal：

- 已成功步保持成功，不做业务回滚（不自动取关、撤回私信或取消点赞）。
- 未完成与未开始的步取消，原因为 `manual`。
- 若某步已有执行中的 Behavior，按现有取消路径处理该 Todo；设备侧已发出的副作用不回收。
- Signal 派生为取消。已进入终态（全部成功或已失败）的实例不再接受取消。

运营对**失败的那一步**重试（不允许跳过，不允许改已发出快照或从中间插步）：

- 仅当该 Todo 为 `failed`，且同 Signal 中没有更小 `step_index` 仍未成功时，才允许重试。
- 新增状态机事件（例如 `ManualRetry`）：`failed → pending`，并允许再执行一次（例如把 `max_attempts` 提到 `attempt + 1`，或单独记录人工重试次数，但不要绕过状态机直接改库）。
- 重试过程中，后续步保持 `cancelled`（`predecessor_failed`）。
- 该步再次最终失败：后续步维持取消，Signal 仍为失败。
- 该步成功：允许一次受控的 `cancelled → pending`，**仅限** `cancel_reason = predecessor_failed` 的后续步。紧接着的下一步清除等待或改为等前一步（此时前一步已成功，应可进入调度）；更后面的步仍是 `wait_reason = predecessor`。这是对「取消即终态」的唯一例外，不得用它做通用反取消。

要改编排：改定义后重新发送，使用新的实例和新的幂等键。同一次点击的重试沿用同一 `idempotency_key`；运营再次主动发送用新键。

换绑（`binding_changed`）仍取消该设备上未完成 Todo，一条序列里未跑完的步一并取消，不在换绑后再续跑到新设备。

## 与握手、脚本执行的衔接

客户端和脚本本阶段**几乎不用为多动作改协议形态**：

- 设备 hello、能力申报、WebSocket 长推、按 `behavior_id` 幂等回写、跳转后重新注入同一份 `social_proxy.js` 再 `resume`，全部保持。
- 宿主一次只收到一个 `behavior.execute`。链的「下一步」由服务端在上一步成功后再派，不在客户端排班。
- `host.demo === true` 时各动作仍返回模拟数据、不访问 Instagram，便于先测编辑 / 发送 / 串行与失败（`create_post` / `create_story` 的 demo 桩可以成功，但发送确认和追踪文案仍要标明「live 上传未打通」；live 路径必须 `demo: false`，失败不得回落演示数据）。
- 不把定义、步骤数组或第二套 pipeline 下发给客户端。

## 后台三页与接口契约

导航顺序：设备、Signal 编辑、发送 Signal、发令、追踪。「创建 Signal」改名为「发送 Signal」。旧路由 `/signals/create` 必须跳到发送页，避免书签失效。编辑页画布较宽，不要被现有内容最大宽度卡死，断点见「设计确认」。

### Signal 编辑（建议路由 `/signals/definitions`，编辑 `/signals/definitions/:id`）

左侧动作面板（与发令同一白名单和中文旁注；`create_post` / `create_story` 必须有「实打上传尚未打通」说明）、中间单链画布（拖入、调序、删除、复制节点）、右侧参数面板（复用发令表单组件，禁止出现 `sessionid` / `chunk_base` / `result_url` / `result_token`）。不选设备。

未保存不能发送。提供新建、打开、保存、另存、归档，以及「保存并去发送」（跳到发送页并预填该定义）。

接口（仍走现有后台信封 RPC，类型进 `onia-protocol`）：

- `GET/POST /v1/admin/signal-definitions` 列表（默认排除已归档，可查归档）、新建。
- `GET/PATCH/DELETE /v1/admin/signal-definitions/:id` 读取、更新、归档。`DELETE` 做归档，不是物理删除。
- 列表/详情返回 `id`、`name`、`steps`、`updated_at`、`archived_at` 和校验 `warnings`。

空画布、非单链、缺必填参数：保存失败并返回原因。

### 发送 Signal（`/signals/send`，旧 `/signals/create` 跳转过来）

两类发送，不在此页画流程图，也不把整页动态动作表单搬过来：

1. **按定义发送**：选一条未归档定义、选已绑定设备及其当前账号；确认页展示只读链，允许改本次参数；可选「写回定义」。提交时把快照 + 设备做成 `MANUAL_ACTION`，`POST /v1/admin/signals/create`（现有入口）。离线允许提交，对应步骤 `pending` + `wait_reason=offline`。
2. **来源事件**：继续只创建 `INBOX_ACTIVITY` / `PROFILE_SYNC_NEEDED`，payload 仍是简单来源标记。

成功后跳到既有追踪页，带上 `signal_id`。幂等键规则与发令相同。

写回定义是单独的定义更新，失败不得造成「Signal 已发出但运营以为没发出」的歧义：先保存定义（若勾选）、再创建实例；任一步失败要明确告诉运营哪一步已完成。

### 发令（`/commands`，本阶段不改成多动作）

继续提交旧形状 `{ action, params }` 的 `MANUAL_ACTION`。只含一个节点的定义经发送页发出后，与发令在追踪上同类可比：一条 Signal、一个动作、一条 Todo。两边必须共用同一份动作白名单和 params 校验。本阶段不合并两页。

行为变化（有意为之）：发令不再因「设备上已有同类未完成 Todo」而创建出 `todos_created = 0` 的空事实；改为仍创建这一条 Todo，在调度层等待。来源事件的去重保持不变。

### 取消与重试

- `POST /v1/admin/signals/:id/cancel`：取消整条未完成实例。
- `POST /v1/admin/todos/:id/retry`：仅失败步，语义见上。

两者都要服务端鉴权与状态校验，不能只靠前端藏按钮。

### 追踪

现有 `/v1/admin/trace/list`、`/v1/admin/trace/query` 扩展字段即可：

- Todo：按 `step_index` 排序；展示动作名、参数摘要、状态、`wait_reason`、`cancel_reason`、结果摘要。
- Signal 派生状态：全部 `succeeded` → 成功；存在 `failed`（且没有进行中的人工重试）→ 失败；存在 `manual` 取消且无失败 → 取消；否则为进行中（含离线等待、冲突等待、动作内部 pending）。
- 列表项增加 `state`、`step_count`（或 `todo_count`，已有）、进行到第几步。不要只展示 UUID。

## `create_post` / `create_story`

可出现在编辑画布、发令选项和已发送快照中。节点、发送确认必须提示「实打上传尚未打通」。live 失败 = 该步最终失败，后续取消。不得把 demo 成功当成 live 已打通，也不把打通上传列入本阶段必须完成项。

## Coder 需要改的目录和顺序

设计和本方案均已确认，可以按本文开工。不要先改信封字段名，不要做 `executePipeline`，不要回退未提交的登录、注入、发令和 live 改动。界面跟已确认的设计成稿，不要另发明一套导航或状态文案。

1. **`crates/onia-protocol`**：扩展 `ManualActionPayload` 与校验（旧形状 + `steps`）；`CancelReason::PredecessorFailed`；Todo / Trace 的 `step_index`、`wait_reason`；定义与取消 / 重试的请求响应类型；新 kind。`just gen`。
2. **迁移**：`signal_definitions`；`todos.step_index`（已有行回填 1）、`wait_reason`。只向前兼容。
3. **`crates/onia-policy` + `onia-scheduler` + `onia-todo` + `onia-server`**：一次展开 N 条 Todo；去掉 manual 包的 `already_open`；调度门禁；成功后再调度；失败取消后续；人工重试与受控反取消；定义 CRUD；Signal 取消。补单测：旧 payload、两步同名不同参数、前一步未成功不得晋升、失败后后续为取消、重试成功后下一步才开始、冲突等待、来源事件去重仍在。
4. **`apps/admin`**：编辑页、发送页改名与双入口、发令保持单动作、追踪按步展示。参数表单抽成发令与编辑器共用模块。
5. 脚本与 Flutter **原则上不改协议**。若追踪文案或 demo 需要标明发帖未打通，只改必要说明。
6. 更新后台 README 中与「创建 Signal」相关的名称。

实现完成后把进度和如何复现直接告诉用户。不要 commit / push，仓库与 MR 交给 Git 助手。代码评审交给 Code Reviewer，本方案不代替评审。

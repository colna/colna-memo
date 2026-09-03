
## Android elevation + 透明背景 = 白条/灰块伪影(2026-09-02)

**现象**:某个 View(如聊天顶栏)在 Android 上出现一条多余的浅色/白色矩形带 + 底部阴影;iOS 正常。
**根因**:该 View 设了 `elevation`(为了抬升/投影)但 `backgroundColor` 透明。Android 的 elevation 需要不透明背景来计算阴影轮廓,背景透明时会把 elevation 区域填成浅色块。
**修法**:二选一——①不需要投影就去掉 `elevation`(用 `zIndex` 控制层级即可);②需要投影就给该 View 一个不透明 `backgroundColor`。
**出处**:koda `apps/koda/src/components/chat/chat-header.tsx`,去掉 `elevation:10` 修掉顶栏白条。

## flex:1 文字在 hug 宽度父级里塌成竖排(每行 1 字母)(2026-09-02)

**现象**:某文字气泡/卡片里的长文本渲染成一列(每行一个字符),竖着排。
**根因**:容器用 `flexDirection:row` + 文字 `flex:1`,但该容器/其父级的宽度是「按内容 hug」而非确定值(常见叠加 `alignSelf:"stretch"`,让卡片去 stretch 到某个很窄的兄弟宽度)。没有确定主轴宽度时,`flex:1` 文字按 min-content 收缩 → 每行一个字。
**修法**:给该卡片/容器一个**确定宽度**(`width` 或 `maxWidth` 一个具体数),不要靠 `alignSelf:stretch` + `flex:1` 去撑。
**出处**:koda `chat-bubble.tsx` 的 ContactPrivacyNotice(发联系方式拦截提示),`alignSelf:stretch`→`width:268` 修好(R00228)。


## Android elevation + 透明背景 = 白条/灰块伪影(2026-09-02)

**现象**:某个 View(如聊天顶栏)在 Android 上出现一条多余的浅色/白色矩形带 + 底部阴影;iOS 正常。
**根因**:该 View 设了 `elevation`(为了抬升/投影)但 `backgroundColor` 透明。Android 的 elevation 需要不透明背景来计算阴影轮廓,背景透明时会把 elevation 区域填成浅色块。
**修法**:二选一——①不需要投影就去掉 `elevation`(用 `zIndex` 控制层级即可);②需要投影就给该 View 一个不透明 `backgroundColor`。
**出处**:koda `apps/koda/src/components/chat/chat-header.tsx`,去掉 `elevation:10` 修掉顶栏白条。

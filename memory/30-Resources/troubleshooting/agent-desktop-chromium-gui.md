---
title: agent-desktop 操作 Chromium / GUI 的坑（macOS）
date: 2026-11-05
tags: [troubleshooting, agent-desktop, macos, chrome, automation, accessibility]
---

# agent-desktop 操作 Chromium / GUI 的坑（macOS）

## 现象

- `list-windows --app "Google Chrome"` / `snapshot` 报 `WINDOW_NOT_FOUND`，但 Chrome 进程与窗口都还在（AppleScript 能读到标签页）。
- 网页里的 `⋮` 菜单、文件上传点不到；坐标点击偶尔偏。

## 根因与修法

1. **窗口枚举依赖前台激活**：先 `agent-desktop launch "Google Chrome" --activate`，再 `list-windows` / `snapshot`。窗口在后台放一会儿会再次枚举不到，操作前重新 activate。
2. **原生文件选择框（NSOpenPanel）不暴露给 `--window-id`**：`focus-window` / `press --window-id` 会报 `ACTION_NOT_SUPPORTED`，但**它内部的 element 会出现在整个 app 的 snapshot 里**。做法：对整个 app 做 snapshot，点侧边栏 `treeitem`（如「文稿」）再点文件行。
3. **不要用 `set-value` 填文件路径**：本次 `set-value` 落到了搜索框，路径被当成搜索词，`打开` 按钮一直 disabled。改用 UI 导航选文件。
4. **网页里的 `⋮` / 纯 CSS 图标经常不在 AX 树里**：`find`、snapshot 都找不到，只能坐标点击。坐标要精确换算：
   - 取 CGWindowID：Swift `CGWindowListCopyWindowInfo`（AppleScript 的 window id 不是 CGWindowID）。
   - `screencapture -l <CGWindowID> -o out.png` 得到**无阴影**窗口图（2x Retina）。
   - 映射：`screen_x = win_x + px/2`，`screen_y = win_y + py/2`。
   - `agent-desktop screenshot` 的图带 56pt 阴影边距，直接按 px/2 换算会偏。
5. **小窗口会把下拉菜单截断**：`resize-window --width 1100 --height 900` 放大后再点，`删除` 等菜单项才可见。
6. **物理点击报 pointer 未到位**：多半是弹层/模态挡住了目标。先用 `screencapture` 确认当前层级，关掉弹层再点。
7. **`type` 地址栏执行 `javascript:` 不可靠**：AX 插入会被 Chrome 当粘贴、去掉 scheme；本次未采用。

## 结论

agent-desktop 的语义点击（ref）在原生控件上很好用；网页内部图标按钮要准备坐标点击兜底。需要确认「到底点在哪」时，用 `screencapture -l` 无阴影窗口图核对。

---
title: IG mediaPk → shortcode 转化（app-ins-scripts）
date: 2026-10-15
tags: [instagram, sitin-next, app-ins-scripts, createPost]
---

# IG mediaPk → shortcode

- IG 帖子链接里的短码（如 `Ded8C5MtBhT`）= **mediaPk 的 URL-safe base64**（字母表 `A-Za-z0-9-_`，IG 固定算法）。
- **大数陷阱（核心）**：mediaPk 是 19 位十进制数，**超过 JS Number 安全整数上限 2^53**——`Number(pk)` 会四舍五入导致末位错（实测 `4007623325075642451` 用 Number 算法得 `Ded8C5MtBgA` ✗，正确为 `Ded8C5MtBhT` ✓）。必须用**字符串按位除 64** 实现（`createPost.js` 的 `pkToShortcode`）。
- **优先取 configure 响应里的 `media.code`**（实测存在，免算错），`media.user.username` 也在同一响应里（拼链接用），兜底才用算法。
- **双验证样本**：`4008352837356745882 → Degh6sdE_Sa`、`4007623325075642451 → Ded8C5MtBhT`。
- 链接格式：reel → `https://www.instagram.com/{username}/reel/{code}/`；photo/carousel → `/{username}/p/{code}/`（无 username 时可省略，短链仍可访问）。
- 落地：`createPost` result 返回 `shortCode` + `postUrl`（2026-10-14，commit `bc1bce9cc`）。

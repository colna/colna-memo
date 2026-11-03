---
title: macOS 取微信小程序 wxapkg 并解密 V1MMWX（REA 不直接吃 wxapkg）
date: 2026-11-03
tags: [troubleshooting, wechat, miniprogram, wxapkg, reverse-engineering]
---

# 微信小程序包：先取包、再解密，REA 只吃解包后的目录

## 一、取包卡点：macOS TCC 挡住 WMPF 容器

WeChat 4.1.15 启动参数里能看到 WMPF 根目录：

```text
~/Library/Containers/com.tencent.xinWeChat/Data/Documents/app_data/radium
```

但该容器受 TCC 保护，**未获 Full Disk Access 的进程 `ls` 直接 `Operation not permitted`**（`mdfind '*.wxapkg'` 也搜不到，因为 Spotlight 不索引容器内）。解法：给当前宿主 App（如 OpenCode.app）在「系统设置 → 隐私与安全性 → 完全磁盘访问权限」里打开 FDA，重开进程。

包的实际位置：

```text
.../radium/users/<user_hash>/applet/packages/wx<appid>/<version>/__APP__.wxapkg
```

前提是**这个账号在本机微信里真的打开过该小程序**（本次靠用户先点开分享短链 `#小程序://...` 才落地）。Windows 侧等价路径在 `WMPFRuntime/Applet`。

## 二、V1MMWX 解密（2.04MB 包实测）

老格式 `V1MMWX` 头不是明文 zip/wxapkg，算法（取自 wedecode 源码）：

1. `key = pbkdf2(appid, 'saltiest', 1000, 32, 'sha1')` —— **`appid` 是盐也是 key 派生因子**，拿错 appid 解出来就是乱码。
2. 前 1024 字节：`AES-256-CBC`（`iv` 取字面量 `"the iv: 16 bytes"`，**no padding**），解出后**只取前 1023 字节**（第 1024 字节是残位）。
3. 剩余字节：与 `appid[-2]`（appid 倒数第二个字符）逐字节 XOR。
4. 拼起来 = 标准 `0xBE` 开头的 wxapkg（可直接用 wxappUnpacker / unveilr 解）。

```js
const key = crypto.pbkdf2Sync(appid, 'saltiest', 1000, 32, 'sha1');
const d = crypto.createDecipheriv('aes-256-cbc', key, Buffer.from('the iv: 16 bytes'));
d.setAutoPadding(false);
const head = Buffer.concat([d.update(buf.subarray(0, 1024)), d.final()]).subarray(0, 1023);
const x = Buffer.from([appid.charCodeAt(appid.length - 2)]);
const tail = Buffer.from(buf.subarray(1024).map((b, i) => b ^ x[0]));
fs.writeFileSync('__APP__.wxapkg', Buffer.concat([head, tail]));
```

本次产物：221 个文件、`app-service.js` 897KB。

## 三、REA（`analyze_javascript_application`）的边界

- **不支持 `.wxapkg`**（支持 zip/apk/asar/js/wasm 等）→ 必须先解包成 JS/WXML/JSON 目录再跑。
- 扫 3 个 JS / 302k AST 节点 / 713 findings，但 **request 提取 0 endpoints**：uni-app 的 `uni.request` 封装它识别不了。接口清单要靠字符串提取（本次 117 条路径）补。

## 四、读功能栈的快速判据

- `app-config.json` 的 `tabBar` / `pages` 直接给出页面骨架（本次 85 页）。
- `app-service.js` 里 grep `baseUrl` / `https://` 能定位 API base；yudao/ruoyi-vue-pro 风格后端接口前缀通常是 `/app-api/<module>/*`。

## 相关

- 展商数据抓取的接口口径坑见 `10-Projects/meibohui-miniprogram.md`。

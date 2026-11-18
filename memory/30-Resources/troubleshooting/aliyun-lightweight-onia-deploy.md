---
title: "阿里云轻量部署 onia（swas-open + Docker + Caddy）"
date: 2026-11-16
tags: aliyun, lightweight, swas-open, docker, caddy, onia, deploy
---

# 阿里云轻量部署 onia（swas-open + Docker + Caddy）

**场景**：阿里云轻量应用服务器（Debian 12，2C2G，40G ESSD，海外机房），不用 SSH 也能初始化；后续用 Docker Compose 跑 PostgreSQL + Rust 服务 + Caddy。

## 关键命令

### 1. 云助手执行命令（无需 SSH）

```bash
aliyun --profile <p> swas-open RunCommand \
  --RegionId ap-southeast-3 \
  --InstanceId d55838ebced4405fab1c8185fe4d1e0d \
  --Name bootstrap --Type RunShellScript --Timeout 900 \
  --CommandContent "$(cat bootstrap.sh)"

aliyun --profile <p> swas-open DescribeInvocationResult \
  --RegionId ap-southeast-3 \
  --InstanceId d55838ebced4405fab1c8185fe4d1e0d \
  --InvokeId <id>
```

输出字段 `Output` 是 base64，需要解码。

### 2. 本地交叉编译（Apple Silicon → linux/amd64）

- 构建镜像用 `rust:1.85-bookworm`，运行层 `debian:12-slim`（glibc 对齐）。
- 构建上下文排除 `target/`、`node_modules/`、`.git/`、`.env`、Flutter/前端目录，否则会大到不可用。
- `docker buildx build --platform linux/amd64 --pull=false -t onia-server:latest --load .`
- Docker Hub 元数据偶发 `EOF`：先 `docker pull --platform linux/amd64 <image>`，再构建时加 `--pull=false`。

### 3. 上传

```bash
docker save onia-server:latest | gzip -1 | ssh root@IP 'gunzip | docker load'
rsync -az --delete -e "ssh -i ~/.ssh/key" apps/admin/dist/ root@IP:/opt/onia/admin/
```

### 4. Compose 要点

- PG named volume + `pg_isready` healthcheck，server `depends_on: condition: service_healthy`。
- server `ulimits.nofile: 65535`。
- Caddy `:80` 先反代 `/v1/*`、`/health`，其余 `try_files {path} /index.html` 给 Umi SPA。
- 日志 `json-file` 限 10m × 3，40G 盘必做。
- `.env` 在服务器生成、`chmod 600`，不要进 git。

## 坑

- **workbench CLI 只支持 ECS**（`list ecs`，`exec --instance-id i-xxx`），轻量实例 ID 是 32 位 hex，用不了。轻量用 `aliyun swas-open ...`。
- **域名 NXDOMAIN ≠ NS 没配**：先问注册局。`dig @a0.asia.afilias-nst.info <domain> NS` 返回 NXDOMAIN 表示域名没注册成功；只有 NOERROR 但无 NS 才轮到改 NS。
- 2C2G 不要在服务器上 `cargo build`（慢 + 易 OOM）；本地 buildx 交叉编译 3 分钟内完成。
- Caddy WebSocket 无需额外配置，`reverse_proxy` 自动升级；可用 `curl -H 'Upgrade: websocket' ...` 看是否返回 101。

## SSH 间歇性 KEX 超时时的部署

2026-11-18 遇到：TCP 22 可连，但 SSH KEX 阶段频繁超时（约一半概率），短命令多试几次能过，大文件管道不可靠。

- 传镜像：先 `docker save | gzip -1 > /tmp/img.tar.gz`，用 macOS 自带 rsync 2.6.9 的 `rsync -az --partial --append --timeout=60 -e "ssh ..."` 循环重试；2.6.9 **不支持** `--append-verify`。
- 远端操作走 `aliyun --profile onia swas-open RunCommand`（不受 SSH 影响）：校验 sha256、`docker load`、改 `.env`、`docker compose up -d --force-recreate`；结果用 `DescribeInvocationResult` 取 base64 `Output` 解码。
- 确认镜像含新代码：`docker cp <容器>:/usr/local/bin/onia-server` 出来 `strings | grep` 新路由/配置字符串。

## 验证

```bash
curl -s http://IP/health                 # ok
curl -s -o /dev/null -w '%{http_code}' http://IP/
# WS 升级：返回 101 Switching Protocols
```

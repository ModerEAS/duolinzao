# 多邻灶生产部署（Docker Swarm）

目标拓扑：两个节点，一个跑应用（role=app），一个跑 Postgres（role=db）；入口是挂 label 的 Traefik，外部网络名可配（默认 `traefik-public`）。发布方式：CI 在 master 测试全绿后构建镜像推 GHCR，再由 deploy job SSH 到 manager 做 `docker stack deploy`（start-first，失败自动 rollback）。**服务器上永远不 build。**

仓库是开源的，只放通用配置：域名、entrypoint、certresolver、外部网络名、部署路径全部变量化，见 `deploy/stack.yml` 头部注释。本文件不出现任何私有主机名/域名。

## 0. 与代码的关系

- 数据库迁移在**应用启动时自动运行**（`src/main.rs` 里 `sqlx::migrate!`），无需手工 `sqlx migrate run`；回滚镜像不会回滚迁移（迁移只向前，见第 5 节）。
- 健康探针 `GET /healthz`（纯文本 `ok`，不查库、不登录、免 CSRF），Traefik/CI 只看它；`GET /api/health` 仍在。
- 敏感配置全部走 external docker secret，应用从 `DLZ_*_FILE`（即 `/run/secrets/*`）读取，相关单测在 `src/config.rs`（`parse_with_env`）。

## 1. 首次准备（manager 上，一次性）

### 1.1 节点打标

stack 用 `node.labels.duolinzao.role` 做 placement（`app` 跑应用，`db` 跑 Postgres），先核对并打标：

```bash
docker node ls
docker node update --label-add duolinzao.role=app <应用节点名>
docker node update --label-add duolinzao.role=db <数据库节点名>
docker node inspect <节点> --format '{{.Spec.Labels}}'
```

### 1.1b 持久数据目录（bind mount）

持久数据不用命名卷，直接落宿主机目录。**每个会运行 app 或 db 的节点上，只需要事先建好同一个 `DEPLOY_PATH` 目录**（root 所有、权限 `755` 即可，例如 `sudo mkdir -p -m 755 /opt/duolinzao`）。子目录由容器启动时自动创建，CI 不需要 SSH 到 worker：

- db：把 `DEPLOY_PATH` 挂到 `/srv/duolinzao`，`PGDATA=/srv/duolinzao/pgdata`。官方 postgres 镜像的 entrypoint 以 root 启动，会自建 `pgdata/` 并设为 `999`、`0700`，首次启动自动 initdb。
- app：同样挂 `DEPLOY_PATH` → `/srv/duolinzao`。容器以 root 启动，只执行 `mkdir -p /srv/duolinzao/images && chown 65534:65534`，随即用 `setpriv` 降权到 nobody（65534）运行应用，菜品图写入 `/srv/duolinzao/images`（`deploy/config.toml` 的 `dish_image.dir`）。

`DEPLOY_PATH` 必须对容器内用户可遍历（至少 `755`），否则 postgres 初始化会报 `Permission denied`。app 以 nobody 运行时读不到 `pgdata/`（0700），两者互不影响。

### 1.2 网络

- 外部网（默认 `traefik-public`，可由 `TRAEFIK_NETWORK` 改名）：已存在、Traefik 在用，stack 里只引用不创建。
- `duolinzao-internal`：overlay + internal，由 stack 自动创建，db 只连它。

### 1.3 docker secret（5 个，全 external，stack 不创建）

```bash
DBPW='<强随机密码>'; DBURL="postgres://duolinzao:${DBPW}@db:5432/duolinzao"
printf '%s' "$DBPW"  | docker secret create dlz_db_password -
printf '%s' "$DBURL" | docker secret create dlz_database_url -
printf '%s' '<超管密码>' | docker secret create dlz_super_password -
# 32 字节 base64（LLM key 加密根密钥，丢了则已存渠道 key 解不开）：
openssl rand -base64 32 | tr -d '\n' | docker secret create dlz_local_key -
# 不用邮箱时填占位（stack 引用了 external secret，不存在会部署失败）：
printf '%s' 'unused' | docker secret create dlz_smtp_password -
docker secret ls
```

注意 `dlz_database_url` 里的密码必须和 `dlz_db_password` 是**同一个**（db 容器用后者设密码，app 用前者连库）。改密码时两个一起轮换。

### 1.4 swap（小内存节点建议加 1G）

```bash
fallocate -l 1G /swapfile && chmod 600 /swapfile && mkswap /swapfile && swapon /swapfile
echo '/swapfile none swap sw 0 0' >> /etc/fstab
free -h
```

### 1.5 GHCR 拉取

- 镜像默认**公开**最省事（GHCR package 设为 public），manager 无需登录；`GHCR_PULL_TOKEN` 可空着。
- 若设为私有：建一个只有 `read:packages` 的 token，填进仓库 secret `GHCR_PULL_TOKEN`，deploy 会先 `docker login ghcr.io`（用户名取仓库 owner，token 经 stdin 传入，不进远端命令行）。token 用 `[A-Za-z0-9_]` 字符。

## 2. GitHub 侧配置

### 2.1 Variables（Settings → Secrets and variables → Actions → Variables）

| 名称 | 默认值（未设时 stack 用） | 说明 |
|---|---|---|
| `DLZ_DOMAIN` | `example.com` | 域名；同时用于 Traefik Host 规则与 webauthn rp_id/origin |
| `TRAEFIK_ENTRYPOINT` | `web` | Traefik 入口点名 |
| `TRAEFIK_CERTRESOLVER` | `letsencrypt` | 证书解析器名 |
| `TRAEFIK_NETWORK` | `traefik-public` | 外部网络实际名（stack 引用 + `traefik.docker.network` 取同一值） |

### 2.2 Secrets（同位置 Secrets 页）

| 名称 | 说明 |
|---|---|
| `DEPLOY_HOST` | manager IP/域名 |
| `DEPLOY_PORT` | SSH 端口（可空，默认 22） |
| `DEPLOY_USER` | SSH 用户（能跑 docker，需在 docker 组） |
| `DEPLOY_SSH_KEY` | 私钥全文（只用于部署，建议单独建 deploy 专用 key） |
| `DEPLOY_KNOWN_HOSTS` | `ssh-keyscan -p 端口 HOST` 的输出；SSH 强制校验，防中间人 |
| `DEPLOY_PATH` | manager 上放 stack 的目录，如 `/opt/duolinzao` |
| `GHCR_PULL_TOKEN` | 可选；见 1.5 |

### 2.3 Environments

仓库 Settings → Environments 建 `production`，开 **Required reviewers**（至少一人点 Approve 才能发布）。deploy job 声明 `environment: production`，没过审批不会连服务器。

### 2.4 触发规则

deploy job 条件：主仓 + `refs/heads/master` + `push` 事件。fork 的 master push、任何 PR 都不会触发（另有 `environment` 二次把关）。master 每次 push 跑全量测试 → 镜像推送（`sha-<commit>` + `latest`）→ deploy 串行（`concurrency: deploy-production`）。

## 3. 发布流程（自动）

1. `test` / `e2e` / `e2e-scale` 全绿 → `image` 构建并推 `ghcr.io/moderras/duolinzao:sha-<commit>`（+ `latest`）。
2. `deploy`：算 `CONFIG_VERSION`（`deploy/config.toml` 的 sha256 前 12 位，配置一改名一变即触发服务更新），scp `stack.yml` + `config.toml` 到 `DEPLOY_PATH`；`IMAGE_TAG`/`DLZ_DOMAIN`/Traefik 四变量/`CONFIG_VERSION` 注入后 `docker stack deploy -c stack.yml --with-registry-auth duolinzao`。
3. 轮询 `duolinzao_app`：spec 镜像等于新 tag 且有 Running 任务、`UpdateStatus` 非失败态（10 分钟超时即失败；`failure_action: rollback` 已由 Swarm 自动回滚）。
4. runner `curl https://域名/healthz` 重试 5 分钟。
5. `docker image prune -f` 清旧镜像。

## 4. 回滚

- 自动：更新失败 Swarm 按 `update_config.failure_action: rollback` 回滚到上一版（`rollback_config` 已配）。
- 手动：`docker service rollback duolinzao_app`。
- 定版回滚：`DEPLOY_PATH=<同部署路径> IMAGE_TAG=sha-<旧commit> DLZ_DOMAIN=... TRAEFIK_ENTRYPOINT=... TRAEFIK_CERTRESOLVER=... TRAEFIK_NETWORK=... CONFIG_VERSION=... docker stack deploy -c stack.yml --with-registry-auth duolinzao`（DEPLOY_PATH 必须传，bind 路径靠它拼），或在 Actions 里对旧 commit 重跑 workflow（tag 不可变，旧镜像还在）。
- 注意：**迁移不回滚**。应用启动只向前迁移；回滚代码前确认旧代码兼容新表结构（一期 schema 只增不改，安全）。

## 5. 日常运维速查

```bash
docker stack services duolinzao
docker service ps duolinzao_app --no-trunc
docker service logs -f duolinzao_app
curl -fsS https://<域名>/healthz
docker volume ls | grep duolinzao   # 正常应无 duolinzao 卷：持久数据走 DEPLOY_PATH 下 bind mount
```

db 数据在宿主机目录 `DEPLOY_PATH/pgdata`（bind mount，随 db 节点 placement 固定）。备份即 tar 文件夹：`tar czf /backup/pg-$(date +%F).tgz -C <DEPLOY_PATH> pgdata`（停服或接受 crash-consistent）。恢复：停 db 服务后解压回去，保证属主仍是 UID 999。

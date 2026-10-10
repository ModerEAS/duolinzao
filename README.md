# 多邻灶（duolinzao）

部门内部报餐网页。仓库 / crate：`duolinzao`。

技术栈：**Vue3**（前端）+ **Rust Axum**（后端）。

一期需求已锁定，见 [docs/requirements-phase1.md](docs/requirements-phase1.md)。

用户故事、技术方案与实施计划见 [specs/phase1/requirements.md](specs/phase1/requirements.md)。

界面参考见 [docs/ui/README.md](docs/ui/README.md)：员工竖屏，以及食堂横屏（左侧栏为主）。食堂端画面清单在一期需求第 11 节。

## 本地运行

1. 准备 PostgreSQL，并复制配置：

```bash
cp config.example.toml config.toml
```

2. 填掉所有 `REPLACE_ME`。截止钟点、三餐餐后时刻、均分阈值、样本量下限、图片目录、单张大小上限和本地加密密钥都没有代码默认值，缺一项进程会拒绝启动。邮箱能力默认关闭，此时可以不填 SMTP。`local_key` 是 32 字节密钥的 base64。

3. 启动：

```bash
export DATABASE_URL=postgres://duolinzao:duolinzao@127.0.0.1:5432/duolinzao
# 把 config.toml 里的 database.url 指到同一个库
cargo run
```

数据库迁移在进程启动时自动执行。也可以只用数据库容器：`docker compose up db`。业务镜像见 `Dockerfile`，Postgres 不打进业务镜像，餐品图片目录需要挂载。

前端在 `web/`。开发时 Vite 把 `/api` 代理到 `127.0.0.1:8080`：

```bash
cd web && npm install && npm run dev
```

集成测试：

```bash
cargo test
```

测试会在本机 Postgres 上创建 `duolinzao_test`。默认走本机 peer 套接字 `/var/run/postgresql`。需要改管理库连接时设置 `DATABASE_URL`。

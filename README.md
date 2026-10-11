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

## Docker / compose

```bash
cp config.docker.example.toml config.docker.toml   # 再填掉 REPLACE_ME
docker compose build            # 打镜像，名字与 CI 推的 GHCR 一致：ghcr.io/moderras/duolinzao
docker compose up               # Postgres 旁路 + app（静态前端由 app 自己出）
```

`config.docker.example.toml` 与 `config.example.toml` 相同，只把主机名和路径换成容器里的：`database.url` 用 `db`，`static_dir=/app/static`，餐品图片在 `duolinzao-images` 卷里（容器以 nobody 运行，镜像已把该目录的属主交给它）。只需要数据库时：`docker compose up db`。

前端在 `web/`。开发时 Vite 把 `/api` 代理到 `127.0.0.1:8080`：

```bash
cd web && npm install && npm run dev
```

集成测试：

```bash
cargo test
```

测试会在本机 Postgres 上创建 `duolinzao_test`。默认走本机 peer 套接字 `/var/run/postgresql`，管理库连接可用 `DATABASE_URL` 指定；测试库本身用 `TEST_DATABASE_URL` 覆盖（CI 里没有套接字，走 `postgres://postgres:postgres@127.0.0.1:5432/duolinzao_test`）。

## 浏览器测试（Playwright）

`e2e/` 下是 Playwright 规格，对照根目录 `features/*.feature`（Gherkin 中文场景）覆盖员工主路径：登录入口顺序与无短信 OTP、邀请码注册、员工竖屏、绑定食堂、班制只有上二休四/自定义、本周报餐绿（报）/橙（不报）切换。

```bash
# 需要先有一个可用 Postgres；脚本会生成临时配置、起 app、预置食堂与邀请码，再跑 Playwright
make e2e                 # = web 构建 + cargo build + npx playwright install + bash scripts/e2e.sh
```

失败时截图 / trace / video 和 HTML 报告留在 `e2e/test-results`、`e2e/playwright-report`，后端日志在 `e2e/app.log`。对着 Vite dev server 调试时可设 `DLZ_BASE_URL=http://127.0.0.1:5173`（此时脚本不再自己起 app）。

## CI / 交付

`.github/workflows/ci.yml`：

- **每次 PR**、以及推 `master` / `v*` tag：先跑 `cargo test`（Postgres service）与 Playwright 浏览器测试；两者都过了才 build 镜像。
- **只有**推到 `master` 或 `v*` tag 才 push 到 GHCR：`ghcr.io/moderras/duolinzao`（镜像名必须全小写）。PR 只 build，绝不 push。
- 失败时上传 `playwright-report`（截图 / trace）与 `e2e-app-log` 产物。
- CI 只用测试用配置与 seed 数据，不调用任何外部 LLM（模板生成器的渠道在运行期由总管配置，测试不碰）。

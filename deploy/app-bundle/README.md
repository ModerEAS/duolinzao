# `deploy/app-bundle/`：CI 填充的 APK 产物目录（不进 git）

`android-apk` job 在每次 CI 构建出两份包，`image` job 用
`actions/download-artifact` 拉到这里，再由 Dockerfile 拷进镜像：

- `prod/duolinzao.apk` + `prod/version.json` → 镜像 `/app/static/app/`
  （生产默认 `{static_dir}/app`，baseUrl 用仓库级 `DLZ_DOMAIN`）。
- `staging/duolinzao.apk` + `staging/version.json` → 镜像 `/app/static/app-staging/`
  （staging stack 设 `DLZ_APP_DIR=/app/static/app-staging`，baseUrl 用仓库级
  `STAGING_DLZ_DOMAIN`；两份包同镜像、路由同为 `/app/*`，靠目录隔离）。

本地 `docker build` 时这里只有 `.gitkeep`（见 `.gitignore`），两个目录都拷空壳，
构建照样成功，`/app/version.json` 保持 404（App 显示“不可用”，不推坏包）。

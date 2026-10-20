# 多邻灶一期镜像：Axum 二进制 + Vue 构建产物。数据库由部署方单独运行。
# 餐品图片放在挂载目录，不打进镜像。许可证 AGPL-3.0。

FROM node:22-bookworm-slim AS web
WORKDIR /src/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ .
RUN npm run build

FROM rust:1.88-bookworm AS rust
WORKDIR /src
# 页脚版本：优先 CI 传入的 DLZ_VERSION（Gitea 风格归一化串）；无则 build.rs
# 用 GITHUB_SHA 拼 `{cargo_ver}+0-g{sha}`，本地则回退 `{cargo_ver}+unknown`。
# .dockerignore 默认忽略 .git，构建上下文里没有 git 历史，不做 git describe。
ARG DLZ_VERSION=""
ARG GITHUB_SHA=""
ENV DLZ_VERSION=${DLZ_VERSION} GITHUB_SHA=${GITHUB_SHA}
COPY Cargo.toml Cargo.lock build.rs ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY LICENSE /app/LICENSE
COPY --from=rust /src/target/release/duolinzao /app/duolinzao
COPY --from=web /src/web/dist /app/static
# 镜像以 nobody 运行：先把餐品图片目录建好并交给它，命名卷才会继承属主。
RUN mkdir -p /var/lib/duolinzao/images && chown -R 65534:65534 /var/lib/duolinzao
ENV DUOLINZAO_CONFIG=/config/config.toml
EXPOSE 8080
USER 65534:65534
ENTRYPOINT ["/app/duolinzao"]

# 多邻灶一期镜像：Axum 二进制 + Vue 构建产物。数据库由部署方单独运行。
# 餐品图片放在挂载目录，不打进镜像。许可证 AGPL-3.0。

FROM node:22-bookworm-slim AS web
WORKDIR /src/web
COPY web/package.json web/package-lock.json* ./
RUN npm install
COPY web/ .
RUN npm run build

FROM rust:1.88-bookworm AS rust
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY LICENSE /app/LICENSE
COPY --from=rust /src/target/release/duolinzao /app/duolinzao
COPY --from=web /src/web/dist /app/static
ENV DUOLINZAO_CONFIG=/config/config.toml
EXPOSE 8080
USER 65534:65534
ENTRYPOINT ["/app/duolinzao"]

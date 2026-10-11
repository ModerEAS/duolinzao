.PHONY: run test fmt lint e2e e2e-install web-build image up

run:
	cargo run

test:
	cargo test

fmt:
	cargo fmt

lint:
	cargo clippy --all-targets

# 浏览器测试的前置：前端构建 + 后端构建 + Playwright 浏览器。
# 还需要一个可用 Postgres（DATABASE_URL，默认 postgres://duolinzao:duolinzao@127.0.0.1:5432/duolinzao）。
web-build:
	cd web && npm ci && npm run build

e2e-install: web-build
	cargo build
	cd e2e && npm ci && npx playwright install chromium

e2e: e2e-install
	bash scripts/e2e.sh

# 本地镜像：与 CI 推的 GHCR 同名（全小写）。
image:
	docker compose build

# 起栈前先 cp config.docker.example.toml config.docker.toml 并填掉 REPLACE_ME。
up:
	docker compose up

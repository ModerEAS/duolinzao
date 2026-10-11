#!/usr/bin/env bash
# 本地 / CI 共用的 e2e 栈：生成测试配置 → 起 Postgres 上的 app → 预置数据 → Playwright。
# 前置：Postgres 已可用（DATABASE_URL）、target/debug/duolinzao 与 web/dist 已构建、
# 已 npx playwright install chromium。CI 里由 .github/workflows/ci.yml 负责这些。
set -euo pipefail
cd "$(dirname "$0")/.."

DB_URL=${DATABASE_URL:-postgres://duolinzao:duolinzao@127.0.0.1:5432/duolinzao}
PORT=${DLZ_PORT:-8080}
# 规模化旅程用独立库并每轮重建：拨钟会写入未来时间戳的行（审计按 created_at 倒序取 200），
# 与默认库混用会把切片用例的新行挤出窗口。独立库即隔离，默认库零影响。
if [ -n "${DLZ_SCALE:-}" ]; then
  SCALE_URL=$(DB_URL="$DB_URL" node -e 'const u = new URL(process.env.DB_URL); u.pathname = "/duolinzao_scale"; console.log(u.toString());')
  psql "$DB_URL" -v ON_ERROR_STOP=1 \
    -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = 'duolinzao_scale' AND pid <> pg_backend_pid();" \
    -c "DROP DATABASE IF EXISTS duolinzao_scale" \
    -c "CREATE DATABASE duolinzao_scale"
  DB_URL=$SCALE_URL
fi
BASE_URL=${DLZ_BASE_URL:-http://localhost:$PORT}
BIN=${DLZ_BIN:-target/debug/duolinzao}
INVITE_CODE=${DLZ_E2E_INVITE_CODE:-DLZ-E2E-INVITE}
CANTEEN=${DLZ_E2E_CANTEEN:-一食堂}
RUN_DIR=$PWD/e2e/.tmp
CONFIG=$RUN_DIR/config.toml

mkdir -p "$RUN_DIR"

# 1. 从 config.example.toml 生成临时配置：只填最小合法值，不含真实密钥。
DLZ_CONFIG_OUT=$CONFIG \
DLZ_DB_URL=$DB_URL \
DLZ_STATIC_DIR=$PWD/web/dist \
DLZ_BIND=127.0.0.1:$PORT \
DLZ_RP_ORIGIN=http://localhost:$PORT \
DLZ_IMAGE_DIR=$RUN_DIR/dish-images \
node - <<'JS'
const fs = require("node:fs");
let text = fs.readFileSync("config.example.toml", "utf8");
const set = (section, key, value) => {
  const re = new RegExp(`(\\[${section}\\][\\s\\S]*?\\n${key} = )[^\\n]*`);
  if (!re.test(text)) throw new Error(`config.example.toml 缺少 ${section}.${key}`);
  text = text.replace(re, `$1${value}`);
};
set("super_admin", "username", '"root"');
set("super_admin", "password", '"root-password-1"');
set("webauthn", "rp_id", '"localhost"');
set("webauthn", "rp_origin", `"${process.env.DLZ_RP_ORIGIN}"`);
// e2e 假 SMTP：只为让「邮箱开关」场景能开启能力并看到 magic link，不真实发信。
// 步骤里从不调用 magic-link/request，所以不会连 smtp.example.invalid。
set("smtp", "host", '"smtp.example.invalid"');
set("smtp", "from", '"e2e@example.invalid"');
set("schedule", "lock_time", '"09:00"');
set("schedule", "breakfast_after", '"09:30"');
set("schedule", "lunch_after", '"13:30"');
set("schedule", "dinner_after", '"19:30"');
set("score_alert", "mean_threshold", '"-0.5"');
set("score_alert", "min_samples", '"5"');
set("crypto", "local_key", '"aj8Y3Os8bzTXUCXvj1LV6xbjEtHtBPGY3Gc2n0aNXhA="');
set("dish_image", "dir", `"${process.env.DLZ_IMAGE_DIR}"`);
set("dish_image", "max_bytes", '"2097152"');
set("server", "bind", `"${process.env.DLZ_BIND}"`);
set("server", "static_dir", `"${process.env.DLZ_STATIC_DIR}"`);
set("database", "url", `"${process.env.DLZ_DB_URL}"`);
if (text.includes('"REPLACE_ME"')) throw new Error("测试配置里还有 REPLACE_ME");
fs.writeFileSync(process.env.DLZ_CONFIG_OUT, text);
JS

# 2. 起 app（若 DLZ_BASE_URL 指向别处，例如 Vite dev server，则跳过）。
# DLZ_ALLOW_TEST_CLOCK=1 只开给 e2e 栈内的 app：放行 /api/test/clock（仍只认 super_admin），
# 生产配置（config.*.example.toml / compose）里永不设置。
started=0
if [ -z "${DLZ_BASE_URL:-}" ]; then
  DLZ_ALLOW_TEST_CLOCK=1 DUOLINZAO_CONFIG=$CONFIG "$BIN" >e2e/app.log 2>&1 &
  app_pid=$!
  started=1
  trap 'kill "$app_pid" 2>/dev/null || true' EXIT
fi

for _ in $(seq 1 60); do
  curl -fsS "$BASE_URL/api/health" >/dev/null 2>&1 && break
  sleep 1
done
curl -fsS "$BASE_URL/api/health" >/dev/null || {
  echo "app 未在 $BASE_URL 就绪，见 e2e/app.log" >&2
  exit 1
}
[ "$started" = 1 ] || echo "复用 $BASE_URL 上已运行的服务"

# 3. 预置食堂 + 邀请码（库内存哈希 sha256 hex）。
CANTEEN2=${DLZ_E2E_CANTEEN2:-二食堂}
HASH=$(node -e 'console.log(require("node:crypto").createHash("sha256").update(process.argv[1]).digest("hex"))' "$INVITE_CODE")
psql "$DB_URL" -v ON_ERROR_STOP=1 \
  -v canteen="$CANTEEN" -v canteen2="$CANTEEN2" -v hash="$HASH" -v super_user=root \
  -f e2e/seed.sql

# 3b. 角色与渠道：二食堂兜底 + 本地脚本 LLM 渠道 + canteen/admin 账号与分配（走真实 API，幂等）。
DLZ_BASE_URL="$BASE_URL" node e2e/setup-roles.mjs

# 3c. 规模化旅程花名册：DLZ_SCALE=smoke（默认 3 食堂 × 30 员工）/ full（10 × 300）。
# 未设 DLZ_SCALE 时不生成、不跑 @scale；设了则 bdd 只生成 @scale、timeout 放宽到 30 分钟。
if [ -n "${DLZ_SCALE:-}" ]; then
  DLZ_SCALE="$DLZ_SCALE" DLZ_SEED="${DLZ_SEED:-}" node e2e/scale/roster.mjs
  export DLZ_BDD_TAGS="${DLZ_BDD_TAGS:-@scale}"
  export DLZ_E2E_TIMEOUT_MS="${DLZ_E2E_TIMEOUT_MS:-1800000}"
fi

# 4. 由 features/*.feature 生成用例（默认排除 @wip 与 @scale），再跑浏览器测试。
cd e2e
npx bddgen --tags "${DLZ_BDD_TAGS:-not @wip and not @scale}"
npx playwright test "$@"

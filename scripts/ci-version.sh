#!/usr/bin/env bash
# Gitea 风格版本串（见 AGENTS.md「Version / Release」），供 CI 各 job 复用，
# 避免 image job 与 android-apk job 各写一份漂移：
#   - 正好落在 vX.Y.Z tag：去掉 v，不附 commit；
#   - 否则把 `git describe --always` 第一个 `-` 换成 `+` 再去 v
#     （如 0.1.5+12-gc7d0119）；无 v* tag 时拼 `{Cargo 版本}+0-g{sha}`；
#   - 无 git 时回退 `{Cargo 版本}+unknown`。
# 用法：VER="$(bash scripts/ci-version.sh)"（仓库根目录下执行）。
set -euo pipefail

CARGO_VER="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)"
RAW="$(git describe --tags --always 2>/dev/null || true)"
if [ -z "$RAW" ]; then
  VER="${CARGO_VER}+unknown"
elif [[ "$RAW" == *-* ]]; then
  VER="${RAW/-/+}"
  VER="${VER#v}"
elif [[ "$RAW" == v* ]]; then
  VER="${RAW#v}"
elif [ "$RAW" = "$CARGO_VER" ]; then
  VER="$RAW"
else
  VER="${CARGO_VER}+0-g${RAW}"
fi
printf '%s\n' "$VER"

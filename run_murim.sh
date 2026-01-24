#!/bin/sh
# 빌드 후 기존 murim_server를 종료하고 서버를 (재)시작합니다.
# 사용: ./run_murim.sh

set -e
cd "$(dirname "$0")"

echo "=== cargo build --bin murim_server ==="
cargo build --bin murim_server

echo "=== 기존 murim_server 종료 ==="
pkill -f murim_server 2>/dev/null || true
sleep 1

echo "=== cargo run --bin murim_server ==="
exec cargo run --bin murim_server

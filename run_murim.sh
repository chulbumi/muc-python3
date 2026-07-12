#!/bin/sh
# Rust MUD 서버를 빌드한 뒤 현재 터미널에서 실행합니다.
# 사용:
#   ./run_murim.sh             # 기본 포트 9999
#   ./run_murim.sh 9998        # 지정 포트
#   MUD_PORT=9998 ./run_murim.sh

set -eu
cd "$(dirname "$0")"

PORT="${1:-${MUD_PORT:-${PORT:-9999}}}"
case "$PORT" in
    ''|*[!0-9]*)
        echo "오류: 포트는 숫자여야 합니다: $PORT" >&2
        exit 2
        ;;
esac

if [ "$PORT" -lt 1 ] || [ "$PORT" -gt 65535 ]; then
    echo "오류: 포트 범위는 1~65535입니다: $PORT" >&2
    exit 2
fi

echo "=== cargo build --bin murim_server ==="
cargo build --bin murim_server

echo "=== 기존 murim_server 종료 ==="
OLD_PIDS="$(pgrep -x murim_server 2>/dev/null || true)"
if [ -n "$OLD_PIDS" ]; then
    kill $OLD_PIDS 2>/dev/null || true

    # 정상 종료를 잠시 기다리고, 남아 있는 이전 서버만 강제 종료합니다.
    WAIT_COUNT=0
    while [ "$WAIT_COUNT" -lt 20 ] && pgrep -x murim_server >/dev/null 2>&1; do
        sleep 0.1
        WAIT_COUNT=$((WAIT_COUNT + 1))
    done

    REMAINING_PIDS="$(pgrep -x murim_server 2>/dev/null || true)"
    if [ -n "$REMAINING_PIDS" ]; then
        echo "기존 서버가 종료되지 않아 강제 종료합니다: $REMAINING_PIDS"
        kill -KILL $REMAINING_PIDS 2>/dev/null || true
        sleep 0.2
    fi
fi

if pgrep -x murim_server >/dev/null 2>&1; then
    echo "오류: 기존 murim_server를 종료하지 못했습니다." >&2
    exit 1
fi

echo "=== Rust MUD 서버 시작: 0.0.0.0:$PORT ==="
exec ./target/debug/murim_server "$PORT"

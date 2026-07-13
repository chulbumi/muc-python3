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

echo "=== 포트 $PORT 점유 확인 ==="
if command -v fuser >/dev/null 2>&1; then
    OLD_PIDS="$(fuser -n tcp "$PORT" 2>/dev/null || true)"
else
    OLD_PIDS=""
    if command -v ss >/dev/null 2>&1; then
        SS_OUTPUT="$(ss -ltnp "sport = :$PORT" 2>/dev/null || true)"
        if echo "$SS_OUTPUT" | grep -q LISTEN; then
            OLD_PIDS="$(echo "$SS_OUTPUT" | sed -n 's/.*pid=\([0-9][0-9]*\).*/\1/p')"
            if [ -z "$OLD_PIDS" ]; then
                echo "오류: 포트 $PORT가 사용 중이지만 점유 프로세스를 안전하게 확인할 수 없습니다." >&2
                echo "확인: ss -ltnp 'sport = :$PORT'" >&2
                exit 1
            fi
        fi
    fi
fi

if [ -n "$OLD_PIDS" ]; then
    for PID in $OLD_PIDS; do
        COMM="$(cat "/proc/$PID/comm" 2>/dev/null || true)"
        if [ "$COMM" != "murim_server" ]; then
            echo "오류: 포트 $PORT를 다른 프로세스가 사용 중입니다: PID $PID ($COMM)" >&2
            exit 1
        fi
    done

    echo "=== 포트 $PORT의 기존 murim_server 종료: $OLD_PIDS ==="
    kill $OLD_PIDS 2>/dev/null || true

    # 정상 종료를 잠시 기다리고, 같은 포트에 남은 이전 서버만 강제 종료합니다.
    WAIT_COUNT=0
    while [ "$WAIT_COUNT" -lt 20 ] && kill -0 $OLD_PIDS 2>/dev/null; do
        sleep 0.1
        WAIT_COUNT=$((WAIT_COUNT + 1))
    done

    REMAINING_PIDS=""
    for PID in $OLD_PIDS; do
        if kill -0 "$PID" 2>/dev/null; then
            REMAINING_PIDS="$REMAINING_PIDS $PID"
        fi
    done
    if [ -n "$REMAINING_PIDS" ]; then
        echo "기존 서버가 종료되지 않아 강제 종료합니다: $REMAINING_PIDS"
        kill -KILL $REMAINING_PIDS 2>/dev/null || true
        sleep 0.2
    fi
fi

echo "=== Rust MUD 서버 시작: 0.0.0.0:$PORT ==="
exec ./target/debug/murim_server "$PORT"

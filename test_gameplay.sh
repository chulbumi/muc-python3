#!/bin/bash
# Detailed gameplay comparison test

echo "=== PYTHON MUD (9900) Gameplay Test ==="
(
  sleep 0.3
  echo "무명객"
  sleep 0.3
  echo ""
  sleep 0.3
  echo "보기"
  sleep 0.3
  echo "인벤토리"
  sleep 0.3
  echo "무공"
  sleep 0.3
  echo "상태"
  sleep 0.3
  echo "8"  # 북 (North)
  sleep 0.3
  echo "보기"
  sleep 0.3
) | nc localhost 9900 2>/dev/null | tail -40

echo ""
echo "=== RUST MUD (9990) Gameplay Test ==="
(
  sleep 0.3
  echo "무명객"
  sleep 0.3
  echo ""
  sleep 0.3
  echo "보기"
  sleep 0.3
  echo "인벤토리"
  sleep 0.3
  echo "무공"
  sleep 0.3
  echo "상태"
  sleep 0.3
  echo "8"  # 북 (North)
  sleep 0.3
  echo "보기"
  sleep 0.3
) | nc localhost 9990 2>/dev/null | tail -40

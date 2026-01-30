#!/bin/bash
# Test both servers with full login flow

echo "=== Testing Python MUD (9900) ==="
{
  sleep 0.5
  echo "테스터"
  sleep 0.3
  echo "test1234"
  sleep 0.3
  echo ""
  sleep 0.5
  echo "보기"
  sleep 0.5
  echo "status"
  sleep 0.5
} | nc localhost 9900 2>/dev/null | head -80

echo ""
echo "=== Testing Rust MUD (9990) ==="
{
  sleep 0.5
  echo "테스터"
  sleep 0.3
  echo "test1234"
  sleep 0.3
  echo ""
  sleep 0.5
  echo "보기"
  sleep 0.5
  echo "status"
  sleep 0.5
} | nc localhost 9990 2>/dev/null | head -80

#!/bin/bash
# Simple test script to compare Python and Rust MUD

echo "=== Testing Python MUD (port 9900) ==="
echo "보기" | nc -q 1 localhost 9900 2>/dev/null | head -50

echo ""
echo "=== Testing Rust MUD (port 9990) ==="
echo "보기" | nc -q 1 localhost 9990 2>/dev/null | head -50

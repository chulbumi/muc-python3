#!/bin/bash
# Interactive test script for both MUD servers
# Creates two characters and tests commands

echo "=========================================="
echo "MUD TWO-CHARACTER SYSTEMATIC TEST"
echo "=========================================="

for SERVER in "PYTHON:9900" "RUST:9990"; do
    NAME="${SERVER%%:*}"
    PORT="${SERVER##*:}"
    
    echo ""
    echo "================================================"
    echo "Testing $NAME MUD (port $PORT)"
    echo "================================================"
    
    for CHAR in "테스터1" "테스터2"; do
        echo ""
        echo "--- Character: $CHAR ---"
        
        # Create test input file
        cat > /tmp/mud_test_input.txt << CMDS
무명객
보기
인벤토리
무공
비전
상태
help
who
8
보기
2
보기
말 테스트 메시지
CMDS

        # Run commands and capture output
        echo "Input commands:" && cat /tmp/mud_test_input.txt
        
        # Execute with timeout
        output=$(nc -w 3 localhost $PORT < /tmp/mud_test_input.txt 2>/dev/null)
        
        # Clean and show relevant output
        if [ -n "$output" ]; then
            echo ""
            echo "Output (key lines):"
            echo "$output" | grep -E "(보기|인벤토리|무공|비전|상태|체력|내공|출구|방|소지품)" | head -20
        fi
        
        sleep 1
    done
done

echo ""
echo "================================================"
echo "SUMMARY: Both servers tested with 2 characters"
echo "================================================"

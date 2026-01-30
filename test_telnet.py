#!/usr/bin/env python3
import telnetlib
import time
import re

def clean(text):
    return re.sub(r'\x1b\[[0-9;]*[mHJK]', '', text)

def test_server(port, name):
    print(f"\n{'='*60}\n{name} (port {port})\n{'='*60}")
    
    try:
        tn = telnetlib.Telnet('localhost', port, timeout=3)
        
        # Get initial
        time.sleep(0.5)
        initial = tn.read_very_eager().decode('utf-8', errors='ignore')
        print(f"Banner: {clean(initial)[:100]}...")
        
        # Login
        tn.write("테스터\n".encode('utf-8'))
        time.sleep(0.3)
        
        # Check response
        resp = tn.read_very_eager().decode('utf-8', errors='ignore')
        print(f"After name: {clean(resp)[:100]}...")
        
        # Enter password or continue
        tn.write("test\n".encode('utf-8'))
        time.sleep(0.3)
        
        tn.write("\n".encode('utf-8'))
        time.sleep(0.3)
        
        # Get game screen
        game = tn.read_very_eager().decode('utf-8', errors='ignore')
        print(f"\nGame screen:\n{clean(game)[:400]}")
        
        # Test 보기
        tn.write("보기\n".encode('utf-8'))
        time.sleep(0.5)
        look = tn.read_very_eager().decode('utf-8', errors='ignore')
        print(f"\n=== 보기 ===\n{clean(look)[:300]}")
        
        tn.close()
        return True
    except Exception as e:
        print(f"Error: {e}")
        return False

test_server(9900, "Python MUD")
test_server(9990, "Rust MUD")

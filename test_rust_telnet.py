#!/usr/bin/env python3
import telnetlib
import time

def test_command(cmd):
    try:
        tn = telnetlib.Telnet("localhost", 9999, timeout=5)
        time.sleep(0.5)

        # Read greeting - wait for password prompt after name
        data = tn.read_until(b"Password", timeout=2)
        
        # Send name
        tn.write("테스터러스트\n".encode('utf-8'))
        time.sleep(0.5)

        # Send password
        tn.write("1234\n".encode('utf-8'))
        time.sleep(1)

        # Read until prompt
        data = tn.read_until(b"[ 450/900", timeout=3)
        
        # Send command
        tn.write((cmd + "\n").encode('utf-8'))
        time.sleep(1)

        # Read response
        response = tn.read_until(b"[ 450/900", timeout=3)
        
        tn.close()
        return response.decode('utf-8', errors='ignore')
    except Exception as e:
        return f'ERROR: {e}'

# Test commands
commands = ['능력치', '점수', '무공', '소지품', '누구', '봐']
for cmd in commands:
    print(f'===== {cmd} =====')
    output = test_command(cmd)
    print(output[:800])
    print()

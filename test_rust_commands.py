#!/usr/bin/env python3
import socket
import time

def test_command(cmd):
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.settimeout(3)
    try:
        s.connect(('localhost', 9999))
        time.sleep(0.3)

        # Login
        data = s.recv(4096).decode('utf-8', errors='ignore')
        s.sendall('테스터러스트\n'.encode('utf-8'))
        time.sleep(0.3)

        data = s.recv(4096).decode('utf-8', errors='ignore')
        s.sendall('1234\n'.encode('utf-8'))
        time.sleep(0.5)

        # Clear welcome messages
        for _ in range(3):
            try:
                data = s.recv(4096).decode('utf-8', errors='ignore')
                if '[ 450/900' in data or '[450/900' in data:
                    break
            except:
                break

        # Send command
        s.sendall((cmd + '\n').encode('utf-8'))
        time.sleep(0.5)

        # Get response
        responses = []
        for _ in range(5):
            try:
                data = s.recv(4096).decode('utf-8', errors='ignore')
                if not data:
                    break
                responses.append(data)
                if '[ 450/900' in data or '[450/900' in data:
                    break
            except:
                break

        return ''.join(responses)
    except Exception as e:
        return f'ERROR: {e}'
    finally:
        try:
            s.close()
        except:
            pass

# Test commands
commands = ['능력치', '점수', '무공', '소지품', '누구', '봐']
results = {}
for cmd in commands:
    print(f'Testing {cmd}...')
    output = test_command(cmd)
    results[cmd] = output
    # Save each result
    with open(f'/tmp/rust_{cmd}.txt', 'w') as f:
        f.write(output)

# Summary
print('\n=== SUMMARY ===')
for cmd in commands:
    out = results[cmd]
    if 'ERROR' in out:
        print(f'{cmd}: ERROR')
    elif '오류' in out or 'Function not found' in out:
        print(f'{cmd}: SCRIPT ERROR')
    else:
        print(f'{cmd}: OK')

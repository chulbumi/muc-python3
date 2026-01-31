#!/usr/bin/env python3
import subprocess
import time
import os
import signal
import telnetlib

# Kill any existing server
try:
    subprocess.run(['pkill', '-9', 'murim_server'], check=False)
except:
    pass
time.sleep(2)

# Start server in background
server_log = open('/tmp/murim_debug.log', 'w')
server_proc = subprocess.Popen(
    ['./target/debug/murim_server'],
    stdout=server_log,
    stderr=subprocess.STDOUT,
    cwd='/home/ubuntu/muc-python3'
)
print(f"Server started, PID: {server_proc.pid}")

# Wait for server to start
time.sleep(5)

# Try telnet connection
try:
    tn = telnetlib.Telnet('localhost', 9999, timeout=10)
    time.sleep(1)

    # Read greeting
    output = tn.read_until('무림존함'.encode('utf-8'), timeout=5).decode('utf-8', errors='ignore')
    print("Greeting:", repr(output[-200:]))

    # Send username
    tn.write(b'test\n')
    time.sleep(2)

    # Read password prompt
    output = tn.read_until('assword'.encode('utf-8'), timeout=5).decode('utf-8', errors='ignore')
    print("Password prompt:", repr(output[-200:]))

    # Send password
    tn.write(b'1234\n')
    time.sleep(5)

    # Read more output
    output = tn.read_very_eager().decode('utf-8', errors='ignore')
    print("After password:", repr(output[-500:]))

    # Press enter
    tn.write(b'\n\n')
    time.sleep(4)

    # Read room display
    output = tn.read_very_eager().decode('utf-8', errors='ignore')
    print("Room display:", repr(output[-500:]))

    # Look at room
    tn.write('봐\n'.encode('utf-8'))
    time.sleep(3)

    # Read final output
    output = tn.read_very_eager().decode('utf-8', errors='ignore')
    print("\n=== FINAL OUTPUT ===")
    print(output)

    # Check for mobs
    if '밍밍' in output or '포졸' in output:
        print("\n>>> MOBS FOUND!")
    else:
        print("\n>>> NO MOBS FOUND")

    tn.close()

except Exception as e:
    print(f"Error: {e}")

# Show server log
print("\n=== SERVER LOG ===")
server_log.close()
with open('/tmp/murim_debug.log', 'r') as f:
    log_content = f.read()
    print(log_content[-2000:])  # Last 2000 chars

# Cleanup
server_proc.send_signal(signal.SIGTERM)
time.sleep(1)
try:
    server_proc.kill()
except:
    pass
print("\nTest complete")

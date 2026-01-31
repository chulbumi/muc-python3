#!/usr/bin/env python3
import subprocess
import time
import os
import signal

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

# Now run nc in a subprocess
test_proc = subprocess.Popen(
    ['nc', '-w', '10', 'localhost', '9999'],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.STDOUT,
    cwd='/home/ubuntu/muc-python3'
)

# Send commands
test_proc.stdin.write(b'test\n')
test_proc.stdin.flush()
time.sleep(2)
test_proc.stdin.write(b'1234\n')
test_proc.stdin.flush()
time.sleep(5)
test_proc.stdin.write(b'\n\n')
test_proc.stdin.flush()
time.sleep(4)
test_proc.stdin.write('봐\n'.encode('utf-8'))
test_proc.stdin.flush()
time.sleep(4)

# Close stdin to signal we're done
test_proc.stdin.close()

# Get output
output = test_proc.stdout.read().decode('utf-8', errors='ignore')
print("=== CLIENT OUTPUT ===")
print(output)

# Check for mobs
if '밍밍' in output or '포졸' in output:
    print("\n>>> MOBS FOUND!")
else:
    print("\n>>> NO MOBS FOUND")

# Show server log
print("\n=== SERVER LOG ===")
server_log.close()
with open('/tmp/murim_debug.log', 'r') as f:
    print(f.read())

# Cleanup
server_proc.send_signal(signal.SIGTERM)
time.sleep(1)
try:
    server_proc.kill()
except:
    pass
print("\nTest complete")

#!/usr/bin/env python3
"""Interactive test using telnetlib"""

import telnetlib
import time

print("Connecting to localhost:9999...")
tn = telnetlib.Telnet('localhost', 9999)

# Read until prompt
print("\n=== Reading initial banner ===")
banner = tn.read_until(b'\xc0\xd4', timeout=5)
print(f"Banner ({len(banner)} bytes)")
print(banner.decode('utf-8', errors='replace')[-500:])

# Send username
print("\n=== Sending username '비교테스터' ===")
tn.write("비교테스터\n".encode('utf-8'))
time.sleep(1)

# Read until password prompt
print("Reading response...")
try:
    response = tn.read_until(b'\xc0\xd4', timeout=5)
    print(f"Response ({len(response)} bytes)")
    print(response.decode('utf-8', errors='replace'))
except EOFError:
    print("Connection closed")

# Send password
print("\n=== Sending password 'test1234' ===")
tn.write("test1234\n".encode('utf-8'))
time.sleep(2)

# Read response
print("Reading response...")
try:
    response = tn.read_very_eager()
    print(f"Response ({len(response)} bytes)")
    print(response.decode('utf-8', errors='replace')[:1000])
except EOFError:
    print("Connection closed")

# Try look command
print("\n=== Sending '봐' command ===")
tn.write("봐\n".encode('utf-8'))
time.sleep(1)
try:
    response = tn.read_very_eager()
    print(f"Response ({len(response)} bytes)")
    print(response.decode('utf-8', errors='replace')[:800])
except EOFError:
    print("Connection closed")

tn.close()
print("\nDone!")

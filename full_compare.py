#!/usr/bin/env python3
"""Full comparison test - all commands"""

import socket
import time
import re
import os
from test_mud_comprehensive import TestConfig, ServerType, MUDConnection

def clean_ansi(text):
    ansi_escape = re.compile(r'\x1b\[[0-9;]*[a-zA-Z]|\[0m|\[\d+m|\[\d+;\d+m')
    return ansi_escape.sub('', text)

def recv_all(sock, timeout=0.15):
    sock.settimeout(timeout)
    data = b""
    try:
        while True:
            chunk = sock.recv(4096)
            if not chunk:
                break
            data += chunk
    except socket.timeout:
        pass
    return data.decode('utf-8', errors='ignore')

def connect(port):
    kind = ServerType.PYTHON if port == 9903 else ServerType.RUST
    cfg = TestConfig(host="localhost", py_port=9903, rust_port=9999,
                     command_timeout=2, base_password="test1234")
    conn = MUDConnection(cfg, kind, port, character_name="비교테스터")
    if not conn.connect() or not conn.login_or_create():
        conn.disconnect()
        raise RuntimeError(f"login failed on {port}")
    conn.discard_pending_output()
    return conn

def send_cmd(sock, cmd):
    return clean_ansi(sock.execute_command(cmd, wait_time=0.35)).strip()

def get_commands():
    # Compare player commands only.  cmds/ also contains internal/library and
    # local diagnostic Rhai files which Python never registers as commands.
    python_commands = {
        f[:-3] for f in os.listdir('cmds') if f.endswith('.py')
    }
    return sorted(
        f[:-5] for f in os.listdir('cmds')
        if f.endswith('.rhai') and f[:-5] in python_commands
    )

def normalize(text):
    # Remove HP/MP status
    text = re.sub(r'\[\s*\d+/\d+,\s*\d+/\d+\s*\]', '', text)
    # Normalize whitespace
    text = re.sub(r'\s+', ' ', text)
    return text.strip()

# Get all commands
CMDS = get_commands()
# These Python commands switch the connection into a multi-line input mode.
# A one-line all-command sweep cannot safely continue after them; they are
# covered by their dedicated interactive scenarios instead.
INTERACTIVE_COMMANDS = {
    "몹제작", "방설명", "방제작", "방파방설명", "아이템제작", "암호변경", "쪽지", "체인지",
}
CMDS = [cmd for cmd in CMDS if cmd not in INTERACTIVE_COMMANDS]
print(f"Testing {len(CMDS)} commands...")
print("=" * 60)

# Connect
print("Connecting to Python (9903)...")
py = connect(9903)
print("Connecting to Rust (9999)...")
rs = connect(9999)

matches = 0
diffs = []
tested = 0

for i, cmd in enumerate(CMDS):
    try:
        py_out = normalize(send_cmd(py, cmd))
        rs_out = normalize(send_cmd(rs, cmd))

        tested += 1
        if py_out == rs_out:
            matches += 1
        else:
            diffs.append((cmd, py_out[:80], rs_out[:80]))

        # Progress
        if (i + 1) % 10 == 0:
            print(f"  Tested {i+1}/{len(CMDS)}... ({matches} matched)")
    except Exception as e:
        print(f"  Error testing {cmd}: {e}")

print("\n" + "=" * 60)
print("FINAL RESULTS")
print("=" * 60)
print(f"Commands tested: {tested}")
print(f"Matched: {matches}")
print(f"Different: {len(diffs)}")
print(f"Match rate: {matches/tested*100:.1f}%")

if diffs:
    print(f"\n{len(diffs)} commands with differences:")
    for cmd, py, rs in diffs[:30]:
        print(f"\n  [{cmd}]")
        print(f"    PY: {py}")
        print(f"    RS: {rs}")

# Save report
with open('full_comparison_report.txt', 'w') as f:
    f.write(f"Commands tested: {tested}\n")
    f.write(f"Matched: {matches}\n")
    f.write(f"Different: {len(diffs)}\n")
    f.write(f"Match rate: {matches/tested*100:.1f}%\n\n")
    f.write("Differences:\n")
    for cmd, py, rs in diffs:
        f.write(f"\n[{cmd}]\n")
        f.write(f"Python: {py}\n")
        f.write(f"Rust: {rs}\n")

print("\nReport saved to: full_comparison_report.txt")

py.disconnect()
rs.disconnect()

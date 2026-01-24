import subprocess
import time
import sys

def capture_story_detailed(port, name):
    cmd = f'timeout 15 sh -c \'printf "{name}\\n\\n\\n\\n\\n" | nc localhost {port}\''
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=15)
    return result.stdout, result.stderr, result.returncode

print("=== COMPARISON OF PORTS 9900 AND 9999 ===\n")

# Capture from port 9900
print("1. Capturing from port 9900...")
stdout_9900, stderr_9900, code_9900 = capture_story_detailed(9900, "무명객")
print(f"Return code: {code_9900}")
if stderr_9900:
    print(f"Stderr: {stderr_9900[:200]}...")

print("\n" + "="*50 + "\n")

# Capture from port 9999
print("2. Capturing from port 9999...")
stdout_9999, stderr_9999, code_9999 = capture_story_detailed(9999, "무명객")
print(f"Return code: {code_9999}")
if stderr_9999:
    print(f"Stderr: {stderr_9999[:200]}...")

# Save captures
with open('port_9900_full.log', 'w', encoding='utf-8') as f:
    f.write(stdout_9900)
with open('port_9999_full.log', 'w', encoding='utf-8') as f:
    f.write(stdout_9999)

print("\nCaptures saved to port_9900_full.log and port_9999_full.log")

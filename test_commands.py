#!/usr/bin/env python3
"""Test MUD commands using existing test characters."""

import socket
import time
import select

SERVERS = {
    "python": ("localhost", 9900),
    "rust": ("localhost", 9999)
}

# Use existing characters with known passwords
CHARACTERS = {
    "python": ("test", "1234"),
    "rust": ("테스터러스트", ""),  # Try empty password first
}

COMMANDS = [
    "능력치",
    "무공",
    "소지품",
    "점수",
    "도움말",
    "누구",
    "봐",
    "말 테스트",
    "지도",
    "어디"
]

def recv_all(sock, timeout=2.0):
    """Receive all available data."""
    sock.setblocking(False)
    data = b""
    start = time.time()
    last_data = time.time()
    
    while time.time() - start < timeout:
        ready = select.select([sock], [], [], 0.2)
        if ready[0]:
            try:
                chunk = sock.recv(8192)
                if chunk:
                    data += chunk
                    last_data = time.time()
                    start = time.time()
                else:
                    break
            except BlockingIOError:
                pass
        elif time.time() - last_data > 0.5:
            break
    
    return data

def login(server_name, host, port, username, password):
    """Login with existing character."""
    print(f"  Logging in as '{username}' (password: '{password}')...")
    
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect((host, port))
    
    time.sleep(0.5)
    banner = recv_all(sock, timeout=1.0)
    
    # Send username
    sock.sendall(username.encode('utf-8') + b"\r\n")
    time.sleep(0.8)
    
    response = recv_all(sock, timeout=1.0).decode('utf-8', errors='ignore')
    
    # Send password if prompted
    if '암호' in response:
        sock.sendall(password.encode('utf-8') + b"\r\n")
        time.sleep(0.8)
        response2 = recv_all(sock, timeout=1.0).decode('utf-8', errors='ignore')
        
        if '잘못된' in response2 or 'incorrect' in response2.lower():
            print(f"    Wrong password!")
            sock.close()
            return None
    
    # Read post-login
    post_login = recv_all(sock, timeout=1.0)
    print(f"    Logged in! ({len(post_login)}b)")
    
    return sock

def test_server(server_name, host, port, username, password):
    """Test all commands on a server."""
    print(f"\n{'='*60}")
    print(f"{server_name.upper()} ({host}:{port})")
    print('='*60)

    results = {}

    try:
        sock = login(server_name, host, port, username, password)
        
        if sock is None:
            results["error"] = "Login failed"
            return results

        print(f"\n  Testing commands:")
        
        for cmd in COMMANDS:
            print(f"    {cmd}...", end=" ", flush=True)
            cmd_bytes = cmd.encode('utf-8') + b"\r\n"
            
            try:
                sock.sendall(cmd_bytes)
            except:
                print("DISCONNECTED")
                results[cmd] = ""
                break
            
            time.sleep(1.0)
            output = recv_all(sock, timeout=2.0)
            output_str = output.decode('utf-8', errors='ignore')
            results[cmd] = output_str
            print(f"{len(output)}b")

        try:
            sock.close()
        except:
            pass

    except Exception as e:
        print(f"\n  Error: {e}")
        import traceback
        traceback.print_exc()
        results["error"] = str(e)

    return results

def strip_ansi(text):
    """Remove ANSI escape codes."""
    import re
    ansi_escape = re.compile(r'\x1b\[[0-9;]*m')
    return ansi_escape.sub('', text)

def save_results(all_results):
    """Save results to markdown file."""
    with open("/home/ubuntu/muc-python3/test_results.md", "w", encoding="utf-8") as f:
        f.write("# MUD Server Command Test Results\n\n")
        f.write(f"Test Date: {time.strftime('%Y-%m-%d %H:%M:%S')}\n\n")
        f.write("## Servers\n\n")
        f.write("- Python Server: localhost:9900 (user: test)\n")
        f.write("- Rust Server: localhost:9999 (user: 테스터러스트)\n\n")

        for cmd in COMMANDS:
            f.write(f"\n{'='*60}\n\n")
            f.write(f"## Command: {cmd}\n\n")

            python_output = all_results.get("python", {}).get(cmd, "")
            rust_output = all_results.get("rust", {}).get(cmd, "")

            f.write("### Python Server (9900)\n\n")
            clean_python = strip_ansi(python_output)
            if clean_python.strip():
                f.write("```\n" + clean_python + "\n```\n\n")
            else:
                f.write("*No output*\n\n")

            f.write("### Rust Server (9999)\n\n")
            clean_rust = strip_ansi(rust_output)
            if clean_rust.strip():
                f.write("```\n" + clean_rust + "\n```\n\n")
            else:
                f.write("*No output*\n\n")

            f.write("### Comparison\n\n")
            if clean_python.strip() and clean_rust.strip():
                if clean_python == clean_rust:
                    f.write("* **IDENTICAL**\n\n")
                else:
                    f.write("* **DIFFERENT**\n\n")
                    p_lines = [l for l in clean_python.split('\n') if l.strip()]
                    r_lines = [l for l in clean_rust.split('\n') if l.strip()]
                    f.write("**Python output:**\n```\n")
                    for line in p_lines[:30]:
                        f.write(line + "\n")
                    f.write("```\n\n**Rust output:**\n```\n")
                    for line in r_lines[:30]:
                        f.write(line + "\n")
                    f.write("```\n\n")
            elif clean_python.strip():
                f.write("* **Python only**\n\n")
            elif clean_rust.strip():
                f.write("* **Rust only**\n\n")
            else:
                f.write("* **Neither**\n\n")

        # Summary
        f.write("\n" + "="*60 + "\n\n")
        f.write("## Summary\n\n")
        python_results = all_results.get("python", {})
        rust_results = all_results.get("rust", {})

        working_python = sum(1 for cmd in COMMANDS if python_results.get(cmd) and strip_ansi(python_results.get(cmd, "")).strip())
        working_rust = sum(1 for cmd in COMMANDS if rust_results.get(cmd) and strip_ansi(rust_results.get(cmd, "")).strip())

        f.write(f"| Server | Commands Working | Total |\n")
        f.write(f"|--------|------------------|-------|\n")
        f.write(f"| Python | {working_python} | {len(COMMANDS)} |\n")
        f.write(f"| Rust | {working_rust} | {len(COMMANDS)} |\n\n")
        
        f.write("## Command Status\n\n")
        f.write("| Command | Python | Rust |\n")
        f.write("|---------|--------|------|\n")
        for cmd in COMMANDS:
            p_ok = "OK" if python_results.get(cmd) and strip_ansi(python_results.get(cmd, "")).strip() else "--"
            r_ok = "OK" if rust_results.get(cmd) and strip_ansi(rust_results.get(cmd, "")).strip() else "--"
            f.write(f"| {cmd} | {p_ok} | {r_ok} |\n")

def main():
    all_results = {}

    # Test Python server
    username, password = CHARACTERS["python"]
    all_results["python"] = test_server("python", "localhost", 9900, username, password)
    
    # Test Rust server  
    username, password = CHARACTERS["rust"]
    all_results["rust"] = test_server("rust", "localhost", 9999, username, password)

    save_results(all_results)

    # Print results
    print(f"\n{'='*60}")
    print("RESULTS")
    print('='*60)

    for cmd in COMMANDS:
        python_output = all_results.get("python", {}).get(cmd, "")
        rust_output = all_results.get("rust", {}).get(cmd, "")

        clean_python = strip_ansi(python_output)
        clean_rust = strip_ansi(rust_output)

        print(f"\n[{cmd}]")
        print(f"  Python: {len(clean_python)}c - {'OK' if clean_python.strip() else '--'}")
        print(f"  Rust:   {len(clean_rust)}c - {'OK' if clean_rust.strip() else '--'}")

    return all_results

if __name__ == "__main__":
    main()

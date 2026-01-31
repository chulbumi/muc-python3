#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Test script to compare command outputs between Python (9900) and Rust (9999) servers
"""

import socket
import time
import sys
import subprocess

def create_char_if_not_exists(name):
    """Create character using telnet"""
    try:
        import telnetlib
        tn = telnetlib.Telnet("localhost", 9999, timeout=10)
        time.sleep(1)
        tn.read_very_eager()

        tn.write(name.encode('euc-kr') + b"\n")
        time.sleep(0.5)
        output = tn.read_very_eager().decode('euc-kr', errors='ignore')

        if "비번" in output or "assword" in output or "암호" in output or "new" in output.lower():
            tn.write(b"1234\n")  # password
            time.sleep(0.5)
            tn.read_very_eager()
            tn.write(b"1234\n")  # confirm
            time.sleep(0.5)

        tn.close()
    except:
        pass


def test_server_socket(host, port, name, commands):
    """Connect to server using raw socket, login, and run commands"""
    try:
        print(f"  Connecting to {host}:{port}...", end=" ", flush=True)
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(10)
        sock.connect((host, port))
        print("Connected!")

        time.sleep(0.5)

        def send(text):
            sock.sendall((text + "\n").encode('euc-kr'))

        def recv():
            try:
                data = b""
                sock.setblocking(False)
                while True:
                    try:
                        chunk = sock.recv(4096)
                        if not chunk:
                            break
                        data += chunk
                    except BlockingIOError:
                        break
                return data.decode('euc-kr', errors='ignore')
            except:
                return ""

        # Initial read
        time.sleep(0.5)
        output = recv()
        print(f"  Initial output: {repr(output[:100])}")

        # Send name
        send(name)
        time.sleep(0.5)
        output = recv()
        print(f"  After name: {repr(output[:100])}")

        # Send password if needed
        if "비번" in output or "assword" in output or "암호" in output:
            send("1234")
            time.sleep(0.5)
            output = recv()
            print(f"  After password: {repr(output[:100])}")

        # Clear buffer
        send("")
        time.sleep(0.3)
        recv()

        results = {}
        for cmd in commands:
            print(f"  Sending: {cmd}")
            send(cmd)
            time.sleep(1.0)

            # Read multiple times to get all data
            data = ""
            for _ in range(5):
                time.sleep(0.2)
                chunk = recv()
                if chunk:
                    data += chunk

            results[cmd] = data
            print(f"    Output: {repr(data[:200])}")

            send("")
            time.sleep(0.3)
            recv()

        sock.close()
        return results

    except Exception as e:
        print(f"  Error: {e}")
        import traceback
        traceback.print_exc()
        return {}


def main():
    # Ensure character exists on both servers
    print("Creating characters...")
    create_char_if_not_exists("테스터")

    commands = ["점수", "능력치", "무공"]

    print("\n" + "=" * 80)
    print("Testing PYTHON Server (port 9900)")
    print("=" * 80)
    python_results = test_server_socket("localhost", 9900, "테스터", commands)

    print("\n" + "=" * 80)
    print("Testing RUST Server (port 9999)")
    print("=" * 80)
    rust_results = test_server_socket("localhost", 9999, "테스터", commands)

    # Compare results
    for cmd in commands:
        print("\n" + "=" * 80)
        print(f"COMMAND: {cmd}")
        print("=" * 80)

        python_output = python_results.get(cmd, "NO OUTPUT")
        rust_output = rust_results.get(cmd, "NO OUTPUT")

        print("\n--- PYTHON (9900) OUTPUT ---")
        print(python_output)
        print("\n--- RUST (9999) OUTPUT ---")
        print(rust_output)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Test Rust MUD server commands using telnet"""
import telnetlib
import time
import sys

HOST = "localhost"
PORT = 9999
USERNAME = "테스터러스트"
PASSWORD = "1234"

def test_commands():
    try:
        # Connect to the MUD server
        print("=" * 60)
        print("Connecting to MUD server at {}:{}".format(HOST, PORT))
        print("=" * 60)

        tn = telnetlib.Telnet(HOST, PORT, timeout=10)

        # Wait for login prompt
        output = tn.read_until(b"ID:", timeout=5).decode('utf-8', errors='ignore')
        print("\n[Initial output]")
        print(output)

        # Send username
        tn.write(USERNAME.encode('utf-8') + b"\r\n")

        # Wait for password prompt
        output = tn.read_until(b"Password:", timeout=5).decode('utf-8', errors='ignore')
        print("\n[After username]")
        print(output)

        # Send password
        tn.write(PASSWORD.encode('utf-8') + b"\r\n")

        # Wait for login to complete
        time.sleep(1)

        # Commands to test
        commands = ["능력치", "점수", "무공", "소지품", "누구", "봐"]

        for cmd in commands:
            print("\n" + "=" * 60)
            print("Testing command: {}".format(cmd))
            print("=" * 60)

            tn.write(cmd.encode('utf-8') + b"\r\n")
            time.sleep(1)

            # Read response
            output = tn.read_very_eager().decode('utf-8', errors='ignore')
            print(output)

        # Disconnect
        print("\n" + "=" * 60)
        print("Disconnecting...")
        print("=" * 60)
        tn.write("접속종료".encode('utf-8') + b"\r\n")
        time.sleep(0.5)
        tn.close()

    except Exception as e:
        print("Error: {}".format(e))
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    test_commands()

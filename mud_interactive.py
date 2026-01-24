#!/usr/bin/env python3
import telnetlib
import time
import sys

def interactive_mud_session():
    """Interactive MUD session to document character creation"""
    host = 'localhost'
    port = 9900

    print(f"Connecting to {host}:{port}...")
    try:
        tn = telnetlib.Telnet(host, port, timeout=30)
        print("Connected!\n")

        # Receive initial screen
        print("=" * 80)
        print("INITIAL SCREEN")
        print("=" * 80)
        data = tn.read_until(b":", timeout=5).decode('utf-8', errors='replace')
        print(data)
        print("=" * 80)
        print()

        # Send name
        name = "테스트"
        print(f"SENDING NAME: {name}")
        tn.write(name.encode('utf-8') + b"\r\n")
        time.sleep(1)

        # Read response
        print("\n" + "=" * 80)
        print("RESPONSE AFTER NAME")
        print("=" * 80)

        # Try to read data with multiple attempts
        all_data = b""
        for i in range(10):
            try:
                chunk = tn.read_very_eager()
                if chunk:
                    all_data += chunk
                    time.sleep(0.5)
                else:
                    break
            except:
                break

        if all_data:
            print(all_data.decode('utf-8', errors='replace'))
        else:
            print("(No data received)")

        print("=" * 80)
        print()

        # Check character status
        decoded = all_data.decode('utf-8', errors='replace')

        if "존재" in decoded or "있습니다" in decoded:
            print("\n*** CHARACTER ALREADY EXISTS ***")
            tn.close()
            return

        # If we need password, send it
        if "암호" in decoded or "비밀" in decoded:
            password = "test1234"
            print(f"SENDING PASSWORD: {password}")
            tn.write(password.encode('utf-8') + b"\r\n")
            time.sleep(1)

            # Read response
            data = tn.read_very_eager()
            if data:
                print("\n" + "=" * 80)
                print("RESPONSE AFTER PASSWORD")
                print("=" * 80)
                print(data.decode('utf-8', errors='replace'))
                print("=" * 80)

                # Confirm password if needed
                if "암호" in data.decode('utf-8', errors='replace'):
                    print(f"\nCONFIRMING PASSWORD: {password}")
                    tn.write(password.encode('utf-8') + b"\r\n")
                    time.sleep(1)

        # Continue monitoring for more prompts
        print("\n" + "=" * 80)
        print("MONITORING FOR MORE PROMPTS (10 seconds)...")
        print("=" * 80)

        for i in range(10):
            time.sleep(1)
            try:
                data = tn.read_very_eager()
                if data:
                    print(f"\n--- DATA RECEIVED AT T+{i+1} ---")
                    print(data.decode('utf-8', errors='replace'))
                    print("--- END DATA ---\n")
            except:
                pass

        tn.close()
        print("\nConnection closed.")

    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    interactive_mud_session()

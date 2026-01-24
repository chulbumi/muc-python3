#!/usr/bin/env python3
import socket
import time
import sys

def connect_to_mud(host='localhost', port=9900):
    """Connect to MUD server and document character creation"""
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(60.0)

    try:
        print("Connecting to {}:{}...".format(host, port))
        sock.connect((host, port))
        print("Connected successfully!\n")

        # Receive initial greeting
        data = receive_until_prompt(sock, timeout=5)
        print("\n=== INITIAL SCREEN ===")
        print(data)
        print("\n=== SENDING NAME: 테스트 ===\n")

        # Send name
        send_with_delay(sock, "테스트\n")
        time.sleep(1)

        # Receive response - wait longer for server response
        time.sleep(1)
        data = receive_data(sock, timeout=5)
        print("\n=== RESPONSE AFTER NAME ===")
        print(repr(data))  # Use repr to see exact content
        print("\n")
        print(data)

        # Check if character exists or needs creation
        if "존재" in data or "있습니다" in data:
            print("\n=== CHARACTER ALREADY EXISTS ===")
            return

        # If we need to create character, continue with prompts
        if "암호" in data or "비밀" in data or "password" in data.lower():
            print("\n=== SENDING PASSWORD: test1234 ===\n")
            send_with_delay(sock, "test1234\n")
            time.sleep(1)
            data = receive_data(sock, timeout=3)
            print("\n=== RESPONSE AFTER PASSWORD ===")
            print(data)

            # Confirm password
            if "암호" in data or "비밀" in data:
                print("\n=== CONFIRMING PASSWORD: test1234 ===\n")
                send_with_delay(sock, "test1234\n")
                time.sleep(1)
                data = receive_data(sock, timeout=3)
                print("\n=== RESPONSE AFTER PASSWORD CONFIRMATION ===")
                print(data)

        # Continue with other prompts
        # This is a loop to handle all creation prompts
        for i in range(10):
            time.sleep(1)
            data = receive_data(sock, timeout=2)

            if not data:
                break

            print(f"\n=== STEP {i+1} ===")
            print(data)

            # Determine appropriate response based on prompt
            response = get_response_for_prompt(data)
            if response:
                print(f"\n=== SENDING: {response} ===\n")
                send_with_delay(sock, response)
            else:
                print("\n=== NO RESPONSE - CHECKING FOR MORE DATA ===\n")

            # If we see the main game prompt, we're done
            if ">" in data and "명령" in data:
                print("\n=== CHARACTER CREATION COMPLETE - ENTERED GAME ===\n")
                break

    except socket.timeout:
        print("\n=== CONNECTION TIMED OUT ===")
    except Exception as e:
        print(f"\n=== ERROR: {e} ===")
        import traceback
        traceback.print_exc()
    finally:
        sock.close()
        print("\n=== CONNECTION CLOSED ===")

def receive_until_prompt(sock, timeout=5):
    """Receive data until we see a prompt or timeout"""
    sock.settimeout(timeout)
    data = b""
    start_time = time.time()

    while time.time() - start_time < timeout:
        try:
            chunk = sock.recv(4096)
            if chunk:
                data += chunk
                # Check for common prompt indicators
                if b":" in chunk:
                    break
        except socket.timeout:
            break

    return decode_ansi(data)

def receive_data(sock, timeout=3):
    """Receive any available data"""
    sock.settimeout(timeout)
    data = b""
    start_time = time.time()

    while time.time() - start_time < timeout:
        try:
            chunk = sock.recv(4096)
            if chunk:
                data += chunk
                time.sleep(0.2)  # Small delay to get more data
            else:
                break
        except socket.timeout:
            break

    return decode_ansi(data)

def send_with_delay(sock, message):
    """Send message with small delay"""
    sock.sendall(message.encode('utf-8'))
    time.sleep(0.3)

def decode_ansi(data):
    """Decode bytes with ANSI codes preserved"""
    try:
        return data.decode('utf-8', errors='replace')
    except:
        return str(data)

def get_response_for_prompt(data):
    """Determine appropriate response based on the prompt"""
    data_lower = data.lower()

    # Gender selection
    if "성별" in data or "남자" in data:
        return "1\n"  # Select male

    # Class/profession selection
    if "직업" in data or "무사" in data:
        return "1\n"  # Select warrior

    # Confirm creation
    if "확인" in data or "맞" in data or "yes" in data_lower:
        return "y\n"

    # Any number prompt - select first option
    if "선택" in data or "번호" in data:
        return "1\n"

    # Default - no response
    return None

if __name__ == "__main__":
    connect_to_mud()

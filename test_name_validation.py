#!/usr/bin/env python3
"""
Test script to compare name validation between Python (port 9900) and Rust (port 9999) servers
"""

import socket
import time
import sys

def test_name_validation(server_port, server_name):
    """Test name validation against a server"""
    test_names = [
        "test123",  # with numbers
        "john",     # English
        "김철수",    # valid Korean name
        "",         # empty
        "손님",     # special guest name
        "무명객",   # special anonymous name
        "한123",    # mixed Korean and numbers
        "abc한글",  # mixed English and Korean
    ]

    results = {}

    print(f"\n=== Testing {server_name} (port {server_port}) ===")

    for name in test_names:
        try:
            # Connect to server
            s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            s.settimeout(5)
            s.connect(('localhost', server_port))

            # Read welcome message
            welcome = s.recv(1024).decode('utf-8', errors='ignore')

            # Send name
            s.send((name + '\n').encode('utf-8'))

            # Read response
            response = s.recv(1024).decode('utf-8', errors='ignore')

            # Close connection
            s.close()

            results[name] = {
                'success': '암호 :' in response or '이름 :' in response or '비번 :' in response,
                'response': response.strip(),
                'full_response': response
            }

            print(f"Name: '{name}'")
            print(f"Response: {response.strip()}")
            print("-" * 50)

        except Exception as e:
            results[name] = {
                'success': False,
                'error': str(e)
            }
            print(f"Name: '{name}' - Error: {e}")
            print("-" * 50)

        time.sleep(0.5)  # Small delay between tests

    return results

def main():
    test_names = [
        "test123",  # with numbers
        "john",     # English
        "김철수",    # valid Korean name
        "",         # empty
        "손님",     # special guest name
        "무명객",   # special anonymous name
        "한123",    # mixed Korean and numbers
        "abc한글",  # mixed English and Korean
    ]

    # Test Python server (port 9900)
    python_results = test_name_validation(9900, "Python Server")

    # Test Rust server (port 9999)
    rust_results = test_name_validation(9999, "Rust Server")

    # Create comparison table
    print("\n" + "="*80)
    print("COMPARISON TABLE")
    print("="*80)
    print(f"{'Name':<15} {'Python':<40} {'Rust':<40}")
    print("-"*95)

    for name in test_names:
        python_resp = python_results.get(name, {})
        rust_resp = rust_results.get(name, {})

        python_result = "Success" if python_resp.get('success', False) else "Failed/Error"
        rust_result = "Success" if rust_resp.get('success', False) else "Failed/Error"

        print(f"{str(name):<15} {python_result:<40} {rust_result:<40}")

    # Print detailed error messages
    print("\n" + "="*80)
    print("DETAILED ERROR MESSAGES")
    print("="*80)

    for name in test_names:
        python_resp = python_results.get(name, {})
        rust_resp = rust_results.get(name, {})

        print(f"\nName: '{name}'")
        print("-"*50)

        if python_resp.get('error'):
            print(f"Python Error: {python_resp['error']}")
        elif python_resp.get('response'):
            print(f"Python Response: {python_resp['response']}")

        if rust_resp.get('error'):
            print(f"Rust Error: {rust_resp['error']}")
        elif rust_resp.get('response'):
            print(f"Rust Response: {rust_resp['response']}")

if __name__ == "__main__":
    main()
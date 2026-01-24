#!/usr/bin/env python3
"""
Detailed test script to compare name validation between Python (port 9900) and Rust (port 9999) servers
"""

import socket
import time
import sys
import os

# Add the project root to Python path to import lib modules
sys.path.insert(0, '/home/ubuntu/muc-python3')

from lib.hangul import is_han

def test_server_interaction(server_port, server_name):
    """Test server interaction step by step"""
    test_cases = [
        {
            'name': 'test123',
            'description': 'Name with numbers'
        },
        {
            'name': 'john',
            'description': 'English name'
        },
        {
            'name': '김철수',
            'description': 'Valid Korean name'
        },
        {
            'name': '',
            'description': 'Empty name'
        },
        {
            'name': '손님',
            'description': 'Special guest name'
        },
        {
            'name': '무명객',
            'description': 'Special anonymous name'
        },
        {
            'name': '한123',
            'description': 'Mixed Korean and numbers'
        },
        {
            'name': 'abc한글',
            'description': 'Mixed English and Korean'
        }
    ]

    print(f"\n=== Testing {server_name} (port {server_port}) ===")

    for i, test_case in enumerate(test_cases):
        print(f"\n--- Test {i+1}: {test_case['name']} ({test_case['description']}) ---")

        try:
            # Connect to server
            s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            s.settimeout(10)
            s.connect(('localhost', server_port))

            # Read initial welcome message
            welcome = s.recv(4096).decode('utf-8', errors='ignore')
            print("Initial message received")

            # Send the test name
            print(f"Sending name: '{test_case['name']}'")
            s.send((test_case['name'] + '\n').encode('utf-8'))

            # Read response
            time.sleep(0.5)  # Give server time to process
            response = s.recv(4096).decode('utf-8', errors='ignore')

            # Check what happened
            if '한글 입력만 가능합니다' in response:
                print("Result: REJECTED - Not Korean")
                print(f"Response: {response.strip()}")
            elif '이미 사용중인 이름입니다' in response:
                print("Result: REJECTED - Name already exists")
                print(f"Response: {response.strip()}")
            elif '사용할 수 없는 이름입니다' in response:
                print("Result: REJECTED - Reserved name")
                print(f"Response: {response.strip()}")
            elif '한글자 이상 입력하세요' in response:
                print("Result: REJECTED - Too short")
                print(f"Response: {response.strip()}")
            elif '암호 :' in response or '비번 :' in response:
                print("Result: ACCEPTED - Asking for password")
                print(f"Response: {response.strip()}")
            elif '무림존함ː' in response:
                print("Result: REJECTED - Asking for name again")
                print(f"Response: {response.strip()}")
            elif len(response.strip()) == 0:
                print("Result: No response (connection closed?)")
            else:
                print("Result: UNKNOWN")
                print(f"Response: {response}")

            # Close connection
            s.close()

        except Exception as e:
            print(f"Error: {e}")

        time.sleep(1)  # Delay between tests

def main():
    # Test the is_han function directly
    print("=== Testing is_han function (Python implementation) ===")
    test_names = ["test123", "john", "김철수", "", "손님", "무명객", "한123", "abc한글"]

    print(f"{'Name':<15} {'is_han()':<10} {'Description'}")
    print("-"*70)

    for name in test_names:
        result = is_han(name)
        print(f"{str(name):<15} {result:<10} {'Korean only' if result else 'Not Korean'}")

    # Test both servers
    print("\n" + "="*80)
    print("SERVER INTERACTION TESTS")
    print("="*80)

    # Test Python server
    if os.path.exists('/home/ubuntu/muc-python3/server.py'):
        test_server_interaction(9900, "Python Server")
    else:
        print("Python server not found")

    # Test Rust server
    test_server_interaction(9999, "Rust Server")

if __name__ == "__main__":
    main()
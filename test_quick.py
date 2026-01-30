#!/usr/bin/env python3
import socket
import time

def quick_test(port):
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(2)
    try:
        sock.connect(('localhost', port))
        # Get initial with non-blocking
        sock.setblocking(False)
        data = b""
        start = time.time()
        while time.time() - start < 1:
            try:
                chunk = sock.recv(4096)
                if chunk:
                    data += chunk
                else:
                    break
            except:
                time.sleep(0.1)
                if data:
                    break
        
        text = data.decode('utf-8', errors='ignore')
        # Simple strip
        lines = []
        for line in text.split('\r\n'):
            if line and not line.startswith('\x1b'):
                lines.append(line)
        
        print(f"\nPort {port}:")
        if lines:
            print("  ".join(lines[:5]))
        else:
            print("  (banner displayed)")
        return True
    except Exception as e:
        print(f"\nPort {port}: Error - {e}")
        return False
    finally:
        sock.close()

print("Quick server test:")
quick_test(9900)
quick_test(9990)

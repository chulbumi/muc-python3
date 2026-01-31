import socket
import time
import sys

def recv_all(s, timeout=0.3):
    s.setblocking(False)
    data = b""
    start = time.time()
    while time.time() - start < timeout:
        try:
            chunk = s.recv(4096)
            if not chunk:
                break
            data += chunk
        except BlockingIOError:
            time.sleep(0.05)
    return data

def test_score():
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.connect(('localhost', 9999))
    s.setblocking(False)
    
    time.sleep(0.3)
    initial = recv_all(s, 0.5)
    print("INITIAL:", len(initial), "bytes")
    
    name = f"test{int(time.time() * 1000) % 10000}"
    password = "9999"
    gender = "남"
    
    try:
        # 빠른도우미
        s.sendall("빠른도우미\r\n".encode('utf-8'))
        time.sleep(0.2)
        recv_all(s, 0.2)
        
        # Name
        s.sendall(f"{name}\r\n".encode('utf-8'))
        time.sleep(0.2)
        resp = recv_all(s, 0.2)
        print(f"After name: {len(resp)} bytes")
        
        # Password
        s.sendall(f"{password}\r\n".encode('utf-8'))
        time.sleep(0.2)
        recv_all(s, 0.2)
        
        # Gender
        s.sendall(f"{gender}\r\n".encode('utf-8'))
        time.sleep(0.5)
        resp = recv_all(s, 1.0)
        print(f"After gender: {len(resp)} bytes")
        print(resp.decode('utf-8', errors='replace')[-500:])
        
        # Score
        s.sendall("능력치\r\n".encode('utf-8'))
        time.sleep(0.5)
        resp = recv_all(s, 1.0)
        print(f"\n=== SCORE OUTPUT ({len(resp)} bytes) ===")
        output = resp.decode('utf-8', errors='replace')
        print(output)
        
        if "오류" in output or "Syntax error" in output:
            print("\n*** ERROR: Syntax error! ***")
            return 1
        elif "체력" in output or "힘" in output:
            print("\n*** SUCCESS: Score works! ***")
            return 0
        else:
            print("\n*** UNKNOWN ***")
            return 2
            
    except Exception as e:
        print(f"Exception: {e}")
        return 3
    finally:
        try:
            s.close()
        except:
            pass

sys.exit(test_score())

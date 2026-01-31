import socket
import time

def login_and_test(port, name, password):
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.connect(('localhost', port))
    time.sleep(0.4)
    
    # Clear initial
    s.setblocking(False)
    for _ in range(10):
        try:
            s.recv(4096)
        except:
            time.sleep(0.05)
            break
    
    # Name
    s.sendall(f"{name}\r\n".encode('utf-8'))
    time.sleep(0.3)
    for _ in range(10):
        try:
            s.recv(4096)
        except:
            break
    
    # Password
    s.sendall(f"{password}\r\n".encode('utf-8'))
    time.sleep(0.5)
    
    data = b""
    for _ in range(20):
        try:
            chunk = s.recv(4096)
            if not chunk:
                break
            data += chunk
        except:
            time.sleep(0.05)
    
    return s, data.decode('utf-8', errors='replace')

# First try to create a new account via 나만바라바
def create_account(port, name, password):
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.connect(('localhost', port))
    time.sleep(0.4)
    
    # Clear initial
    s.setblocking(False)
    for _ in range(10):
        try:
            s.recv(4096)
        except:
            time.sleep(0.05)
            break
    
    # Trigger 빠른도우미
    s.sendall("나만바라바\r\n".encode('utf-8'))
    time.sleep(0.5)
    
    # Clear response
    for _ in range(10):
        try:
            s.recv(4096)
        except:
            break
    
    # Name
    s.sendall(f"{name}\r\n".encode('utf-8'))
    time.sleep(0.3)
    for _ in range(10):
        try:
            s.recv(4096)
        except:
            break
    
    # Password
    s.sendall(f"{password}\r\n".encode('utf-8'))
    time.sleep(0.3)
    for _ in range(10):
        try:
            s.recv(4096)
        except:
            break
    
    # Gender
    s.sendall("남\r\n".encode('utf-8'))
    time.sleep(0.3)
    for _ in range(10):
        try:
            s.recv(4096)
        except:
            break
    
    # Enter
    s.sendall("\r\n".encode('utf-8'))
    time.sleep(0.5)
    
    data = b""
    for _ in range(20):
        try:
            chunk = s.recv(4096)
            if not chunk:
                break
            data += chunk
        except:
            time.sleep(0.05)
    
    output = data.decode('utf-8', errors='replace')
    s.close()
    return output

# Create accounts on both servers
name = "비교테"
password = "9999"

print(f"Creating account on Python (9900)...")
py_create = create_account(9900, name, password)
if "입장" in py_create or "완료" in py_create:
    print(f"✓ Python: Account created")
elif "이미" in py_create:
    print(f"✓ Python: Account already exists")
else:
    print(f"✗ Python: {py_create[-100:]}")

print(f"\nCreating account on Rust (9999)...")
rust_create = create_account(9999, name, password)
if "입장" in rust_create or "완료" in rust_create:
    print(f"✓ Rust: Account created")
elif "이미" in rust_create:
    print(f"✓ Rust: Account already exists")
else:
    print(f"✗ Rust: {rust_create[-100:]}")

print(f"\n{'='*60}")
print("Testing SCORE command")
print('='*60)

s_py, py_output = login_and_test(9900, name, password)
s_py.sendall("점수\r\n".encode('utf-8'))
time.sleep(0.5)
data = b""
for _ in range(20):
    try:
        chunk = s_py.recv(4096)
        if not chunk:
            break
        data += chunk
    except:
        time.sleep(0.05)
py_score = data.decode('utf-8', errors='replace')
s_py.close()

s_rust, rust_output = login_and_test(9999, name, password)
s_rust.sendall("능력치\r\n".encode('utf-8'))
time.sleep(0.5)
data = b""
for _ in range(20):
    try:
        chunk = s_rust.recv(4096)
        if not chunk:
            break
        data += chunk
    except:
        time.sleep(0.05)
rust_score = data.decode('utf-8', errors='replace')
s_rust.close()

print("\nPython (9900) Score:")
print(py_score)
print("\nRust (9999) Score:")
print(rust_score)

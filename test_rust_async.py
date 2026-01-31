import asyncio
import sys
import traceback

async def test_commands():
    try:
        reader, writer = await asyncio.wait_for(
            asyncio.open_connection("127.0.0.1", 9999), 
            timeout=5.0
        )
        
        # Read greeting
        data = await asyncio.wait_for(reader.read(4096), timeout=3.0)
        print(f"Greeting: {len(data)} bytes", flush=True)
        
        # Send name
        writer.write("테스터러스트\n".encode('utf-8'))
        await writer.drain()
        
        # Read password prompt  
        data = await asyncio.wait_for(reader.read(4096), timeout=2.0)
        print(f"After name: {len(data)} bytes", flush=True)
        
        # Send password
        writer.write("1234\n".encode('utf-8'))
        await writer.drain()
        
        # Read all welcome data
        all_data = b""
        for _ in range(10):
            try:
                chunk = await asyncio.wait_for(reader.read(4096), timeout=0.5)
                if not chunk:
                    break
                all_data += chunk
                if b"[ 450/900" in chunk or b"[450/900" in chunk:
                    break
            except asyncio.TimeoutError:
                break
        print(f"After password: {len(all_data)} bytes", flush=True)
        
        # Test commands
        commands = ['능력치', '점수', '무공', '소지품', '누구', '봐']
        for cmd in commands:
            print(f"\n===== {cmd} =====", flush=True)
            try:
                writer.write((cmd + "\n").encode('utf-8'))
                await writer.drain()
                
                response = b""
                for _ in range(10):
                    try:
                        chunk = await asyncio.wait_for(reader.read(4096), timeout=0.3)
                        if not chunk:
                            break
                        response += chunk
                        if b"[ 450/900" in chunk or b"[450/900" in chunk:
                            break
                    except asyncio.TimeoutError:
                        break
                
                output = response.decode('utf-8', errors='ignore')
                if 'ERROR' in output or '오류' in output:
                    print("SCRIPT ERROR!")
                    print(output[:300])
                elif output.strip():
                    print("OK")
                    print(output[:400])
                else:
                    print("NO OUTPUT")
            except Exception as e:
                print(f"ERROR: {e}")
                traceback.print_exc()
        
        writer.close()
        await writer.wait_closed()
        
    except Exception as e:
        print(f"CONNECTION ERROR: {e}")
        traceback.print_exc()

asyncio.run(test_commands())

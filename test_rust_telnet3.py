import asyncio
import sys

async def test_commands():
    try:
        reader, writer = await asyncio.open_connection("127.0.0.1", 9999)
        
        # Read greeting
        data = await reader.read(4096)
        print(f"Greeting received: {len(data)} bytes")
        
        # Send name
        writer.write(b"\xff\xfd\x18\xff\xfd\x20\xff\xfd\x23\xff\xfd\x27")  # telnet opts
        writer.write("테스터러스트\n".encode('utf-8'))
        await writer.drain()
        
        # Read password prompt
        data = await reader.read(4096)
        print(f"After name: {len(data)} bytes")
        
        # Send password
        writer.write("1234\n".encode('utf-8'))
        await writer.drain()
        
        # Read welcome
        data = await asyncio.wait_for(reader.read(8192), timeout=3.0)
        print(f"After password: {len(data)} bytes")
        
        # Test commands
        commands = ['능력치', '점수', '무공', '소지품', '누구', '봐']
        for cmd in commands:
            writer.write((cmd + "\n").encode('utf-8'))
            await writer.drain()
            
            try:
                response = await asyncio.wait_for(reader.read(8192), timeout=2.0)
                output = response.decode('utf-8', errors='ignore')
                print(f"\n===== {cmd} =====")
                print(output[:500])
            except:
                print(f"\n===== {cmd} =====")
                print("TIMEOUT or ERROR")
        
        writer.close()
        await writer.wait_closed()
    except Exception as e:
        print(f"ERROR: {e}")

asyncio.run(test_commands())

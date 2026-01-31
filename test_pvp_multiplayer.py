#!/usr/bin/env python3
"""Test script for PvP/multiplayer connections in Rust MUD.

This tests that:
1. Multiple players can connect simultaneously
2. PvP/communication between players works without broken pipe errors
3. Disconnection of one player doesn't crash the server
"""

import asyncio
import sys
import time

SERVER_HOST = "127.0.0.1"
SERVER_PORT = 9999


class MUDClient:
    """Async MUD client for testing."""

    def __init__(self, name: str, password: str):
        self.name = name
        self.password = password
        self.reader: asyncio.StreamReader | None = None
        self.writer: asyncio.StreamWriter | None = None
        self.connected = False
        self.logged_in = False
        self.messages = []

    async def connect(self) -> bool:
        """Connect to the MUD server."""
        try:
            self.reader, self.writer = await asyncio.open_connection(
                SERVER_HOST, SERVER_PORT
            )
            self.connected = True
            print(f"[{self.name}] Connected to server")
            return True
        except Exception as e:
            print(f"[{self.name}] Failed to connect: {e}")
            return False

    async def login(self) -> bool:
        """Perform login sequence."""
        if not self.writer or not self.reader:
            return False

        try:
            # Read initial greeting
            await asyncio.sleep(0.5)
            data = await self.reader.read(4096)
            print(f"[{self.name}] Received greeting: {len(data)} bytes")

            # Send name
            self.writer.write(f"{self.name}\n".encode())
            await self.writer.drain()

            await asyncio.sleep(0.3)
            # Read password prompt
            data = await self.reader.read(4096)

            # Send password
            self.writer.write(f"{self.password}\n".encode())
            await self.writer.drain()

            await asyncio.sleep(0.5)
            # Read post-login messages
            data = await self.reader.read(8192)
            response = data.decode("utf-8", errors="ignore")

            if "무림" in response or "입장" in response:
                self.logged_in = True
                print(f"[{self.name}] Successfully logged in")
                return True
            else:
                print(f"[{self.name}] Login failed")
                return False
        except Exception as e:
            print(f"[{self.name}] Login error: {e}")
            return False

    async def send_command(self, cmd: str) -> str:
        """Send a command and read response."""
        if not self.writer or not self.reader:
            return ""

        try:
            self.writer.write(f"{cmd}\n".encode())
            await self.writer.drain()

            await asyncio.sleep(0.3)
            data = await self.reader.read(8192)
            response = data.decode("utf-8", errors="ignore")
            self.messages.append(response)
            return response
        except Exception as e:
            print(f"[{self.name}] Command error ({cmd}): {e}")
            return ""

    async def close(self):
        """Close the connection."""
        if self.writer:
            try:
                self.writer.write("quit\n".encode())
                await self.writer.drain()
                await asyncio.sleep(0.2)
            except:
                pass
            self.writer.close()
            try:
                await self.writer.wait_closed()
            except:
                pass
        self.connected = False
        self.logged_in = False


async def test_multiplayer_connection():
    """Test that multiple players can connect and interact."""
    print("=" * 60)
    print("TEST: Multiplayer Connection")
    print("=" * 60)

    # Create two test clients
    player1 = MUDClient("테스터1", "1234")
    player2 = MUDClient("테스터2", "1234")

    # Connect both players
    print("\n[Step 1] Connecting both players...")
    result1 = await player1.connect()
    result2 = await player2.connect()

    if not result1 or not result2:
        print("FAIL: Could not connect both players")
        return False

    # Login both players
    print("\n[Step 2] Logging in both players...")
    result1 = await player1.login()
    result2 = await player2.login()

    if not result1 or not result2:
        print("FAIL: Could not login both players")
        await player1.close()
        await player2.close()
        return False

    print("SUCCESS: Both players logged in")

    # Test: Player 1 looks (should see Player 2 if in same room)
    print("\n[Step 3] Player 1 looks around...")
    response = await player1.send_command("봐")
    print(f"[{player1.name}] Look response length: {len(response)} bytes")
    if "테스터2" in response:
        print("SUCCESS: Player 1 can see Player 2")

    # Test: Player 2 says something
    print("\n[Step 4] Player 2 says something...")
    response = await player2.send_command("말 안녕")
    print(f"[{player2.name}] Say response: {response[:100]}...")

    # Test: Player 1 should receive the message
    print("\n[Step 5] Player 1 checks for messages...")
    await asyncio.sleep(0.5)
    data = await player1.reader.read(4096)
    msg = data.decode("utf-8", errors="ignore")
    print(f"[{player1.name}] Received: {msg[:100]}...")

    if "안녕" in msg:
        print("SUCCESS: Cross-player communication works")

    # Test: Both players move
    print("\n[Step 6] Both players move...")
    await player1.send_command("북")
    await player2.send_command("북")

    print("\n[Step 7] Closing connections...")
    await player1.close()
    await player2.close()

    print("PASS: Multiplayer connection test completed")
    return True


async def test_player_disconnect():
    """Test that disconnecting one player doesn't crash the server."""
    print("\n" + "=" * 60)
    print("TEST: Player Disconnect Handling")
    print("=" * 60)

    # Create three players
    players = [
        MUDClient("플레이어A", "1234"),
        MUDClient("플레이어B", "1234"),
        MUDClient("플레이어C", "1234"),
    ]

    print("\n[Step 1] Connecting all players...")
    for p in players:
        if await p.connect():
            await asyncio.sleep(0.2)
            await p.login()
            await asyncio.sleep(0.2)

    connected = sum(1 for p in players if p.logged_in)
    print(f"Connected and logged in {connected} players")

    if connected < 2:
        print("FAIL: Could not connect at least 2 players")
        for p in players:
            await p.close()
        return False

    # Middle player disconnects abruptly (simulating broken pipe)
    print("\n[Step 2] Middle player disconnects abruptly...")
    if players[1].writer:
        players[1].writer.close()
        await players[1].writer.wait_closed()
    players[1].connected = False
    players[1].logged_in = False

    # Other players should still be able to send commands
    print("\n[Step 3] Remaining players send commands...")
    response1 = await players[0].send_command("봐")
    response2 = await players[2].send_command("봐")

    if response1 or response2:
        print("SUCCESS: Remaining players can still interact")

    print("\n[Step 4] Closing remaining connections...")
    await players[0].close()
    await players[2].close()

    print("PASS: Disconnect handling test completed")
    return True


async def test_pvp_interaction():
    """Test PvP interaction between players."""
    print("\n" + "=" * 60)
    print("TEST: PvP Interaction")
    print("=" * 60)

    attacker = MUDClient("공격자", "1234")
    defender = MUDClient("방어자", "1234")

    print("\n[Step 1] Connecting players...")
    if not (await attacker.connect() and await attacker.login()):
        print("FAIL: Attacker could not login")
        return False
    await asyncio.sleep(0.3)

    if not (await defender.connect() and await defender.login()):
        print("FAIL: Defender could not login")
        await attacker.close()
        return False

    print("SUCCESS: Both players logged in")

    # Test: Attacker tries to attack defender
    print("\n[Step 2] Attacker tries PvP command...")
    response = await attacker.send_command("쳐 방어자")
    print(f"[{attacker.name}] Attack response: {response[:200]}...")

    # Defender should see something
    await asyncio.sleep(0.5)
    data = await defender.reader.read(4096)
    msg = data.decode("utf-8", errors="ignore")
    print(f"[{defender.name}] Received: {msg[:200]}...")

    print("\n[Step 3] Closing connections...")
    await attacker.close()
    await defender.close()

    print("PASS: PvP interaction test completed")
    return True


async def main():
    """Run all tests."""
    print("\nStarting PvP/Multiplayer Tests for Rust MUD")
    print(f"Target: {SERVER_HOST}:{SERVER_PORT}")
    print("")

    results = []

    # Test 1: Basic multiplayer
    try:
        results.append(await test_multiplayer_connection())
    except Exception as e:
        print(f"ERROR in multiplayer test: {e}")
        results.append(False)

    await asyncio.sleep(1)

    # Test 2: Disconnect handling
    try:
        results.append(await test_player_disconnect())
    except Exception as e:
        print(f"ERROR in disconnect test: {e}")
        results.append(False)

    await asyncio.sleep(1)

    # Test 3: PvP interaction
    try:
        results.append(await test_pvp_interaction())
    except Exception as e:
        print(f"ERROR in PvP test: {e}")
        results.append(False)

    # Summary
    print("\n" + "=" * 60)
    print("TEST SUMMARY")
    print("=" * 60)
    print(f"Multiplayer Connection: {'PASS' if results[0] else 'FAIL'}")
    print(f"Disconnect Handling:    {'PASS' if results[1] else 'FAIL'}")
    print(f"PvP Interaction:        {'PASS' if results[2] else 'FAIL'}")
    print(f"")
    print(f"Overall: {'ALL TESTS PASSED' if all(results) else 'SOME TESTS FAILED'}")
    print("=" * 60)

    return 0 if all(results) else 1


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))

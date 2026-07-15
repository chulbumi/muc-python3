#!/usr/bin/env python3
"""Compare safe administrator commands against the Python and Rust servers."""

import re
import socket
import sys
import time


ANSI = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")
PROMPT = re.compile(r"\[\s*\d+/\d+\s*,\s*\d+/\d+\s*\]\s*$")


class Session:
    def __init__(self, port: int, name: str, password: str):
        self.sock = socket.create_connection(("127.0.0.1", port), timeout=5)
        self.sock.settimeout(0.2)
        self.name = name
        self.password = password
        self._drain()
        self._send(name)
        time.sleep(0.35)
        self._drain()
        self._send(password)
        time.sleep(0.35)
        self._drain()
        self._send("")
        time.sleep(0.25)
        self._drain()

    def _send(self, text: str):
        self.sock.sendall((text + "\r\n").encode("utf-8"))

    def _drain(self) -> str:
        chunks = []
        while True:
            try:
                chunks.append(self.sock.recv(8192).decode("utf-8", "ignore"))
            except socket.timeout:
                return "".join(chunks)

    def command(self, text: str) -> str:
        self._send(text)
        time.sleep(0.8)
        value = ANSI.sub("", self._drain()).replace("\r\n", "\n")
        value = PROMPT.sub("", value).strip()
        return value

    def close(self):
        self.sock.close()


def main() -> int:
    commands = sys.argv[1:] or ["기연리스트", "무공리스트", "맵 동"]
    sessions = []
    try:
        for port in (9903, 9999):
            sessions.append(Session(port, "운영자", "운영자"))
        failed = 0
        for command in commands:
            outputs = [session.command(command) for session in sessions]
            same = outputs[0] == outputs[1]
            print(f"[{command}] {'MATCH' if same else 'DIFFER'}")
            if not same:
                failed += 1
                print(f"  python: {outputs[0][:500]!r}")
                print(f"  rust:   {outputs[1][:500]!r}")
        return int(bool(failed))
    finally:
        for session in sessions:
            session.close()


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Offline regression tests for the Python/Rust MUD test harness."""

from __future__ import annotations

import contextlib
import importlib.util
import io
import socket
import subprocess
import sys
import threading
import unittest
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[1]
SKILL_DIR = ROOT / "skills" / "mud-test"
sys.path.insert(0, str(ROOT))

from test import test_mud_comprehensive as comprehensive


def load_script_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Cannot load module from {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


mud_test_skill = load_script_module("mud_test_skill", SKILL_DIR / "mud-test.py")
mud_test_socket = load_script_module("mud_test_socket", SKILL_DIR / "mud-test-socket.py")


class MUDHarnessRegressionTests(unittest.TestCase):
    def test_python_connection_reads_raw_socket_command_response(self):
        client, server = socket.socketpair()
        received = []
        expected = "파이썬 서버 실제 응답\r\n"

        def serve_once():
            try:
                received.append(server.recv(4096))
                server.sendall(expected.encode("utf-8"))
                server.shutdown(socket.SHUT_WR)
            finally:
                server.close()

        worker = threading.Thread(target=serve_once)
        worker.start()

        config = comprehensive.TestConfig(command_timeout=1)
        connection = comprehensive.MUDConnection(
            config, comprehensive.ServerType.PYTHON, 9900
        )
        connection.sock = client
        connection.connected = True
        connection.logged_in = True
        try:
            output = connection.execute_command("능력치", wait_time=1)
        finally:
            connection.disconnect()
            worker.join(timeout=2)

        self.assertEqual(received, ["능력치\r\n".encode("utf-8")])
        self.assertEqual(output, expected)

    def test_comparison_connection_requires_both_logins(self):
        config = comprehensive.TestConfig()
        runner = comprehensive.MUDTestRunner(config)

        def connect(connection):
            connection.connected = True
            return True

        def only_python_login(connection):
            connection.logged_in = (
                connection.server_type == comprehensive.ServerType.PYTHON
            )
            return connection.logged_in

        with mock.patch.object(comprehensive.MUDConnection, "connect", connect), \
                mock.patch.object(
                    comprehensive.MUDConnection,
                    "login_or_create",
                    only_python_login,
                ), contextlib.redirect_stdout(io.StringIO()):
            self.assertFalse(runner._connect_servers())

        with contextlib.redirect_stdout(io.StringIO()):
            runner._cleanup()

    def test_skill_run_failure_becomes_nonzero_exit(self):
        class FailingRunner:
            def __init__(self, config):
                self.config = config

            def run_all_tests(self):
                return False

        with mock.patch.object(mud_test_skill, "MUDTestRunner", FailingRunner), \
                contextlib.redirect_stdout(io.StringIO()):
            exit_code = mud_test_skill.MUDTestSkill().run_test("all")

        self.assertEqual(exit_code, 1)

    def test_quick_comparison_rejects_same_length_but_different_output(self):
        class Connection:
            logged_in = True
            character_name = "비교테스터"

            def __init__(self, output):
                self.output = output

            def execute_command(self, command, wait_time=1.0):
                return self.output

        class Runner:
            def __init__(self):
                self.py_conn = Connection("파이썬")
                self.rust_conn = Connection("러스트")

            def _check_servers(self):
                pass

            def _connect_servers(self):
                return True

            def _cleanup(self):
                pass

        with contextlib.redirect_stdout(io.StringIO()):
            result = mud_test_skill.MUDTestSkill()._run_quick_test(Runner())

        self.assertFalse(result)

    def test_standalone_runner_failure_becomes_nonzero_exit(self):
        class FailingRunner:
            def __init__(self, config):
                self.config = config

            def run_all_tests(self):
                return False

            def _cleanup(self):
                pass

        with mock.patch.object(
            comprehensive, "parse_arguments", return_value=comprehensive.TestConfig()
        ), mock.patch.object(comprehensive, "MUDTestRunner", FailingRunner), \
                contextlib.redirect_stdout(io.StringIO()):
            exit_code = comprehensive.main()

        self.assertEqual(exit_code, 1)

    def test_raw_socket_cli_propagates_run_test_failure(self):
        with mock.patch.object(mud_test_socket, "SocketMUDTest") as test_class, \
                mock.patch.object(sys, "argv", ["mud-test-socket.py"]):
            test_class.return_value.run_test.return_value = False
            exit_code = mud_test_socket.main()

        self.assertEqual(exit_code, 1)

    def test_wrapper_supports_help_and_equals_options(self):
        help_result = subprocess.run(
            [str(SKILL_DIR / "mud-test"), "-h"],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=15,
            check=False,
        )

        self.assertEqual(help_result.returncode, 0, help_result.stdout)
        self.assertIn("MUD Test Skill - Python/Rust Comparison", help_result.stdout)

        py_port, rust_port = self._closed_ports(2)
        command = [
            str(SKILL_DIR / "mud-test"),
            "status",
            "--host=127.0.0.1",
            f"--py-port={py_port}",
            f"--rust-port={rust_port}",
            f"--report={ROOT / 'unused-offline-report.md'}",
        ]

        result = subprocess.run(
            command,
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=15,
            check=False,
        )

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn(f"Python (127.0.0.1:{py_port}): OFFLINE", result.stdout)
        self.assertIn(f"Rust (127.0.0.1:{rust_port}): OFFLINE", result.stdout)

    def test_wrapper_quick_is_nonzero_when_both_servers_are_offline(self):
        py_port, rust_port = self._closed_ports(2)
        command = [
            str(SKILL_DIR / "mud-test"),
            "quick",
            "--host=127.0.0.1",
            f"--py-port={py_port}",
            f"--rust-port={rust_port}",
        ]

        result = subprocess.run(
            command,
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=20,
            check=False,
        )

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("Python server status: FAILED", result.stdout)
        self.assertIn("Rust server status: FAILED", result.stdout)

    @staticmethod
    def _closed_ports(count: int):
        ports = []
        while len(ports) < count:
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
                sock.bind(("127.0.0.1", 0))
                port = sock.getsockname()[1]
            if port not in ports:
                ports.append(port)
        return ports


if __name__ == "__main__":
    unittest.main()

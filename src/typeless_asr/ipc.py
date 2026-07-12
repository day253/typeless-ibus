from __future__ import annotations

import os
import socket
import threading
from collections.abc import Callable
from contextlib import suppress
from hashlib import sha256
from pathlib import Path

VALID_COMMANDS = {"toggle", "start", "stop", "show", "quit"}


def effective_socket_path(path: Path) -> Path:
    """Keep AF_UNIX paths below the stricter macOS sockaddr limit."""
    if len(os.fsencode(path)) <= 90:
        return path
    digest = sha256(os.fsencode(path)).hexdigest()[:16]
    return Path("/tmp") / f"typeless-asr-{os.getuid()}-{digest}.sock"


def send_command(path: Path, command: str, timeout: float = 1.0) -> bool:
    if command not in VALID_COMMANDS:
        raise ValueError(f"unknown command: {command}")
    client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    client.settimeout(timeout)
    try:
        client.connect(str(effective_socket_path(path)))
        client.sendall((command + "\n").encode())
        return client.recv(16).strip() == b"ok"
    except (FileNotFoundError, ConnectionError, OSError, TimeoutError):
        return False
    finally:
        client.close()


class ControlServer:
    def __init__(self, path: Path, on_command: Callable[[str], None]) -> None:
        self.path = effective_socket_path(path)
        self.on_command = on_command
        self._stop = threading.Event()
        self._server: socket.socket | None = None
        self._thread: threading.Thread | None = None

    def start(self) -> None:
        self.path.parent.mkdir(parents=True, exist_ok=True)
        if self.path.exists():
            self.path.unlink()
        self._server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self._server.bind(str(self.path))
        os.chmod(self.path, 0o600)
        self._server.listen(4)
        self._server.settimeout(0.2)
        self._thread = threading.Thread(target=self._serve, name="typeless-ipc", daemon=True)
        self._thread.start()

    def _serve(self) -> None:
        assert self._server is not None
        while not self._stop.is_set():
            try:
                connection, _ = self._server.accept()
            except TimeoutError:
                continue
            except OSError:
                break
            with connection:
                try:
                    command = connection.recv(64).decode().strip()
                    if command in VALID_COMMANDS:
                        self.on_command(command)
                        connection.sendall(b"ok\n")
                    else:
                        connection.sendall(b"error\n")
                except OSError:
                    continue

    def stop(self) -> None:
        self._stop.set()
        if self._server is not None:
            self._server.close()
        if self._thread is not None:
            self._thread.join(timeout=1)
        with suppress(FileNotFoundError):
            self.path.unlink()

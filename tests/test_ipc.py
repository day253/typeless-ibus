from __future__ import annotations

import time

from typeless_asr.ipc import ControlServer, send_command


def test_control_server_round_trip(tmp_path) -> None:
    received: list[str] = []
    path = tmp_path / "control.sock"
    server = ControlServer(path, received.append)
    server.start()
    try:
        assert send_command(path, "toggle") is True
        deadline = time.monotonic() + 1
        while not received and time.monotonic() < deadline:
            time.sleep(0.01)
        assert received == ["toggle"]
    finally:
        server.stop()

    assert not path.exists()


def test_missing_server_returns_false(tmp_path) -> None:
    assert send_command(tmp_path / "missing.sock", "toggle") is False

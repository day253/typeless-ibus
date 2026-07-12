from __future__ import annotations

import json

from typeless_asr.config import AppConfig


def test_config_round_trip(tmp_path) -> None:
    path = tmp_path / "config.json"
    expected = AppConfig(
        hotkey="<alt>+space",
        auto_paste=False,
        restore_clipboard=False,
        input_device=3,
    )

    expected.save(path)

    assert AppConfig.load(path) == expected


def test_config_ignores_unknown_fields(tmp_path) -> None:
    path = tmp_path / "config.json"
    path.write_text(json.dumps({"hotkey": "<ctrl>+a", "future_option": True}))

    loaded = AppConfig.load(path)

    assert loaded.hotkey == "<ctrl>+a"
    assert loaded.auto_paste is True


def test_invalid_config_falls_back_to_defaults(tmp_path) -> None:
    path = tmp_path / "config.json"
    path.write_text("not json")

    assert AppConfig.load(path) == AppConfig()

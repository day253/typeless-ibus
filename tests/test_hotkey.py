from __future__ import annotations

import pytest

from typeless_asr.hotkey import display_hotkey, normalize_hotkey


@pytest.mark.parametrize(
    ("raw", "expected"),
    [
        ("ctrl + shift + space", "<ctrl>+<shift>+<space>"),
        ("<alt>+x", "<alt>+x"),
        ("cmd+f12", "<cmd>+<f12>"),
    ],
)
def test_normalize_hotkey(raw: str, expected: str) -> None:
    assert normalize_hotkey(raw) == expected


@pytest.mark.parametrize("raw", ["space", "ctrl+shift", "ctrl+wat", "ctrl+ctrl+x", ""])
def test_rejects_invalid_hotkey(raw: str) -> None:
    with pytest.raises(ValueError):
        normalize_hotkey(raw)


def test_display_hotkey() -> None:
    assert display_hotkey("<ctrl>+<shift>+<space>") == "Ctrl+Shift+Space"

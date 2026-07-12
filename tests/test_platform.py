from __future__ import annotations

from typeless_asr.platform import SessionType, supports_auto_paste, supports_native_hotkey


def test_wayland_uses_safe_fallbacks() -> None:
    assert supports_native_hotkey(SessionType.WAYLAND) is False
    assert supports_auto_paste(SessionType.WAYLAND) is False


def test_x11_and_macos_support_native_flow() -> None:
    for current in (SessionType.X11, SessionType.MACOS):
        assert supports_native_hotkey(current) is True
        assert supports_auto_paste(current) is True

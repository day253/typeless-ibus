from __future__ import annotations

import os
import sys
from enum import StrEnum


class SessionType(StrEnum):
    MACOS = "macos"
    X11 = "x11"
    WAYLAND = "wayland"
    UNKNOWN = "unknown"


def session_type() -> SessionType:
    if sys.platform == "darwin":
        return SessionType.MACOS
    if not sys.platform.startswith("linux"):
        return SessionType.UNKNOWN

    declared = os.environ.get("XDG_SESSION_TYPE", "").casefold()
    if declared == "wayland" or os.environ.get("WAYLAND_DISPLAY"):
        return SessionType.WAYLAND
    if declared == "x11" or os.environ.get("DISPLAY"):
        return SessionType.X11
    return SessionType.UNKNOWN


def supports_native_hotkey(current: SessionType | None = None) -> bool:
    return (current or session_type()) in {SessionType.MACOS, SessionType.X11}


def supports_auto_paste(current: SessionType | None = None) -> bool:
    return (current or session_type()) in {SessionType.MACOS, SessionType.X11}


def platform_note(current: SessionType | None = None) -> str:
    current = current or session_type()
    if current == SessionType.WAYLAND:
        return (
            "Wayland 会限制全局按键和模拟粘贴。请把桌面快捷键绑定到 "
            "`typeless-asr --toggle`；识别结果会保留在剪贴板。"
        )
    if current == SessionType.MACOS:
        return "首次使用请授予麦克风和辅助功能权限。"
    if current == SessionType.X11:
        return "当前为 X11，会自动注册全局快捷键并粘贴识别结果。"
    return "未识别到受支持的桌面会话；仍可从托盘操作并复制识别结果。"

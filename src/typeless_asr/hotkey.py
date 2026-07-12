from __future__ import annotations

import re
from collections.abc import Callable

MODIFIERS = {"ctrl", "alt", "shift", "cmd", "cmd_l", "cmd_r"}
NAMED_KEYS = {
    "space",
    "enter",
    "tab",
    "esc",
    "backspace",
    "delete",
    "insert",
    "home",
    "end",
    "page_up",
    "page_down",
    "up",
    "down",
    "left",
    "right",
}


def normalize_hotkey(value: str) -> str:
    parts = [part.strip().casefold() for part in value.split("+") if part.strip()]
    normalized: list[str] = []
    for part in parts:
        bare = part[1:-1] if part.startswith("<") and part.endswith(">") else part
        if bare in MODIFIERS or bare in NAMED_KEYS or re.fullmatch(r"f(?:[1-9]|1\d|2[0-4])", bare):
            normalized.append(f"<{bare}>")
        elif len(bare) == 1 and bare.isprintable():
            normalized.append(bare)
        else:
            raise ValueError(f"不支持的按键：{part}")

    modifier_count = sum(item.strip("<>") in MODIFIERS for item in normalized)
    primary_count = len(normalized) - modifier_count
    if modifier_count < 1 or primary_count != 1:
        raise ValueError("快捷键需要至少一个修饰键和一个普通按键")
    if len(set(normalized)) != len(normalized):
        raise ValueError("快捷键中存在重复按键")
    return "+".join(normalized)


def display_hotkey(value: str) -> str:
    replacements = {
        "<ctrl>": "Ctrl",
        "<alt>": "Alt",
        "<shift>": "Shift",
        "<cmd>": "Cmd",
        "<cmd_l>": "Left Cmd",
        "<cmd_r>": "Right Cmd",
        "<space>": "Space",
    }
    return "+".join(replacements.get(part, part.strip("<>").upper()) for part in value.split("+"))


class GlobalHotkey:
    """Small lazy-import wrapper so headless tests do not require an X connection."""

    def __init__(self, hotkey: str, callback: Callable[[], None]) -> None:
        self.hotkey = normalize_hotkey(hotkey)
        self.callback = callback
        self._listener = None

    def start(self) -> None:
        from pynput import keyboard

        self._listener = keyboard.GlobalHotKeys({self.hotkey: self.callback})
        self._listener.start()

    def stop(self) -> None:
        if self._listener is not None:
            self._listener.stop()
            self._listener = None

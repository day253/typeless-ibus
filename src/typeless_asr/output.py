from __future__ import annotations

import subprocess
import sys

from PySide6.QtCore import QObject, QTimer, Signal
from PySide6.QtGui import QGuiApplication

from .platform import SessionType, session_type, supports_auto_paste


class TextOutput(QObject):
    completed = Signal(str)
    warning = Signal(str)

    def __init__(self, auto_paste: bool, restore_clipboard: bool) -> None:
        super().__init__()
        self.auto_paste = auto_paste
        self.restore_clipboard = restore_clipboard

    def update_options(self, *, auto_paste: bool, restore_clipboard: bool) -> None:
        self.auto_paste = auto_paste
        self.restore_clipboard = restore_clipboard

    def deliver(self, text: str) -> None:
        clipboard = QGuiApplication.clipboard()
        previous = clipboard.text()
        clipboard.setText(text)

        if not self.auto_paste or not supports_auto_paste():
            self.completed.emit("识别结果已复制到剪贴板")
            if self.auto_paste and session_type() == SessionType.WAYLAND:
                self.warning.emit("Wayland 不允许应用模拟粘贴，请手动按 Ctrl+V")
            return

        QTimer.singleShot(40, lambda: self._paste_and_maybe_restore(text, previous))

    def _paste_and_maybe_restore(self, inserted: str, previous: str) -> None:
        try:
            _simulate_paste()
        except Exception as error:
            self.warning.emit(f"自动粘贴失败，结果已保留在剪贴板：{error}")
            return

        self.completed.emit("识别结果已输入")
        if self.restore_clipboard:
            QTimer.singleShot(900, lambda: _restore_if_unchanged(inserted, previous))


def _restore_if_unchanged(inserted: str, previous: str) -> None:
    clipboard = QGuiApplication.clipboard()
    if clipboard.text() == inserted:
        clipboard.setText(previous)


def _simulate_paste() -> None:
    if sys.platform == "darwin":
        completed = subprocess.run(
            [
                "osascript",
                "-e",
                'tell application "System Events" to keystroke "v" using command down',
            ],
            check=False,
            capture_output=True,
            text=True,
            timeout=3,
        )
        if completed.returncode != 0:
            raise RuntimeError(completed.stderr.strip() or "osascript 返回错误")
        return

    from pynput.keyboard import Controller, Key

    keyboard = Controller()
    with keyboard.pressed(Key.ctrl):
        keyboard.press("v")
        keyboard.release("v")

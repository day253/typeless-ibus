from __future__ import annotations

import sys

from PySide6.QtCore import QObject, QThread, Signal, Slot
from PySide6.QtGui import QAction
from PySide6.QtWidgets import QApplication, QMenu, QSystemTrayIcon

from .config import AppConfig, credentials_path, socket_path
from .hotkey import GlobalHotkey, display_hotkey
from .ipc import ControlServer
from .output import TextOutput
from .platform import supports_native_hotkey
from .ui import RecordingPill, SettingsWindow, app_icon
from .worker import DictationWorker


class CommandBridge(QObject):
    received = Signal(str)


class TypelessApplication(QObject):
    def __init__(self, qt_app: QApplication) -> None:
        super().__init__()
        self.qt_app = qt_app
        self.config = AppConfig.load()
        self.window = SettingsWindow(self.config)
        self.pill = RecordingPill()
        self.output = TextOutput(self.config.auto_paste, self.config.restore_clipboard)
        self.command_bridge = CommandBridge()
        self.control_server = ControlServer(socket_path(), self.command_bridge.received.emit)

        self._hotkey: GlobalHotkey | None = None
        self._thread: QThread | None = None
        self._worker: DictationWorker | None = None
        self._recording = False
        self._stopping = False
        self._had_result = False
        self._quitting = False

        self._build_tray()
        self._connect_signals()

    def _build_tray(self) -> None:
        self.tray = QSystemTrayIcon(app_icon(), self)
        menu = QMenu()
        self.toggle_action = QAction("开始录音", menu)
        self.toggle_action.triggered.connect(self.toggle)
        settings_action = QAction("设置", menu)
        settings_action.triggered.connect(self.show_settings)
        quit_action = QAction("退出", menu)
        quit_action.triggered.connect(self.quit)
        menu.addAction(self.toggle_action)
        menu.addSeparator()
        menu.addAction(settings_action)
        menu.addAction(quit_action)
        self.tray.setContextMenu(menu)
        self.tray.activated.connect(self._tray_activated)

    def _connect_signals(self) -> None:
        self.window.toggle_requested.connect(self.toggle)
        self.window.save_requested.connect(self.save_config)
        self.output.completed.connect(self._output_completed)
        self.output.warning.connect(self._show_warning)
        self.command_bridge.received.connect(self._handle_command)

    def start(self) -> None:
        self.control_server.start()
        self.tray.show()
        self._install_hotkey()
        hotkey_text = display_hotkey(self.config.hotkey)
        self.window.set_status(f"就绪 · 按 {hotkey_text} 开始录音")
        if not QSystemTrayIcon.isSystemTrayAvailable():
            self.window.show()
        else:
            self.tray.showMessage(
                "Typeless ASR 已启动",
                f"按 {hotkey_text} 开始或结束录音",
                QSystemTrayIcon.MessageIcon.Information,
                2500,
            )

    def _install_hotkey(self) -> None:
        if self._hotkey is not None:
            self._hotkey.stop()
            self._hotkey = None
        if not supports_native_hotkey():
            return
        try:
            self._hotkey = GlobalHotkey(
                self.config.hotkey,
                lambda: self.command_bridge.received.emit("toggle"),
            )
            self._hotkey.start()
        except Exception as error:
            self._hotkey = None
            self._show_warning(f"全局快捷键注册失败：{error}")

    @Slot()
    def toggle(self) -> None:
        if self._stopping:
            return
        if self._recording:
            self.stop_recording()
        else:
            self.start_recording()

    @Slot()
    def start_recording(self) -> None:
        if self._recording or self._thread is not None:
            return
        self._recording = True
        self._stopping = False
        self._had_result = False
        self.window.transcript.clear()
        self.window.set_recording(True)
        self.window.set_status("正在连接豆包语音识别…")
        self.toggle_action.setText("结束并识别")
        self.pill.show_centered()

        thread = QThread(self)
        worker = DictationWorker(credentials_path(), self.config.input_device)
        worker.moveToThread(thread)
        thread.started.connect(worker.run)
        worker.state_changed.connect(self.window.set_status)
        worker.interim_changed.connect(self.window.transcript.setPlainText)
        worker.result_ready.connect(self._result_ready)
        worker.failed.connect(self._worker_failed)
        worker.finished.connect(thread.quit)
        worker.finished.connect(worker.deleteLater)
        thread.finished.connect(self._session_finished)
        thread.finished.connect(thread.deleteLater)
        self._thread = thread
        self._worker = worker
        thread.start()

    @Slot()
    def stop_recording(self) -> None:
        if not self._recording or self._worker is None:
            return
        self._stopping = True
        self.pill.hide()
        self.window.set_status("正在完成识别…")
        self.toggle_action.setText("正在识别…")
        self.window.toggle_button.setEnabled(False)
        self._worker.request_stop()

    @Slot(str)
    def _result_ready(self, text: str) -> None:
        self._had_result = True
        self.window.transcript.setPlainText(text)
        self.output.deliver(text)

    @Slot(str)
    def _worker_failed(self, message: str) -> None:
        self.window.set_status(message, error=True)
        self.tray.showMessage(
            "Typeless ASR",
            message,
            QSystemTrayIcon.MessageIcon.Warning,
            4000,
        )

    @Slot()
    def _session_finished(self) -> None:
        self._recording = False
        self._stopping = False
        self._worker = None
        self._thread = None
        self.pill.hide()
        self.window.set_recording(False)
        self.window.toggle_button.setEnabled(True)
        self.toggle_action.setText("开始录音")
        error_prefixes = ("没有", "语音", "无法")
        if not self._had_result and not self.window.status.text().startswith(error_prefixes):
            self.window.set_status("就绪")
        if self._quitting:
            self._finalize_quit()

    @Slot(str)
    def _output_completed(self, message: str) -> None:
        self.window.set_status(message)
        self.tray.showMessage(
            "Typeless ASR",
            message,
            QSystemTrayIcon.MessageIcon.Information,
            1800,
        )

    @Slot(str)
    def _show_warning(self, message: str) -> None:
        self.window.set_status(message, error=True)
        if self.tray.isVisible():
            self.tray.showMessage(
                "Typeless ASR",
                message,
                QSystemTrayIcon.MessageIcon.Warning,
                4500,
            )

    @Slot(object)
    def save_config(self, config: AppConfig) -> None:
        try:
            config.save()
            self.config = config
            self.output.update_options(
                auto_paste=config.auto_paste,
                restore_clipboard=config.restore_clipboard,
            )
            self._install_hotkey()
            self.window.update_config(config)
            self.window.set_status(f"设置已保存 · 快捷键 {display_hotkey(config.hotkey)}")
        except OSError as error:
            self.window.set_status(f"设置保存失败：{error}", error=True)

    @Slot(str)
    def _handle_command(self, command: str) -> None:
        handlers = {
            "toggle": self.toggle,
            "start": self.start_recording,
            "stop": self.stop_recording,
            "show": self.show_settings,
            "quit": self.quit,
        }
        handler = handlers.get(command)
        if handler is not None:
            handler()

    @Slot()
    def show_settings(self) -> None:
        self.window.show()
        self.window.raise_()
        self.window.activateWindow()

    def _tray_activated(self, reason: QSystemTrayIcon.ActivationReason) -> None:
        if reason == QSystemTrayIcon.ActivationReason.Trigger:
            self.show_settings()

    @Slot()
    def quit(self) -> None:
        if self._worker is not None:
            self._quitting = True
            self.pill.hide()
            self.window.set_status("正在结束录音后退出…")
            self._worker.request_stop()
            return
        self._finalize_quit()

    def _finalize_quit(self) -> None:
        if self._hotkey is not None:
            self._hotkey.stop()
        self.control_server.stop()
        self.tray.hide()
        self.window.really_close()
        self.qt_app.quit()


def run_gui() -> int:
    qt_app = QApplication(sys.argv)
    qt_app.setApplicationName("Typeless ASR")
    qt_app.setOrganizationName("day253")
    qt_app.setQuitOnLastWindowClosed(False)
    controller = TypelessApplication(qt_app)
    controller.start()
    # Keep the controller alive for the full Qt event loop.
    qt_app._typeless_controller = controller  # type: ignore[attr-defined]
    return qt_app.exec()

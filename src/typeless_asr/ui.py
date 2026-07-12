from __future__ import annotations

from PySide6.QtCore import Qt, Signal
from PySide6.QtGui import QCloseEvent, QColor, QFont, QIcon, QPainter, QPixmap
from PySide6.QtWidgets import (
    QCheckBox,
    QComboBox,
    QFormLayout,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QMainWindow,
    QPushButton,
    QSystemTrayIcon,
    QTextEdit,
    QVBoxLayout,
    QWidget,
)

from .config import AppConfig
from .hotkey import normalize_hotkey
from .platform import platform_note, supports_auto_paste


def microphone_devices() -> list[tuple[int, str]]:
    try:
        import sounddevice as sd

        return [
            (index, str(device["name"]))
            for index, device in enumerate(sd.query_devices())
            if int(device["max_input_channels"]) > 0
        ]
    except Exception:
        return []


def app_icon(color: QColor | None = None) -> QIcon:
    pixmap = QPixmap(64, 64)
    pixmap.fill(Qt.GlobalColor.transparent)
    painter = QPainter(pixmap)
    painter.setRenderHint(QPainter.RenderHint.Antialiasing)
    painter.setPen(Qt.PenStyle.NoPen)
    painter.setBrush(color or QColor("#6C5CE7"))
    painter.drawRoundedRect(8, 8, 48, 48, 15, 15)
    painter.setPen(QColor("white"))
    font = QFont()
    font.setBold(True)
    font.setPixelSize(34)
    painter.setFont(font)
    painter.drawText(pixmap.rect(), Qt.AlignmentFlag.AlignCenter, "T")
    painter.end()
    return QIcon(pixmap)


class SettingsWindow(QMainWindow):
    toggle_requested = Signal()
    save_requested = Signal(object)

    def __init__(self, config: AppConfig) -> None:
        super().__init__()
        self.setWindowTitle("Typeless ASR")
        self.setWindowIcon(app_icon())
        self.setMinimumSize(580, 430)
        self._allow_close = False

        root = QWidget()
        layout = QVBoxLayout(root)
        layout.setContentsMargins(28, 24, 28, 24)
        layout.setSpacing(14)

        title = QLabel("Typeless ASR")
        title.setStyleSheet("font-size: 26px; font-weight: 700;")
        subtitle = QLabel("轻量的 macOS / Linux 语音输入 · 无 LLM")
        subtitle.setStyleSheet("color: #777;")
        layout.addWidget(title)
        layout.addWidget(subtitle)

        self.status = QLabel("就绪")
        self.status.setStyleSheet(
            "background: #f2f0ff; color: #4f3dc8; padding: 10px 12px; border-radius: 8px;"
        )
        layout.addWidget(self.status)

        self.transcript = QTextEdit()
        self.transcript.setReadOnly(True)
        self.transcript.setPlaceholderText("识别中的文字会显示在这里")
        self.transcript.setMaximumHeight(110)
        layout.addWidget(self.transcript)

        form = QFormLayout()
        self.hotkey = QLineEdit(config.hotkey)
        self.hotkey.setPlaceholderText("<ctrl>+<shift>+space")
        form.addRow("全局快捷键", self.hotkey)

        self.device = QComboBox()
        self.device.addItem("系统默认麦克风", None)
        selected_index = 0
        for index, name in microphone_devices():
            self.device.addItem(name, index)
            if config.input_device == index:
                selected_index = self.device.count() - 1
        self.device.setCurrentIndex(selected_index)
        form.addRow("麦克风", self.device)
        layout.addLayout(form)

        self.auto_paste = QCheckBox("识别完成后自动粘贴到当前应用")
        self.auto_paste.setChecked(config.auto_paste)
        if not supports_auto_paste():
            self.auto_paste.setEnabled(False)
            self.auto_paste.setChecked(False)
        self.restore_clipboard = QCheckBox("粘贴后恢复原剪贴板文字")
        self.restore_clipboard.setChecked(config.restore_clipboard)
        layout.addWidget(self.auto_paste)
        layout.addWidget(self.restore_clipboard)

        note = QLabel(platform_note())
        note.setWordWrap(True)
        note.setTextFormat(Qt.TextFormat.MarkdownText)
        note.setStyleSheet("color: #666;")
        layout.addWidget(note)

        buttons = QHBoxLayout()
        self.toggle_button = QPushButton("开始录音")
        self.toggle_button.clicked.connect(self.toggle_requested)
        save_button = QPushButton("保存设置")
        save_button.clicked.connect(self._save)
        buttons.addWidget(self.toggle_button)
        buttons.addStretch()
        buttons.addWidget(save_button)
        layout.addLayout(buttons)

        self.setCentralWidget(root)

    def _save(self) -> None:
        try:
            hotkey = normalize_hotkey(self.hotkey.text())
        except ValueError as error:
            self.set_status(str(error), error=True)
            return
        config = AppConfig(
            hotkey=hotkey,
            auto_paste=self.auto_paste.isChecked(),
            restore_clipboard=self.restore_clipboard.isChecked(),
            input_device=self.device.currentData(),
        )
        self.hotkey.setText(hotkey)
        self.save_requested.emit(config)

    def set_recording(self, recording: bool) -> None:
        self.toggle_button.setText("结束并识别" if recording else "开始录音")

    def set_status(self, message: str, *, error: bool = False) -> None:
        self.status.setText(message)
        if error:
            self.status.setStyleSheet(
                "background: #fff0f0; color: #b42318; padding: 10px 12px; border-radius: 8px;"
            )
        else:
            self.status.setStyleSheet(
                "background: #f2f0ff; color: #4f3dc8; padding: 10px 12px; border-radius: 8px;"
            )

    def update_config(self, config: AppConfig) -> None:
        self.hotkey.setText(config.hotkey)
        self.auto_paste.setChecked(config.auto_paste and supports_auto_paste())
        self.restore_clipboard.setChecked(config.restore_clipboard)

    def closeEvent(self, event: QCloseEvent) -> None:  # noqa: N802
        if self._allow_close or not QSystemTrayIcon.isSystemTrayAvailable():
            event.accept()
        else:
            event.ignore()
            self.hide()

    def really_close(self) -> None:
        self._allow_close = True
        self.close()


class RecordingPill(QWidget):
    def __init__(self) -> None:
        flags = (
            Qt.WindowType.Tool
            | Qt.WindowType.FramelessWindowHint
            | Qt.WindowType.WindowStaysOnTopHint
        )
        super().__init__(None, flags)
        self.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground)
        label = QLabel("●  正在聆听…  再按快捷键结束", self)
        label.setStyleSheet(
            "background: rgba(28, 28, 32, 235); color: white; padding: 11px 18px; "
            "border-radius: 18px; font-size: 13px;"
        )
        layout = QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.addWidget(label)
        self.adjustSize()

    def show_centered(self) -> None:
        screen = self.screen() or self.windowHandle().screen()
        area = screen.availableGeometry()
        self.adjustSize()
        self.move(area.center().x() - self.width() // 2, area.bottom() - self.height() - 50)
        self.show()

from __future__ import annotations

import asyncio
import threading
from collections.abc import AsyncIterator
from contextlib import suppress
from pathlib import Path

from PySide6.QtCore import QObject, Signal, Slot


class DictationWorker(QObject):
    state_changed = Signal(str)
    interim_changed = Signal(str)
    result_ready = Signal(str)
    failed = Signal(str)
    finished = Signal()

    def __init__(self, credential_path: Path, input_device: int | None = None) -> None:
        super().__init__()
        self.credential_path = credential_path
        self.input_device = input_device
        self._stop = threading.Event()

    def request_stop(self) -> None:
        self._stop.set()

    @Slot()
    def run(self) -> None:
        self._stop.clear()
        try:
            asyncio.run(self._run_session())
        except Exception as error:  # worker boundary: turn dependency errors into UI state
            self.failed.emit(_friendly_error(error))
        finally:
            self.finished.emit()

    async def _microphone_source(
        self,
        sample_rate: int,
        channels: int,
        frame_duration_ms: int,
    ) -> AsyncIterator[bytes]:
        import sounddevice as sd

        loop = asyncio.get_running_loop()
        queue: asyncio.Queue[bytes] = asyncio.Queue(maxsize=100)
        samples_per_frame = sample_rate * frame_duration_ms // 1000

        def enqueue(data: bytes) -> None:
            if queue.full():
                with suppress(asyncio.QueueEmpty):
                    queue.get_nowait()
            queue.put_nowait(data)

        def callback(indata, frames, time_info, status) -> None:  # noqa: ANN001, ARG001
            if status:
                self.state_changed.emit(f"录音设备提示：{status}")
            loop.call_soon_threadsafe(enqueue, bytes(indata))

        with sd.RawInputStream(
            samplerate=sample_rate,
            channels=channels,
            dtype="int16",
            blocksize=samples_per_frame,
            device=self.input_device,
            callback=callback,
        ):
            self.state_changed.emit("正在聆听，再按一次快捷键结束")
            while not self._stop.is_set():
                try:
                    yield await asyncio.wait_for(queue.get(), timeout=0.15)
                except TimeoutError:
                    continue

    async def _run_session(self) -> None:
        from .native_libs import configure_native_libraries

        configure_native_libraries()
        from doubaoime_asr import ASRConfig, ResponseType, transcribe_realtime

        config = ASRConfig(credential_path=self.credential_path)
        final_text = ""
        self.state_changed.emit("正在连接豆包语音识别…")

        source = self._microphone_source(
            sample_rate=config.sample_rate,
            channels=config.channels,
            frame_duration_ms=config.frame_duration_ms,
        )
        async for response in transcribe_realtime(source, config=config):
            if response.type == ResponseType.SESSION_STARTED:
                self.state_changed.emit("正在聆听，再按一次快捷键结束")
            elif response.type == ResponseType.INTERIM_RESULT:
                self.interim_changed.emit(response.text)
            elif response.type == ResponseType.FINAL_RESULT:
                final_text = response.text
                self.interim_changed.emit(response.text)
            elif response.type == ResponseType.ERROR:
                raise RuntimeError(response.error_msg or "豆包语音识别返回错误")

        final_text = final_text.strip()
        if final_text:
            self.result_ready.emit(final_text)
        elif self._stop.is_set():
            self.failed.emit("没有识别到语音，请靠近麦克风后重试")


def _friendly_error(error: Exception) -> str:
    message = str(error).strip()
    name = type(error).__name__
    if "PortAudio" in message or name == "PortAudioError":
        return f"无法使用麦克风：{message}"
    if "Opus" in message or "opus" in message:
        return "无法加载 Opus。macOS 请运行 `brew install opus`；Linux 请安装 `libopus0`。"
    if "WebSocket" in message or "connect" in message.casefold():
        return f"无法连接语音识别服务：{message}"
    return f"语音识别失败（{name}）：{message or '未知错误'}"

from __future__ import annotations

import json
import os
from contextlib import suppress
from dataclasses import asdict, dataclass, fields
from pathlib import Path
from typing import Any

from platformdirs import user_config_path

APP_NAME = "typeless-asr"


def config_dir() -> Path:
    override = os.environ.get("TYPELESS_ASR_CONFIG_DIR")
    if override:
        return Path(override).expanduser()
    return user_config_path(APP_NAME, appauthor=False)


def config_path() -> Path:
    return config_dir() / "config.json"


def credentials_path() -> Path:
    return config_dir() / "credentials.json"


def socket_path() -> Path:
    return config_dir() / "control.sock"


@dataclass(slots=True)
class AppConfig:
    hotkey: str = "<ctrl>+<shift>+space"
    auto_paste: bool = True
    restore_clipboard: bool = True
    input_device: int | None = None

    @classmethod
    def load(cls, path: Path | None = None) -> AppConfig:
        target = path or config_path()
        try:
            raw = json.loads(target.read_text(encoding="utf-8"))
        except (FileNotFoundError, json.JSONDecodeError, OSError):
            return cls()

        allowed = {item.name for item in fields(cls)}
        values = {key: value for key, value in raw.items() if key in allowed}
        try:
            return cls(**values)
        except TypeError:
            return cls()

    def save(self, path: Path | None = None) -> None:
        target = path or config_path()
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(
            json.dumps(asdict(self), ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        with suppress(OSError):
            target.chmod(0o600)

    def updated(self, **changes: Any) -> AppConfig:
        values = asdict(self)
        values.update(changes)
        return AppConfig(**values)

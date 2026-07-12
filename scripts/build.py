from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
    launcher = ROOT / "scripts" / "launcher.py"
    command = [
        sys.executable,
        "-m",
        "PyInstaller",
        "--noconfirm",
        "--clean",
        "--windowed",
        "--name",
        "TypelessASR",
        "--collect-all",
        "doubaoime_asr",
        "--collect-all",
        "sounddevice",
        str(launcher),
    ]
    completed = subprocess.run(command, cwd=ROOT, check=False)
    if completed.returncode == 0:
        suffix = ".app" if sys.platform == "darwin" else ""
        print(f"Build complete: {ROOT / 'dist' / f'TypelessASR{suffix}'}")
    return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main())

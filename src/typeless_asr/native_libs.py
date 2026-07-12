from __future__ import annotations

import ctypes.util
import os
import sys
from pathlib import Path


def configure_native_libraries() -> None:
    """Expose Homebrew's keg-only-style paths to ctypes on macOS."""
    if sys.platform != "darwin" or ctypes.util.find_library("opus") is not None:
        return

    candidates = (
        Path("/opt/homebrew/opt/opus/lib"),
        Path("/usr/local/opt/opus/lib"),
    )
    for candidate in candidates:
        if (candidate / "libopus.dylib").exists():
            existing = os.environ.get("DYLD_LIBRARY_PATH")
            os.environ["DYLD_LIBRARY_PATH"] = (
                f"{candidate}{os.pathsep}{existing}" if existing else str(candidate)
            )
            return

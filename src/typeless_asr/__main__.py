from __future__ import annotations

import argparse
import sys

from . import __version__
from .config import socket_path
from .ipc import send_command


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="macOS / Linux 语音输入")
    commands = parser.add_mutually_exclusive_group()
    commands.add_argument("--toggle", action="store_const", const="toggle", dest="command")
    commands.add_argument("--start", action="store_const", const="start", dest="command")
    commands.add_argument("--stop", action="store_const", const="stop", dest="command")
    commands.add_argument("--show", action="store_const", const="show", dest="command")
    commands.add_argument("--quit", action="store_const", const="quit", dest="command")
    parser.add_argument("--version", action="version", version=f"%(prog)s {__version__}")
    return parser


def main() -> int:
    args = build_parser().parse_args()
    if args.command:
        if send_command(socket_path(), args.command):
            return 0
        print("Typeless ASR 尚未运行，请先执行 `typeless-asr`。", file=sys.stderr)
        return 1

    if send_command(socket_path(), "show"):
        return 0

    from .app import run_gui

    return run_gui()


if __name__ == "__main__":
    raise SystemExit(main())

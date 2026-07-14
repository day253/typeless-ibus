English | [中文](README_zh.md)

# typeless-ibus

typeless-ibus is a native voice input method for Ubuntu and Linux. It writes speech recognition
results directly into the focused text field through IBus, so it works on GNOME Wayland without
clipboard injection, simulated paste, or X11.

The product focuses on one workflow: hold a key, speak, release it, and get text in the app you are
already using.

## Highlights

- **Native input method**: interim results appear as preedit text and final results are committed by IBus.
- **Hold to talk**: hold `Fn` to record and release it to stop, with toggle mode and alternative keys available.
- **Built for Wayland**: text input uses IBus D-Bus interfaces instead of keyboard or paste simulation.
- **System-native controls**: change the trigger key and recording mode from the IBus input-source menu, with English and Chinese labels selected from the system locale.
- **Small Rust codebase**: the engine is written in Rust, with no GUI toolkit, Python runtime, or LLM.
- **Broad Linux packaging**: native `.deb` builds cover Ubuntu 20.04 through 26.04, while the Nix Flake supports x86_64 and aarch64 Linux.
- **Automatic recovery**: rejected ASR service discovery credentials are refreshed while the current audio is replayed.

## How it feels

1. Add `typeless-ibus` from Ubuntu **Settings → Keyboard → Input Sources**.
2. Switch to `typeless-ibus` and focus any text field.
3. Hold `Fn` and speak.
4. Release `Fn`; the recognized text is inserted into the focused app.

Press `Esc` to cancel the active recording or recognition session.

## Scope

The current release targets Ubuntu/Linux and IBus 1.5.22 or newer. IBus remains the only input-method
backend; the project does not ship separate GTK, Qt, XIM, Wayland, or Fcitx5 frontends. It also does
not include Windows or macOS clients, LLM rewriting, accounts, cloud quotas, history, or dictionary
features.

The speech protocol implementation references
[`yangmoling/doubaoime-asr`](https://github.com/yangmoling/doubaoime-asr), and the product interaction
was inspired by [`tover0314-w/opentypeless`](https://github.com/tover0314-w/opentypeless).

## Documentation

Detailed documentation is currently maintained in Chinese:

- [Installation and removal](docs/installation.md)
- [Usage and configuration](docs/usage.md)
- [Architecture and design](docs/architecture.md)
- [Development and validation](docs/development.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Data, privacy, and risks](docs/privacy.md)
- [Documentation index](docs/README.md)
- [Changelog](CHANGELOG.md)

This project is released under the [MIT License](LICENSE).

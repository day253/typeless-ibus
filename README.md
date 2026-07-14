English | [中文](README_zh.md)

# Typeless IBus

Typeless IBus is a native voice input method for Ubuntu and Linux. It writes speech recognition
results directly into the focused text field through IBus, so it works on GNOME Wayland without
clipboard injection, simulated paste, or X11.

The product focuses on one workflow: hold a key, speak, release it, and get text in the app you are
already using.

## Highlights

- **Native input method**: interim results appear as preedit text and final results are committed by IBus.
- **Hold to talk**: hold `Fn` to record and release it to stop, with toggle mode and alternative keys available.
- **Built for Wayland**: text input uses IBus D-Bus interfaces instead of keyboard or paste simulation.
- **System-native controls**: change the trigger key and recording mode from the IBus input-source menu.
- **Small Rust codebase**: the engine is written in Rust, with no GUI toolkit, Python runtime, or LLM.
- **Automatic recovery**: rejected ASR service discovery credentials are refreshed while the current audio is replayed.

## How it feels

1. Add `Typeless Voice` from Ubuntu **Settings → Keyboard → Input Sources**.
2. Switch to `Typeless Voice` and focus any text field.
3. Hold `Fn` and speak.
4. Release `Fn`; the recognized text is inserted into the focused app.

Press `Esc` to cancel the active recording or recognition session.

## Scope

The current release targets Ubuntu/Linux, GNOME Wayland, and IBus only. It does not include Windows
or macOS clients, LLM rewriting, accounts, cloud quotas, history, or dictionary features.

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

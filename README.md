English | [中文](README_zh.md) · [Product website](https://day253.github.io/typeless-ibus/)

# typeless-ibus

**[See typeless-ibus in action →](https://day253.github.io/typeless-ibus/)**

typeless-ibus is a native IBus voice input method for Linux. It writes speech recognition results
directly into the focused text field, including on GNOME Wayland, without clipboard injection,
simulated paste, or X11.

The product focuses on one workflow: hold a key, speak, release it, and get text in the app you are
already using.

## Highlights

- **Native input method**: interim results appear as preedit text and final results are committed by IBus.
- **Hold to talk**: hold `Fn` to record and release it to stop, with toggle mode and alternative keys available.
- **Built for Wayland**: text input uses IBus D-Bus interfaces instead of keyboard or paste simulation.
- **System-native controls**: change the trigger key and recording mode from the IBus input-source menu, with English and Chinese labels selected from the system locale.
- **Small Rust codebase**: the engine is written in Rust, with no GUI toolkit, Python runtime, or LLM.
- **Zero-config speech recognition**: Doubao is the default provider and automatically obtains its
  own credentials; no account or API key is required. JSON configuration can instead select
  OpenAI, Groq, OpenRouter, SiliconFlow, Zhipu, ElevenLabs, Xiaomi MiMo, Alibaba Cloud Model
  Studio, Volcengine, or another OpenAI-compatible ASR endpoint.
- **Broad Linux support**: native amd64/arm64 `.deb` builds cover Ubuntu 20.04–26.04 and Debian
  11 Bullseye, 12 Bookworm, and 13 Trixie; native x86_64/aarch64 `.rpm` builds cover Fedora 43/44
  and openSUSE Tumbleweed; Arch Linux is protocol-tested; Nix covers both Linux architectures.
- **Automatic recovery**: rejected Doubao service-discovery credentials are refreshed while the
  current audio is replayed.

## How it feels

1. Add `typeless-ibus` from your desktop's input-source settings. On GNOME, open
   **Settings → Keyboard → Input Sources**.
2. Switch to `typeless-ibus` and focus any text field.
3. Hold `Fn` and speak.
4. Release `Fn`; the recognized text is inserted into the focused app.

Press `Esc` to cancel the active recording or recognition session.

## Scope

The current release targets Linux distributions with IBus 1.5.22 or newer. IBus remains the only
input-method backend; integrations supplied by each distribution connect it to GTK, Qt, XIM, and
Wayland applications. The project does not ship separate frontends or Fcitx5 support. It also does
not include Windows or macOS clients, LLM rewriting, accounts, cloud quotas, history, or dictionary
features.

## Documentation

Detailed documentation is maintained in Chinese. Start with the [documentation index](docs/README.md)
for installation, usage, provider configuration, troubleshooting, architecture, and development.
Release changes are recorded in the [changelog](CHANGELOG.md).

This project is released under the [MIT License](LICENSE).

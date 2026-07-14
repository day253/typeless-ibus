# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-07-15

### Added

- Initial native IBus voice input release for Ubuntu/Linux and GNOME Wayland.
- Project, Debian package, command, and input-source display names unified as `typeless-ibus`.
- Hold-to-talk and toggle recording modes with configurable `Fn`, Control, function-key, Space,
  and XKB keysym triggers.
- ALSA/cpal microphone capture, 16 kHz mono PCM processing, Opus encoding, and Doubao IME ASR.
- IBus preedit updates and direct `CommitText` output to the focused application.
- Native IBus property menus for changing the trigger key and recording mode without a GUI toolkit.
- English and Chinese IBus property labels selected automatically from the desktop locale.
- JSON configuration with Debian package and user-level installation support.
- IBus 1.5.22 compatibility with native Ubuntu 20.04, 22.04, 24.04, and 26.04 protocol-tested Debian builds.
- Nix Flake packages for x86_64-linux and aarch64-linux.
- ASR handshake and real-audio diagnostics with `x-tt-logid` logging.
- Automatic credential recovery and buffered audio replay after `service discovery failure`.
- Ubuntu CI for formatting, Clippy, tests, release builds, Debian packaging, and a separate
  non-blocking live ASR availability check.

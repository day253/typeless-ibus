# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- A Rust ASR provider interface that isolates IBus sessions from vendor-specific protocols.
- Opt-in cloud providers for OpenAI-compatible transcription, OpenAI Whisper, Groq, OpenRouter,
  SiliconFlow, Zhipu, ElevenLabs Scribe, Xiaomi MiMo, Alibaba Cloud Model Studio classic
  realtime/Qwen3 Realtime/Fun-ASR-Flash, and Volcengine SAUC.
- Provider-specific endpoint, model, API key, resource, language, and prompt configuration fields.
- Local mock coverage for every HTTP and WebSocket wire protocol, plus configuration compatibility,
  provider routing, WAV encoding, long-audio splitting, and request ID handling.
- Daily local JSONL logs with seven-file retention, final recognition transcripts, per-session UUIDs,
  request correlation, and the application context exposed by IBus without storing recorded audio.
- The recording prompt now shows the actual microphone in IBus auxiliary text and reports a missing
  audio input device separately. The preedit now keeps a muted gray `…` after the current
  uncommitted transcript, replacing the `Listening…`/`聆听中…` labels.

### Changed

- Doubao remains the zero-configuration default when `asr` is absent, including automatic initial
  credential acquisition and service-discovery credential recovery.
- ASR diagnostics and audio fixtures now exercise the provider selected by the JSON configuration.
- Volcengine SAUC now uses the latest single `apiKey` authentication.
- The product website now presents the supported cloud ASR providers and a prominent typeless logo.
- Every cloud provider now accepts a minimal `provider + apiKey` configuration and resolves all
  other supported fields through runtime defaults; the unused `vocabularyId` field was removed.
- Documentation is now organized by user task, and the default protocol implementation reference
  lives with the `doubao` provider documentation instead of the product overview.
- Speech language defaults now follow the system locale and time zone, prefer Chinese for an
  English-default system in a China time zone, and omit hints for providers that cannot accept them.

## [0.5.3] - 2026-07-18

### Added

- An interactive bilingual product website deployed through GitHub Pages, with a CSS-drawn Ubuntu
  voice-input walkthrough and direct links from the project README.

### Changed

- The product website version is now generated from `Cargo.toml` during the GitHub Pages build.
- The website hero now focuses on the Rust implementation and direct input without clipboard
  injection.

## [0.5.2] - 2026-07-17

### Changed

- The waiting preedit now uses compact localized text: `Listening…` in English and `聆听中…` in
  Chinese.

## [0.5.1] - 2026-07-16

### Fixed

- Long dictation now preserves confirmed text across ASR/VAD segments; finishing, timeout, and
  recoverable ASR failures retain the latest visible transcript instead of clearing it.

### Added

- Native arm64 Debian/Ubuntu packages and aarch64 Fedora/openSUSE packages, built and protocol-tested
  on GitHub-hosted ARM64 runners.

### Changed

- The IBus input source now appears as `Typeless Ibus` with the compact `听` status symbol instead
  of the raw package name and `vox`.
- The default maximum recording duration is now 600 seconds (10 minutes).
- Ubuntu ARM CI now keeps each official container image's default APT repository configuration and
  retries dependency installation without substituting a custom mirror.
- Debian jobs and release assets now identify Debian 11 Bullseye, 12 Bookworm, and 13 Trixie by
  both version and official codename.
- Tagged releases now attach 14 DEBs, six binary RPMs, and one architecture-independent SRPM per
  RPM distribution.

## [0.5.0] - 2026-07-15

### Added

- Distribution-native RPM and SRPM builds for Fedora 43/44 and openSUSE Tumbleweed.
- Automatic RPM and SRPM attachment to versioned GitHub Releases.

### Changed

- Fedora and openSUSE CI now builds the native package, installs it, exercises the IBus protocol,
  verifies clean removal, and uploads the resulting artifacts.
- Distribution and installation documentation now covers native RPM packages and SRPM rebuilding.

## [0.4.0] - 2026-07-15

### Added

- Initial native IBus voice input release for Linux, with GNOME Wayland as the primary desktop target.
- Project, package, command, and input-source display names unified as `typeless-ibus`.
- Hold-to-talk and toggle recording modes with configurable `Fn`, Control, function-key, Space,
  and XKB keysym triggers.
- ALSA/cpal microphone capture, 16 kHz mono PCM processing, Opus encoding, and Doubao IME ASR.
- IBus preedit updates and direct `CommitText` output to the focused application.
- Native IBus property menus for changing the trigger key and recording mode without a GUI toolkit.
- English and Chinese IBus property labels selected automatically from the desktop locale.
- JSON configuration with native `.deb`, Nix, and user-level source installation support.
- IBus 1.5.22 compatibility with native Ubuntu 20.04–26.04 and Debian 11–13 protocol-tested `.deb` builds.
- IBus protocol compatibility tests for Arch Linux.
- Nix Flake packages for x86_64-linux and aarch64-linux.
- ASR handshake and real-audio diagnostics with `x-tt-logid` logging.
- Automatic credential recovery and buffered audio replay after `service discovery failure`.
- Multi-distribution CI for formatting, Clippy, tests, release builds, native `.deb` packaging,
  IBus protocol coverage, Nix builds, and a separate non-blocking live ASR availability check.

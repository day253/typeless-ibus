# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

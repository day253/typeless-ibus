# Project execution rules

## Required deployment verification

- After every completed implementation change set, deploy the resulting Linux build to `tiger@192.168.3.85`.
- Do not treat a change as complete after local checks alone. It must be started and exercised on the target Ubuntu machine.
- For IBus changes, verify at minimum that the user IBus service is active, the `typeless` engine is registered, the installed process starts, and the changed interface or behavior is observable on the target machine.
- For settings UI changes, launch the installed settings executable inside the active graphical session and verify that the process and window are created without runtime errors.
- Preserve the target user's existing `~/.config/typeless-ibus/config.json` and ASR credentials during deployment unless the task explicitly changes them.
- Use the repository's user-level installer when sudo is unavailable, then restart or refresh IBus as required.

## Platform and implementation scope

- The product targets Ubuntu/Linux with GNOME Wayland and IBus.
- Product runtime code must not depend on Python.
- Prefer Rust for the IBus engine and native GTK settings components.

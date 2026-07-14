# Project execution rules

## Required deployment verification

- After every completed implementation change set, deploy the resulting Linux build to `tiger@192.168.3.85`.
- Do not treat a change as complete after local checks alone. It must be started and exercised on the target Linux machine.
- For IBus changes, verify at minimum that the user IBus service is active, the `typeless` engine is registered, the installed process starts, and the changed interface or behavior is observable on the target machine.
- For IBus settings-menu changes, trigger `RegisterProperties` inside the active graphical session and verify that the expected localized properties are observable without D-Bus errors.
- Preserve the target user's existing `~/.config/typeless-ibus/config.json` and ASR credentials during deployment unless the task explicitly changes them.
- Use the repository's user-level installer when sudo is unavailable, then restart or refresh IBus as required.

## Platform and implementation scope

- The product targets Linux distributions with IBus 1.5.22 or newer; GNOME Wayland is the primary desktop validation environment.
- Product runtime code must not depend on Python.
- Settings use IBus properties, the Rust CLI, and JSON; do not add a GTK/Qt settings program.
- Do not add Fcitx5 or separate GTK, Qt, XIM, or direct Wayland input-method frontends unless the user explicitly changes the product scope.

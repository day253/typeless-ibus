# Third-party notices

## OpenTypeless

[OpenTypeless](https://github.com/tover0314-w/opentypeless) was used as a design
reference for the Tauri desktop architecture, audio capture boundary, global
shortcut flow, tray behavior, and platform output fallback strategy.

OpenTypeless is available under the MIT License. This repository is a smaller,
independent implementation and does not include its LLM, account, cloud quota,
history, dictionary, or provider framework.

## Doubao IME ASR protocol reference

The Rust interoperability client follows protocol information publicly shown by
[`yangmoling/doubaoime-asr`](https://github.com/yangmoling/doubaoime-asr), an
unofficial client based on analysis of the Doubao IME Android application.

The upstream repository did not contain a license file at the reviewed revision
(`267972f815f519fd7c6149f85a8b7cc99daf61a5`). Its Python source and package are
not copied, vendored, linked, or executed by this project. The Rust code in this
repository independently implements the documented wire messages and service
interaction needed for interoperability.

The protocol and service may change at any time. Review the upstream project,
applicable service terms, and local law before use or redistribution.

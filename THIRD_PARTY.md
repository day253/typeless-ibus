# Third-party notices

## OpenTypeless

[OpenTypeless](https://github.com/tover0314-w/opentypeless) was used as a product
and interaction reference for voice input. typeless-ibus is an independent,
Linux-only implementation and does not include OpenTypeless's LLM, account,
cloud quota, history, dictionary, provider framework, Tauri UI, or source code.

OpenTypeless is available under the MIT License.

## Doubao IME ASR protocol reference

The Rust interoperability client follows protocol information publicly shown by
[`yangmoling/doubaoime-asr`](https://github.com/yangmoling/doubaoime-asr), an
unofficial client based on analysis of the Doubao IME Android application.

The upstream repository did not contain a license file at the reviewed revision
(`267972f815f519fd7c6149f85a8b7cc99daf61a5`). Its Python source and package are
not copied, vendored, linked, or executed by this project. The Rust code in this
repository independently implements the wire messages and service interaction
needed for interoperability.

The protocol and service may change at any time. Review the upstream project,
applicable service terms, and local law before use or redistribution.

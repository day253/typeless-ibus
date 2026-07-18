# Third-party notices

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

## OpenLess cloud ASR protocol reference

The cloud-provider separation and protocol interoperability behavior for Volcengine, Alibaba Cloud
Model Studio, Xiaomi MiMo, ElevenLabs, OpenRouter, and OpenAI-compatible endpoints were informed by
[`Open-Less/openless`](https://github.com/Open-Less/openless) at revision
`07f394f5a2efddebc9cb403a60329bd849addec7`.

OpenLess is licensed under the MIT License:

> Copyright (c) 2026 OpenLess contributors
>
> Permission is hereby granted, free of charge, to any person obtaining a copy of this software and
> associated documentation files (the "Software"), to deal in the Software without restriction,
> including without limitation the rights to use, copy, modify, merge, publish, distribute,
> sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is
> furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in all copies or
> substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT
> NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
> NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
> DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT
> OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

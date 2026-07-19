# typeless-ibus 文档

这里是 typeless-ibus 的安装、配置、ASR 与 LLM 供应商、排障、设计和开发文档。

typeless-ibus 是面向 Linux 的原生 IBus 语音输入法：按住触发键说话，松开后把识别结果
直接提交到当前输入框。

## 快速开始

1. [确认发行版支持范围](distributions.md)。
2. [安装与卸载](installation.md)。
3. [使用与配置](usage.md)。
4. [选择 ASR 供应商](asr/README.md)。默认 `doubao` 不需要 API Key。
5. [可选启用 LLM 润色](llm/README.md)。没有 `llm` 配置时保持关闭。

## 需要帮助

- [故障排查](troubleshooting.md)
- [本地日志](logging.md)
- [数据、隐私与风险](privacy.md)

## 项目资料

- [架构与设计](architecture.md)
- [ASR 供应商设计](asr-providers.md)
- [LLM 润色供应商](llm/README.md)
- [开发与验证](development.md)
- [第三方说明](THIRD_PARTY.md)

[产品主页](https://day253.github.io/typeless-ibus/) ·
[中文 README](https://github.com/day253/typeless-ibus/blob/main/README_zh.md) ·
[English README](https://github.com/day253/typeless-ibus/blob/main/README.md)

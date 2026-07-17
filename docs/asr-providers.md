# ASR 供应商

## 当前支持

| 配置值 | 模式 | 凭据 | 状态 |
| --- | --- | --- | --- |
| `doubao` | 实时 WebSocket，PCM 转 Opus | 自动获取并刷新 | 默认，零配置 |
| `openai-compatible` | 录音结束后 multipart HTTP，PCM 转 WAV | 可选 Bearer API Key | 显式配置 |

`openai-compatible` 面向实现标准 `audio/transcriptions` 请求和 `{ "text": "..." }`
响应的服务。同一个适配器可以连接官方接口、自建兼容服务或其他兼容厂商，而不用在引擎中
逐个写品牌分支。

## OpenLess 调研

调研 [OpenLess](https://github.com/Open-Less/openless) 当前 `beta` 分支时，其桌面 Rust
代码包含以下云端协议：

- 火山引擎流式 ASR。
- OpenAI Whisper-compatible 批量转写。
- 阿里云百炼经典实时 ASR、Qwen3-ASR-Flash Realtime、Fun-ASR-Flash 文件识别。
- 小米 MiMo 音频理解接口。
- ElevenLabs Speech-to-Text。

另外包含平台或本地实现：macOS 的 Qwen3-ASR 和 Apple Speech，以及 Windows 的
Foundry Local Whisper 和 sherpa-onnx。它们依赖平台框架、模型 runtime 或大体积模型，
不适合直接带入当前轻量 Linux IBus 项目。

OpenLess 的关键设计参考价值是：录音端统一输出 PCM，供应商在边界之后自行选择流式或
批量、音频编码、认证和生命周期。typeless-ibus 采用相同方向，但重新实现为更小的
`AsrProvider` trait，不复制 OpenLess 代码，也不依赖其 Tauri、平台 UI 或本地模型栈。

## 扩展约束

新增 provider 应满足：

1. 只接收统一的 16 kHz、单声道、16-bit little-endian PCM channel。
2. 流式服务可发送 `Partial`；批量服务至少发送一次非空 `Final`。
3. 供应商字段放在 `asr` 配置中，并由 `provider` 显式选择，不能根据环境变量隐式切换。
4. API Key、Token 和音频不能写入日志；若上游返回请求 ID，应记录脱敏后的 ID。
5. 使用 Rust 实现并为协议封装、响应解析和错误分支提供本地测试。
6. 默认 `doubao` 的零配置行为和凭据恢复必须保持兼容。

下一步如果继续增加非兼容协议，优先级建议是 ElevenLabs（批量接口简单）、百炼或火山引擎
（能恢复实时 preedit）。本地模型会显著增加包体、CPU/内存需求和架构矩阵，应作为独立
可选 feature，而不是默认运行时依赖。

[返回文档索引](README.md)

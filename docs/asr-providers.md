# ASR 供应商

[文档首页](index.md) · [供应商配置](asr/README.md) · [语种选择](languages.md)

账号开通、凭据获取、可复制 JSON 和真实音频验证命令见
[ASR 供应商配置索引](asr/README.md)。该索引按供应商拆分为独立文档；本页只说明协议
范围与实现边界。

## 当前支持

| 配置值 | 模式 | 凭据 | 状态 |
| --- | --- | --- | --- |
| [`doubao`](asr/doubao.md) | 实时 WebSocket，PCM 转 Opus | 自动获取并刷新 | 默认，零配置 |
| [`openai-compatible`](asr/openai-compatible.md) | 录音结束后 multipart HTTP，PCM 转 WAV | 可选 Bearer API Key | 显式配置 |
| [`whisper`](asr/openai.md) | OpenAI Audio Transcriptions multipart | `apiKey` | 显式配置 |
| [`groq`](asr/groq.md) | Groq Audio Transcriptions multipart | `apiKey` | 显式配置 |
| [`openrouter`](asr/openrouter.md) | Audio Transcriptions JSON + base64 WAV | `apiKey` | 显式配置 |
| [`siliconflow`](asr/siliconflow.md) | Audio Transcriptions multipart | `apiKey` | 显式配置 |
| [`zhipu`](asr/zhipu.md) | Audio Transcriptions multipart | `apiKey` | 显式配置 |
| [`elevenlabs`](asr/elevenlabs.md) | Scribe multipart HTTP | `apiKey` | 显式配置 |
| [`xiaomi-mimo-asr`](asr/xiaomi-mimo.md) | `chat/completions` 音频 JSON | `apiKey` | 显式配置 |
| [`bailian`](asr/alibaba-bailian.md) | DashScope 经典双工 WebSocket | `apiKey` | 显式配置 |
| [`bailian-qwen3-realtime`](asr/alibaba-bailian.md) | Qwen3 Realtime WebSocket | `apiKey` | 显式配置 |
| [`bailian-fun-asr-flash`](asr/alibaba-bailian.md) | DashScope 多模态批量 HTTP | `apiKey` | 显式配置 |
| [`volcengine`](asr/volcengine.md) | SAUC 大模型流式 WebSocket | `apiKey` | 显式配置 |

`openai-compatible` 面向实现标准 multipart `audio/transcriptions` 请求和
`{ "text": "..." }` 响应的服务。同一个适配器可以连接其他兼容厂商或自定义云端服务。
OpenAI、Groq、硅基流动和智谱提供同协议的品牌预设；OpenRouter 的请求体是 JSON，因此
使用独立编码分支。

`doubao`、`bailian`、`bailian-qwen3-realtime` 和 `volcengine` 可以在录音期间返回
partial；其余 provider 在松开触发键后提交整段音频。MiMo 和 Fun-ASR-Flash 按 180 秒
切片；OpenRouter 和智谱按 30 秒切片，逐段识别后按中英文与标点边界重新拼接。

## OpenLess 调研

调研 [OpenLess](https://github.com/Open-Less/openless) 当前 `beta` 分支时，其桌面 Rust
代码包含以下云端协议，当前已全部接入 typeless-ibus：

- 火山引擎流式 ASR。
- OpenAI Whisper、Groq、硅基流动、智谱等 multipart 批量转写，以及 OpenRouter JSON
  音频转写。
- 阿里云百炼经典实时 ASR、Qwen3-ASR-Flash Realtime、Fun-ASR-Flash 文件识别。
- 小米 MiMo 音频理解接口。
- ElevenLabs Speech-to-Text。

OpenLess 另外包含平台或本地实现：macOS 的 Qwen3-ASR 和 Apple Speech，以及 Windows
的 Foundry Local Whisper 和 sherpa-onnx。本项目明确不接入这些实现，不下载本地模型，
也不增加平台 runtime。

OpenLess 的关键设计参考价值是：录音端统一输出 PCM，供应商在边界之后自行选择流式或
批量、音频编码、认证和生命周期。typeless-ibus 采用相同方向，使用较小的
`AsrProvider` trait，不依赖其 Tauri、平台 UI 或本地模型栈。协议实现的许可归属见
[第三方说明](THIRD_PARTY.md)。

## 扩展约束

新增 provider 应满足：

1. 只接收统一的 16 kHz、单声道、16-bit little-endian PCM channel。
2. 流式服务可发送 `Partial`；批量服务至少发送一次非空 `Final`。
3. 供应商字段放在 `asr` 配置中，并由 `provider` 显式选择，不能根据环境变量隐式切换。
4. API Key、Token 和音频不能写入日志；若上游返回请求 ID，应记录脱敏后的 ID。
5. 使用 Rust 实现并为协议封装、响应解析和错误分支提供本地测试。
6. 默认 `doubao` 的零配置行为和凭据恢复必须保持兼容。

当前范围只包含云端接口。本地模型会显著增加包体、CPU/内存需求和架构矩阵，不在项目
计划内。

[返回文档索引](index.md)

# ASR 供应商配置

typeless-ibus 当前提供 13 个 `provider` 配置值，覆盖 10 个具名云端供应商和一个
OpenAI-compatible 通用入口。每个供应商的账号、凭据和可复制配置分别记录在独立文档中：

| 供应商 | `provider` | 配置说明 |
| --- | --- | --- |
| 豆包输入法相关服务 | `doubao` | [零配置与自动凭据](doubao.md) |
| OpenAI-compatible 服务 | `openai-compatible` | [通用兼容接口](openai-compatible.md) |
| OpenAI | `whisper` | [OpenAI 配置](openai.md) |
| Groq | `groq` | [Groq 配置](groq.md) |
| OpenRouter | `openrouter` | [OpenRouter 配置](openrouter.md) |
| SiliconFlow（硅基流动） | `siliconflow` | [SiliconFlow 配置](siliconflow.md) |
| 智谱 AI | `zhipu` | [智谱配置](zhipu.md) |
| ElevenLabs | `elevenlabs` | [ElevenLabs 配置](elevenlabs.md) |
| 小米 MiMo | `xiaomi-mimo-asr` | [小米 MiMo 配置](xiaomi-mimo.md) |
| 阿里云百炼 | `bailian`、`bailian-qwen3-realtime`、`bailian-fun-asr-flash` | [百炼配置](alibaba-bailian.md) |
| 火山引擎 | `volcengine` | [火山引擎配置](volcengine.md) |

## 通用配置步骤

配置文件位于：

```text
~/.config/typeless-ibus/config.json
```

供应商文档中的 JSON 只展示需要合并或替换的 `asr` 对象。请保留已有的
`triggerKey`、`triggerMode`、`inputDevice` 和 `maxRecordingSeconds`。修改后确保只有当前
用户可以读取配置：

```bash
chmod 600 ~/.config/typeless-ibus/config.json
```

切换到其他输入源再切回 Typeless IBus，让新引擎实例读取配置。然后先检查字段，再用一段
16 kHz、单声道、16-bit little-endian PCM 音频做真实识别：

```bash
/usr/libexec/typeless-ibus-engine --check-asr
/usr/libexec/typeless-ibus-engine \
  --check-asr-audio /path/to/16k-mono-s16le.pcm
```

用户级安装请把命令路径换成 `~/.local/libexec/typeless-ibus-engine`。`--check-asr` 对多数
供应商只验证本地字段，不会上传音频；能确认账号、模型与接口真正可用的是
`--check-asr-audio`。

## 凭据安全

`apiKey`、`appKey` 和 `accessKey` 会以明文保存在本机配置文件中，但不会写入日志。
不要把真实配置提交到 Git，也不要把完整 Key 粘贴到 issue。遇到上游错误时可提供日志中的
请求 ID 或 `x-tt-logid`，无需提供密钥。

[返回 ASR 设计说明](../asr-providers.md) · [返回文档索引](../README.md)

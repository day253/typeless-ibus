# OpenAI-compatible 云端 ASR

`openai-compatible` 用于没有专用预设、但实现 OpenAI Audio Transcriptions multipart
协议的云端服务。目标接口必须接收 `file`、`model`，并返回至少包含
`{ "text": "..." }` 的 JSON。

## 需要从服务商获得什么

| typeless-ibus 字段 | 获取方法 |
| --- | --- |
| `endpoint` | 从服务商 API 文档复制完整的 Audio Transcriptions URL，不是 Chat Completions URL |
| `apiKey` | 在该服务商控制台创建；如果服务商明确不要求鉴权才可省略 |
| `model` | 从服务商的语音转写模型列表复制准确的 model ID |
| `language` | 可选；按服务商文档填写语言代码 |
| `prompt` | 可选；仅在服务商声明支持转写提示词时填写 |

不要仅凭“OpenAI-compatible”判断它支持语音；很多兼容服务只实现文本接口。

## 最小配置示例

只填写 `provider + apiKey` 时使用内置的 OpenAI Audio Transcriptions endpoint 和
`whisper-1` 模型：

```json
{
  "asr": {
    "provider": "openai-compatible",
    "apiKey": "replace-with-provider-api-key"
  }
}
```

如果目标接口明确无需鉴权，可以删除 `apiKey`；除此之外不要把空字符串当成省略字段。

## 最大配置示例

连接其他兼容服务时可以显式覆盖全部通用转写字段：

```json
{
  "asr": {
    "provider": "openai-compatible",
    "endpoint": "https://provider.example/v1/audio/transcriptions",
    "apiKey": "replace-with-provider-api-key",
    "model": "replace-with-transcription-model-id",
    "language": "zh",
    "prompt": "Linux 语音输入"
  }
}
```

`endpoint` 必须是完整 URL。`language` 和 `prompt` 只有在目标服务声明支持时才保留；若
目标服务不需要鉴权，删除 `apiKey` 整行以及相邻的多余逗号。

## 验证

```bash
/usr/libexec/typeless-ibus-engine --check-asr
/usr/libexec/typeless-ibus-engine \
  --check-asr-audio /path/to/16k-mono-s16le.pcm
```

如果返回 404，优先检查是否误填了服务商的 base URL，或者重复/遗漏了
`/audio/transcriptions`。如果返回 401/403，重新创建 Key 并检查它是否有音频模型权限。

[OpenAI Audio Transcriptions 协议参考](https://platform.openai.com/docs/api-reference/audio) ·
[返回供应商索引](README.md)

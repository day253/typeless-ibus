# Groq ASR

Groq 预设的 `provider` 为 `groq`，使用兼容 OpenAI Audio Transcriptions 的 multipart
接口。默认模型是 `whisper-large-v3-turbo`。

## 获取 API Key

1. 登录 [GroqCloud Console](https://console.groq.com/)。
2. 选择或创建 Project；Groq 的 Key、日志和用量按 Project 隔离。
3. 打开 [API Keys](https://console.groq.com/keys)，单击 **Create API Key**。
4. 复制生成的 Key。

模型与接口限制以 Groq 官方
[Speech to Text 文档](https://console.groq.com/docs/speech-to-text)为准。

## 配置

```json
{
  "asr": {
    "provider": "groq",
    "apiKey": "replace-with-groq-api-key",
    "language": "zh"
  }
}
```

默认 endpoint 是 `https://api.groq.com/openai/v1/audio/transcriptions`，默认 model 是
`whisper-large-v3-turbo`。需要更高精度时可按官方模型列表覆盖：

```json
{
  "asr": {
    "provider": "groq",
    "apiKey": "replace-with-groq-api-key",
    "model": "whisper-large-v3",
    "language": "zh"
  }
}
```

`language` 和 `prompt` 可选。`language` 使用 ISO-639-1 代码；已知语言时填写可减少识别
延迟。

## 验证

```bash
/usr/libexec/typeless-ibus-engine --check-asr
/usr/libexec/typeless-ibus-engine \
  --check-asr-audio /path/to/16k-mono-s16le.pcm
```

401 表示 Key 无效；403 要检查 Project 权限；413 表示上游文件限制。typeless-ibus 会把
录音转换为 16 kHz 单声道 WAV 后上传。

[返回供应商索引](README.md)

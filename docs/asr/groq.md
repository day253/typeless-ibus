# Groq ASR

[文档首页](../index.md) · [ASR 供应商](README.md) · Groq

Groq 预设的 `provider` 为 `groq`，使用兼容 OpenAI Audio Transcriptions 的 multipart
接口。默认模型是 `whisper-large-v3-turbo`。

## 获取 API Key

1. 登录 [GroqCloud Console](https://console.groq.com/)。
2. 选择或创建 Project；Groq 的 Key、日志和用量按 Project 隔离。
3. 打开 [API Keys](https://console.groq.com/keys)，单击 **Create API Key**。
4. 复制生成的 Key。

模型与接口限制以 Groq 官方
[Speech to Text 文档](https://console.groq.com/docs/speech-to-text)为准。

## 最小配置示例

```json
{
  "asr": {
    "provider": "groq",
    "apiKey": "replace-with-groq-api-key"
  }
}
```

## 最大配置示例

显式覆盖 endpoint、model、language 和 prompt：

```json
{
  "asr": {
    "provider": "groq",
    "endpoint": "https://api.groq.com/openai/v1/audio/transcriptions",
    "apiKey": "replace-with-groq-api-key",
    "model": "whisper-large-v3",
    "language": "zh",
    "prompt": "Linux 语音输入"
  }
}
```

默认 model 是 `whisper-large-v3-turbo`。未填写时，引擎自动发送系统推断出的 ISO-639-1
两字母代码；无法安全转换时省略并交给 Groq 自动识别。显式 `language` 可以覆盖默认提示。
覆盖模型时应从官方列表复制准确 ID。

## 验证

```bash
/usr/libexec/typeless-ibus-engine --check-asr
/usr/libexec/typeless-ibus-engine \
  --check-asr-audio /path/to/16k-mono-s16le.pcm
```

401 表示 Key 无效；403 要检查 Project 权限；413 表示上游文件限制。typeless-ibus 会把
录音转换为 16 kHz 单声道 WAV 后上传。

[返回供应商索引](README.md)

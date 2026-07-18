# OpenRouter ASR

OpenRouter 预设的 `provider` 为 `openrouter`。它使用专用 Speech-to-Text JSON 接口，音频
以 base64 WAV 发送；不要改成 `openai-compatible`。

## 获取 API Key 和模型 ID

1. 登录 [OpenRouter](https://openrouter.ai/)。
2. 打开 [API Keys](https://openrouter.ai/settings/keys)，创建 Key；可按需要设置额度上限。
3. 从 [Speech-to-Text 模型集合](https://openrouter.ai/collections/speech-to-text-models)
   选择模型并复制完整 slug。
4. 也可以按官方说明通过 Models API 的 `output_modalities=transcription` 过滤可用模型。

官方说明：[API Key 鉴权](https://openrouter.ai/docs/api/reference/authentication) ·
[Speech-to-Text](https://openrouter.ai/docs/guides/overview/multimodal/stt)

## 配置

```json
{
  "asr": {
    "provider": "openrouter",
    "apiKey": "replace-with-openrouter-api-key"
  }
}
```

当前默认 model 是 `openai/whisper-large-v3-turbo`，默认 endpoint 是
`https://openrouter.ai/api/v1/audio/transcriptions`。选择其他模型时显式覆盖：

```json
{
  "asr": {
    "provider": "openrouter",
    "apiKey": "replace-with-openrouter-api-key",
    "model": "openai/whisper-large-v3"
  }
}
```

模型目录会变化；遇到 `model not found` 时重新从 STT 集合复制 slug，不要猜模型名。
typeless-ibus 会把长录音按 30 秒切片后识别并合并。

## 验证

```bash
/usr/libexec/typeless-ibus-engine --check-asr
/usr/libexec/typeless-ibus-engine \
  --check-asr-audio /path/to/16k-mono-s16le.pcm
```

401/403 时检查 Key、余额和 Key 的额度限制；404 时检查模型 slug；429 时检查账户或上游
供应商的速率限制。

[返回供应商索引](README.md)

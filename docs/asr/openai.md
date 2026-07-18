# OpenAI ASR

OpenAI 预设的 `provider` 为 `whisper`。名称是为兼容项目早期配置保留的；`model` 不限于
`whisper-1`，也可以显式改成 OpenAI Audio Transcriptions 接口当前支持的其他转写模型。

## 获取 API Key

1. 登录 [OpenAI Platform](https://platform.openai.com/)。
2. 打开 [API Keys](https://platform.openai.com/api-keys)，创建一个新的 secret key。
3. 立即复制并安全保存 Key；不要将其提交到仓库。
4. 确认对应项目已有可用额度，并允许调用所选转写模型。

官方入口：[Developer quickstart](https://platform.openai.com/docs/quickstart) ·
[Audio Transcriptions API](https://platform.openai.com/docs/api-reference/audio)

## 配置

使用项目默认的 `whisper-1`：

```json
{
  "asr": {
    "provider": "whisper",
    "apiKey": "replace-with-openai-api-key",
    "language": "zh"
  }
}
```

要选择其他模型，增加 `model`，例如：

```json
{
  "asr": {
    "provider": "whisper",
    "apiKey": "replace-with-openai-api-key",
    "model": "gpt-4o-transcribe",
    "language": "zh"
  }
}
```

默认 endpoint 是 `https://api.openai.com/v1/audio/transcriptions`，通常不需要填写。
`language` 和 `prompt` 可选；使用非默认模型前请在官方
[模型目录](https://developers.openai.com/api/docs/models)确认它支持 Audio Transcriptions
以及这些可选参数。

## 验证

```bash
/usr/libexec/typeless-ibus-engine --check-asr
/usr/libexec/typeless-ibus-engine \
  --check-asr-audio /path/to/16k-mono-s16le.pcm
```

401 通常表示 Key 无效；403 通常与项目权限有关；429 可能表示额度或速率限制。更换 Key
后切换一次输入源再测试。

[返回供应商索引](README.md)

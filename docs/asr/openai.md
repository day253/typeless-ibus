# OpenAI ASR

[文档首页](../index.md) · [ASR 供应商](README.md) · OpenAI

OpenAI 预设的 `provider` 为 `whisper`。名称是为兼容项目早期配置保留的；`model` 不限于
`whisper-1`，也可以显式改成 OpenAI Audio Transcriptions 接口当前支持的其他转写模型。

## 获取 API Key

1. 登录 [OpenAI Platform](https://platform.openai.com/)。
2. 打开 [API Keys](https://platform.openai.com/api-keys)，创建一个新的 secret key。
3. 立即复制并安全保存 Key；不要将其提交到仓库。
4. 确认对应项目已有可用额度，并允许调用所选转写模型。

官方入口：[Developer quickstart](https://platform.openai.com/docs/quickstart) ·
[Audio Transcriptions API](https://platform.openai.com/docs/api-reference/audio)

## 最小配置示例

使用内置 endpoint 和默认 `whisper-1` 模型，只需 `provider + apiKey`：

```json
{
  "asr": {
    "provider": "whisper",
    "apiKey": "replace-with-openai-api-key"
  }
}
```

## 最大配置示例

以下示例显式列出该适配器支持的全部覆盖字段：

```json
{
  "asr": {
    "provider": "whisper",
    "endpoint": "https://api.openai.com/v1/audio/transcriptions",
    "apiKey": "replace-with-openai-api-key",
    "model": "gpt-4o-transcribe",
    "language": "zh",
    "prompt": "Linux 语音输入"
  }
}
```

默认 endpoint 是 `https://api.openai.com/v1/audio/transcriptions`，通常不需要填写。
省略 `language` 时，引擎会把系统推断出的 ISO-639-1 两字母代码作为提示；无法安全转换时
省略并交给模型自动识别。显式 `language` 和 `prompt` 仍可覆盖；使用非默认模型前请在官方
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

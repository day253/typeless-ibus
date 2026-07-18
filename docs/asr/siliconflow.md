# SiliconFlow（硅基流动）ASR

SiliconFlow 预设的 `provider` 为 `siliconflow`，使用 Audio Transcriptions multipart
接口。中国站默认 endpoint 是 `https://api.siliconflow.cn/v1/audio/transcriptions`，默认
模型是 `FunAudioLLM/SenseVoiceSmall`。

## 获取 API Key

1. 登录 [SiliconFlow 控制台](https://cloud.siliconflow.cn/)。
2. 打开 [API 密钥](https://cloud.siliconflow.cn/account/ak)。
3. 单击“新建 API 密钥”，为 Key 填写便于识别的名称。
4. 创建后复制 Key，并确认账户有可用余额或额度。

官方入口：[快速上手](https://docs.siliconflow.cn/cn/userguide/quickstart) ·
[音频转写 API](https://docs.siliconflow.com/en/api-reference/audio/create-audio-transcriptions)

## 最小配置示例

```json
{
  "asr": {
    "provider": "siliconflow",
    "apiKey": "replace-with-siliconflow-api-key"
  }
}
```

## 最大配置示例

如果账号使用国际站，或需要覆盖全部通用转写字段：

```json
{
  "asr": {
    "provider": "siliconflow",
    "endpoint": "https://api.siliconflow.com/v1/audio/transcriptions",
    "apiKey": "replace-with-siliconflow-api-key",
    "model": "FunAudioLLM/SenseVoiceSmall",
    "language": "zh",
    "prompt": "Linux 语音输入"
  }
}
```

Key、endpoint 和模型必须属于同一站点；不要混用 `.cn` Key 与 `.com` endpoint。模型下线
或改名时，从控制台模型列表复制新的音频转写 model ID。`language` 和 `prompt` 只有在所选
模型声明支持时才保留。

## 验证

```bash
/usr/libexec/typeless-ibus-engine --check-asr
/usr/libexec/typeless-ibus-engine \
  --check-asr-audio /path/to/16k-mono-s16le.pcm
```

401 表示 Key 或站点不匹配；404 通常是 endpoint 或 model 错误；429 表示余额、并发或
速率限制。

[返回供应商索引](README.md)

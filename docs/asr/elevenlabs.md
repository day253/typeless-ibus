# ElevenLabs ASR

ElevenLabs 预设的 `provider` 为 `elevenlabs`，使用 Speech-to-Text multipart 接口，默认
模型是 `scribe_v2`。

## 获取 API Key

1. 登录 [ElevenLabs](https://elevenlabs.io/)。
2. 打开个人 API Keys 设置，创建新的 User API Key；团队生产环境也可以由管理员创建
   Service Account Key。
3. 给 Key 保留 Speech-to-Text 所需权限；如果设置 IP allowlist，应填写运行
   typeless-ibus 的公网出口 IP，而不是 `192.168.x.x` 私网地址。
4. 可选设置 credit quota，随后复制 Key。

官方入口：[API Key 管理与限制](https://elevenlabs.io/docs/overview/administration/workspaces/api-keys) ·
[Speech-to-Text API](https://elevenlabs.io/docs/api-reference/speech-to-text/convert)

## 最小配置示例

使用内置 endpoint、`scribe_v2` 和自动语言检测：

```json
{
  "asr": {
    "provider": "elevenlabs",
    "apiKey": "replace-with-elevenlabs-api-key"
  }
}
```

## 最大配置示例

显式列出 endpoint、model 和 language。ElevenLabs 使用 ISO-639-1 或 ISO-639-3 代码，
例如普通话可写 `zho`：

```json
{
  "asr": {
    "provider": "elevenlabs",
    "endpoint": "https://api.elevenlabs.io/v1/speech-to-text",
    "apiKey": "replace-with-elevenlabs-api-key",
    "model": "scribe_v2",
    "language": "zho"
  }
}
```

引擎会以 `xi-api-key` 请求头发送配置中的 `apiKey`，无需用户手动添加前缀。

## 验证

```bash
/usr/libexec/typeless-ibus-engine --check-asr
/usr/libexec/typeless-ibus-engine \
  --check-asr-audio /path/to/16k-mono-s16le.pcm
```

401 表示 Key 无效；403 常见于 Key scope 或 IP allowlist；429 表示 credit quota、账户额度
或速率限制。

[返回供应商索引](README.md)

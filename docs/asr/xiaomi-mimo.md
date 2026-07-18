# 小米 MiMo ASR

[文档首页](../README.md) · [ASR 供应商](README.md) · 小米 MiMo

小米 MiMo 预设的 `provider` 为 `xiaomi-mimo-asr`，使用 MiMo-V2.5-ASR 的
`chat/completions` 音频 JSON 协议。默认模型是 `mimo-v2.5-asr`。

## 获取 API Key

1. 使用小米账号登录 [MiMo API Open Platform](https://platform.xiaomimimo.com/)。
2. 在 Console 的 **API Keys** 页面创建按量付费 API Key。
3. 复制 `sk-` 开头的 Key；该 Key 只在创建时完整显示，请立即安全保存。
4. 确认账户余额可用于 ASR。MiMo ASR 按输入音频时长计费，实际价格以控制台为准。

官方入口：[First API Call 与 Key 获取](https://mimo.mi.com/docs/en-US/quick-start/summary/first-api-call) ·
[MiMo-V2.5-ASR 接口](https://mimo.mi.com/docs/en-US/api/audio/Speech-Recognition)

## 最小配置示例

```json
{
  "asr": {
    "provider": "xiaomi-mimo-asr",
    "apiKey": "replace-with-mimo-api-key"
  }
}
```

## 最大配置示例

显式列出当前适配器会读取的全部字段：

```json
{
  "asr": {
    "provider": "xiaomi-mimo-asr",
    "endpoint": "https://api.xiaomimimo.com/v1/chat/completions",
    "apiKey": "replace-with-mimo-api-key",
    "model": "mimo-v2.5-asr"
  }
}
```

默认 endpoint 是 `https://api.xiaomimimo.com/v1/chat/completions`。typeless-ibus 使用
官方支持的 `Authorization: Bearer` 鉴权，不需要添加 `api-key` 请求头。当前适配器不读取
`language` 或 `prompt`，因此最大配置也不包含这两个字段；模型自行识别语言。

MiMo Token Plan 的 Key 以 `tp-` 开头并使用独立 Base URL。只有在对应套餐明确包含
`mimo-v2.5-asr` 时才应使用，并把控制台提供的完整 Chat Completions URL 写入
`endpoint`；按量付费 `sk-` Key 不需要覆盖 endpoint。

## 验证

```bash
/usr/libexec/typeless-ibus-engine --check-asr
/usr/libexec/typeless-ibus-engine \
  --check-asr-audio /path/to/16k-mono-s16le.pcm
```

401 表示 Key 类型、Key 内容或 endpoint 不匹配；404 时检查是否误填了 Base URL 而不是
完整 `/v1/chat/completions`；429 时检查余额和速率限制。

[返回供应商索引](README.md)

# Anthropic Claude 润色

[LLM 供应商](README.md) · [Anthropic Messages API](https://platform.claude.com/docs/en/api/messages)

在 [Anthropic Console](https://console.anthropic.com/settings/keys) 创建 API Key。此 provider
直接使用 Anthropic Messages 协议，不经过 OpenRouter。

最小配置：

```json
{
  "llm": {
    "provider": "anthropic",
    "apiKey": "replace-me"
  }
}
```

完整配置：

```json
{
  "llm": {
    "enabled": true,
    "provider": "anthropic",
    "endpoint": "https://api.anthropic.com/v1/messages",
    "apiKey": "replace-me",
    "model": "claude-haiku-4-5",
    "mode": "smart",
    "style": "clean",
    "timeoutMs": 10000,
    "customPrompt": "表达自然、简洁"
  }
}
```

请求使用 `x-api-key` 与 `anthropic-version: 2023-06-01`。默认选择低延迟的 Haiku；需要
更强改写能力或账户不可用时，可显式改成该账户有权限的 Claude API ID。

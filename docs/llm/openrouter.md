# OpenRouter 润色

[LLM 供应商](README.md) · [OpenRouter API Keys](https://openrouter.ai/settings/keys) ·
[API 文档](https://openrouter.ai/docs/api/reference/overview)

最小配置：

```json
{"llm":{"provider":"openrouter","apiKey":"replace-me"}}
```

完整配置：

```json
{
  "llm": {
    "enabled": true,
    "provider": "openrouter",
    "endpoint": "https://openrouter.ai/api/v1/chat/completions",
    "apiKey": "replace-me",
    "model": "openai/gpt-5-mini",
    "mode": "smart",
    "style": "clean",
    "timeoutMs": 10000,
    "customPrompt": "表达自然、简洁"
  }
}
```

请求会附带 typeless-ibus 的 `HTTP-Referer` 和 `X-OpenRouter-Title`，不会发送窗口或文档信息。

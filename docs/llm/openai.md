# OpenAI 润色

[LLM 供应商](README.md) · [OpenAI API Keys](https://platform.openai.com/api-keys)

在 OpenAI Platform 创建 API Key。请求使用 Chat Completions compatible 协议。

最小配置：

```json
{"llm":{"provider":"openai","apiKey":"replace-me"}}
```

完整配置：

```json
{
  "llm": {
    "enabled": true,
    "provider": "openai",
    "endpoint": "https://api.openai.com/v1/chat/completions",
    "apiKey": "replace-me",
    "model": "gpt-5-mini",
    "mode": "smart",
    "style": "clean",
    "timeoutMs": 10000,
    "customPrompt": "表达自然、简洁"
  }
}
```

如果组织使用区域化 OpenAI endpoint，可以直接覆盖完整 `endpoint`。

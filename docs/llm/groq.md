# Groq 润色

[LLM 供应商](README.md) · [Groq API Keys](https://console.groq.com/keys) ·
[Groq 模型目录](https://console.groq.com/docs/models)

最小配置：

```json
{"llm":{"provider":"groq","apiKey":"replace-me"}}
```

完整配置：

```json
{
  "llm": {
    "enabled": true,
    "provider": "groq",
    "endpoint": "https://api.groq.com/openai/v1/chat/completions",
    "apiKey": "replace-me",
    "model": "qwen/qwen3.6-27b",
    "mode": "smart",
    "style": "clean",
    "timeoutMs": 10000,
    "customPrompt": "Use natural, concise wording"
  }
}
```

Groq 会下线旧模型；内置值避开了已公告进入下线周期的 Llama 3.3 默认项。若账户权限不同，
从模型目录复制可用 ID 覆盖 `model`。

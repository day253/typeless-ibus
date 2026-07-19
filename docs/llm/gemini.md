# Google Gemini 润色

[LLM 供应商](README.md) · [Google AI Studio API Key](https://aistudio.google.com/app/apikey) ·
[OpenAI compatibility](https://ai.google.dev/gemini-api/docs/openai)

最小配置：

```json
{"llm":{"provider":"gemini","apiKey":"replace-me"}}
```

完整配置：

```json
{
  "llm": {
    "enabled": true,
    "provider": "gemini",
    "endpoint": "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions",
    "apiKey": "replace-me",
    "model": "gemini-3.5-flash",
    "mode": "smart",
    "style": "clean",
    "timeoutMs": 10000,
    "customPrompt": "Use natural, concise wording"
  }
}
```

实现使用 Google 官方的 OpenAI compatibility endpoint，因此不增加单独的 Gemini SDK。

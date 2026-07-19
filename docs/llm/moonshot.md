# Moonshot / Kimi 润色

[LLM 供应商](README.md) · [Moonshot API Key](https://platform.moonshot.cn/console/api-keys)

最小配置：

```json
{"llm":{"provider":"moonshot","apiKey":"replace-me"}}
```

完整配置：

```json
{
  "llm": {
    "enabled": true,
    "provider": "moonshot",
    "endpoint": "https://api.moonshot.cn/v1/chat/completions",
    "apiKey": "replace-me",
    "model": "moonshot-v1-8k",
    "mode": "smart",
    "style": "clean",
    "timeoutMs": 10000,
    "customPrompt": "表达自然、简洁"
  }
}
```

若控制台为新账户提供了其他 Kimi 模型 ID，直接覆盖 `model` 即可。

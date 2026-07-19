# DeepSeek 润色

[LLM 供应商](README.md) · [DeepSeek API Keys](https://platform.deepseek.com/api_keys) ·
[DeepSeek 模型说明](https://api-docs.deepseek.com/quick_start/pricing)

最小配置：

```json
{"llm":{"provider":"deepseek","apiKey":"replace-me"}}
```

完整配置：

```json
{
  "llm": {
    "enabled": true,
    "provider": "deepseek",
    "endpoint": "https://api.deepseek.com/chat/completions",
    "apiKey": "replace-me",
    "model": "deepseek-v4-flash",
    "mode": "smart",
    "style": "clean",
    "timeoutMs": 10000,
    "customPrompt": "表达自然、简洁"
  }
}
```

默认选择速度和成本更适合短文本清理的 Flash 模型，并关闭思考模式以降低输入延迟；需要
其他能力时覆盖 `model`。

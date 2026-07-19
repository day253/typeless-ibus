# 阿里云通义千问润色

[LLM 供应商](README.md) · [百炼 API Key](https://bailian.console.aliyun.com/?apiKey=1) ·
[OpenAI-compatible 调用](https://help.aliyun.com/zh/model-studio/qwen-api-via-openai-chat-completions)

最小配置：

```json
{"llm":{"provider":"qwen","apiKey":"replace-me"}}
```

完整配置：

```json
{
  "llm": {
    "enabled": true,
    "provider": "qwen",
    "endpoint": "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions",
    "apiKey": "replace-me",
    "model": "qwen3.6-flash",
    "mode": "smart",
    "style": "clean",
    "timeoutMs": 10000,
    "customPrompt": "表达自然、简洁"
  }
}
```

默认是中国内地 endpoint。国际站 Key 需要使用对应区域的 compatible-mode endpoint，
不能与中国内地地址混用。

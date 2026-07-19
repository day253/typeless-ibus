# 自定义 OpenAI-compatible LLM

[LLM 供应商](README.md) · [本地日志](../logging.md)

使用实现了 `POST /chat/completions` 的云端兼容接口。该 provider 仍是云端 BYOK 配置；项目
不内置或管理本地模型运行时。

最小配置会使用 OpenAI 的 endpoint 和 `gpt-5-mini`，如果目标不是 OpenAI，应使用完整配置：

```json
{
  "llm": {
    "provider": "openai-compatible",
    "apiKey": "replace-me"
  }
}
```

```json
{
  "llm": {
    "enabled": true,
    "provider": "openai-compatible",
    "endpoint": "https://llm.example.com/v1/chat/completions",
    "apiKey": "replace-me",
    "model": "model-name",
    "mode": "smart",
    "style": "clean",
    "timeoutMs": 10000,
    "customPrompt": "表达自然、简洁"
  }
}
```

接口需要接受 Bearer Token、`model`、`messages` 与 `stream: false`，响应需要提供
`choices[0].message.content`。兼容接口返回的 `x-request-id`、`request-id`、`x-trace-id`、
`x-tt-logid` 或 `x-api-request-id` 会进入本地日志。

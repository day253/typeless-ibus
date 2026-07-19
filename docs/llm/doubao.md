# 豆包大模型润色

[LLM 供应商](README.md) · [火山方舟控制台](https://console.volcengine.com/ark/region:ark+cn-beijing/apiKey) ·
[Chat Completions](https://api.volcengine.com/api-docs/view?action=ChatCompletions&serviceCode=ark&version=2024-01-01)

这里的 `doubao` 是需要火山方舟 API Key 的文本大模型，与默认零配置的豆包 IME ASR
是两套独立接口和凭据。

最小配置：

```json
{"llm":{"provider":"doubao","apiKey":"replace-me"}}
```

完整配置：

```json
{
  "llm": {
    "enabled": true,
    "provider": "doubao",
    "endpoint": "https://ark.cn-beijing.volces.com/api/v3/chat/completions",
    "apiKey": "replace-me",
    "model": "doubao-seed-1-6-flash-250615",
    "mode": "smart",
    "style": "clean",
    "timeoutMs": 10000,
    "customPrompt": "表达自然、简洁"
  }
}
```

如果方舟项目要求使用推理接入点 ID 或更新的模型 ID，请把控制台显示的值写入 `model`。

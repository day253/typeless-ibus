# 硅基流动润色

[LLM 供应商](README.md) · [硅基流动 API Key](https://cloud.siliconflow.cn/account/ak)

最小配置：

```json
{"llm":{"provider":"siliconflow","apiKey":"replace-me"}}
```

完整配置：

```json
{
  "llm": {
    "enabled": true,
    "provider": "siliconflow",
    "endpoint": "https://api.siliconflow.cn/v1/chat/completions",
    "apiKey": "replace-me",
    "model": "Qwen/Qwen2.5-7B-Instruct",
    "mode": "smart",
    "style": "clean",
    "timeoutMs": 10000,
    "customPrompt": "表达自然、简洁"
  }
}
```

硅基流动模型目录变化较快；如果默认模型对新账户不可见，在模型广场选择可用文本模型并
只覆盖 `model`。

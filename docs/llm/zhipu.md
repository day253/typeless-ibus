# 智谱 GLM 润色

[LLM 供应商](README.md) · [智谱 API Key 管理](https://open.bigmodel.cn/usercenter/apikeys)

最小配置：

```json
{"llm":{"provider":"zhipu","apiKey":"replace-me"}}
```

完整配置：

```json
{
  "llm": {
    "enabled": true,
    "provider": "zhipu",
    "endpoint": "https://open.bigmodel.cn/api/paas/v4/chat/completions",
    "apiKey": "replace-me",
    "model": "glm-4-flash",
    "mode": "smart",
    "style": "clean",
    "timeoutMs": 10000,
    "customPrompt": "表达自然、简洁"
  }
}
```

这是智谱中国站的默认 endpoint；使用其他区域或 Z.AI 账户时应按对应控制台覆盖 endpoint
和 model，避免混用两个站点的 Key。

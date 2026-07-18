# 智谱 AI ASR

[文档首页](../README.md) · [ASR 供应商](README.md) · 智谱 AI

智谱预设的 `provider` 为 `zhipu`，默认模型是 `glm-asr-2512`，通过 multipart HTTP
上传 WAV。由于官方接口限制，typeless-ibus 会把长录音按 30 秒切片再合并结果。

## 获取 API Key

1. 注册并登录 [智谱开放平台](https://bigmodel.cn/)。
2. 打开 [API Keys](https://bigmodel.cn/usercenter/proj-mgmt/apikeys)。
3. 创建 API Key 并立即复制保存。
4. 确认账户有调用 `glm-asr-2512` 的权限和余额。

官方入口：[HTTP API 与 Key 获取步骤](https://docs.bigmodel.cn/cn/guide/develop/http/introduction) ·
[GLM-ASR-2512 模型说明](https://docs.bigmodel.cn/cn/guide/models/sound-and-video/glm-asr-2512)

## 最小配置示例

```json
{
  "asr": {
    "provider": "zhipu",
    "apiKey": "replace-with-zhipu-api-key"
  }
}
```

## 最大配置示例

```json
{
  "asr": {
    "provider": "zhipu",
    "endpoint": "https://open.bigmodel.cn/api/paas/v4/audio/transcriptions",
    "apiKey": "replace-with-zhipu-api-key",
    "model": "glm-asr-2512",
    "language": "zh",
    "prompt": "Linux 语音输入"
  }
}
```

一般不需要填写 `endpoint` 或 `model`。`language` 和 `prompt` 只有在当前模型接口声明支持时
才保留；引擎不会对智谱预设自动添加系统语种。不要把编码套餐的专属文本 endpoint 用于
ASR。

## 验证

```bash
/usr/libexec/typeless-ibus-engine --check-asr
/usr/libexec/typeless-ibus-engine \
  --check-asr-audio /path/to/16k-mono-s16le.pcm
```

官方模型单次上传限制为 25 MB、30 秒；typeless-ibus 已处理时长切片。401 时重新复制
API Key，429 时检查余额、套餐和速率限制。

[返回供应商索引](README.md)

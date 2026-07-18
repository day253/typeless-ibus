# 火山引擎 ASR

火山引擎预设的 `provider` 为 `volcengine`，使用 SAUC 大模型流式 WebSocket 协议，只
支持新版控制台的单 API Key 鉴权。

## 获取新版 API Key

1. 登录[火山引擎豆包语音控制台](https://console.volcengine.com/speech/new/setting/apikeys?projectName=default)。
2. 在新版控制台的 API Key 页面创建或复制 **APP Key**。
3. 确认项目已开通“豆包流式语音识别模型 2.0”。按时长计费通常使用默认 Resource ID
   `volc.seedasr.sauc.duration`；并发版等其他资源需要复制控制台实际显示的 Resource ID。

官方文档：[流式语音识别 WebSocket 鉴权](https://www.volcengine.com/docs/6561/1354869?lang=zh)

## 最小配置示例

按时长计费的模型 2.0 使用默认 Resource ID 时，只需配置 API Key：

```json
{
  "asr": {
    "provider": "volcengine",
    "apiKey": "replace-with-volcengine-app-key"
  }
}
```

程序会把 `apiKey` 写入 `X-Api-Key` 请求头，并自动发送默认 `resourceId` 和随机连接 ID。

## 最大配置示例

控制台显示其他资源或 endpoint 时，可以显式填写全部支持字段：

```json
{
  "asr": {
    "provider": "volcengine",
    "endpoint": "wss://openspeech.bytedance.com/api/v3/sauc/bigmodel_async",
    "apiKey": "replace-with-volcengine-app-key",
    "resourceId": "volc.seedasr.sauc.duration"
  }
}
```

默认 endpoint 是
`wss://openspeech.bytedance.com/api/v3/sauc/bigmodel_async`。只有官方文档为已开通资源给出
其他 WebSocket URL 时才覆盖 `endpoint`。该 provider 没有可配置的 `model` 字段。

## 验证

```bash
/usr/libexec/typeless-ibus-engine --check-asr
/usr/libexec/typeless-ibus-engine \
  --check-asr-audio /path/to/16k-mono-s16le.pcm
```

鉴权失败时先确认 API Key 和 Resource ID 属于同一个项目且服务已开通。`access denied`
通常是 Resource ID 未开通或不匹配。上游返回的 `x-tt-logid` 会写入 typeless-ibus 日志，
向火山引擎排查时提供该 ID，不要提供密钥。

[返回供应商索引](README.md)

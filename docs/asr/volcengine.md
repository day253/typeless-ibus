# 火山引擎 ASR

火山引擎预设的 `provider` 为 `volcengine`，使用 SAUC 大模型流式 WebSocket 协议。它不
使用通用 API Key，而是需要 APP ID、Access Token 和 Resource ID。

## 获取 APP ID、Access Token 和 Resource ID

1. 登录 [火山引擎豆包语音控制台](https://console.volcengine.com/speech/app)。
2. 创建应用，或打开现有应用，并在“开通管理”中开通“大模型流式语音识别”服务。
3. 在应用的 API 调用参数中复制 **APP ID** 和 **Access Token**。
4. 在所开通服务的 API 文档或控制台中确认 **Resource ID**。按时长计费的 SAUC 服务
   通常使用 `volc.seedasr.sauc.duration`，但应以账户实际开通项为准。

官方入口：[大模型流式识别 SDK 与鉴权字段](https://www.volcengine.com/docs/6561/1395846?lang=zh) ·
[语音识别 API 文档](https://www.volcengine.com/docs/6561/1354869?lang=zh)

## 字段对应关系

| 控制台/API 名称 | typeless-ibus 字段 | 请求头 |
| --- | --- | --- |
| APP ID / Appid | `appKey` | `X-Api-App-Key` |
| Access Token | `accessKey` | `X-Api-Access-Key` |
| Resource ID | `resourceId` | `X-Api-Resource-Id` |

这里的 `appKey` 必须填 APP ID，不是火山引擎 IAM Access Key ID；`accessKey` 必须填豆包
语音应用的 Access Token，不是 IAM Secret Access Key。

## 配置

使用默认 Resource ID：

```json
{
  "asr": {
    "provider": "volcengine",
    "appKey": "replace-with-app-id",
    "accessKey": "replace-with-access-token"
  }
}
```

控制台显示其他资源时显式填写：

```json
{
  "asr": {
    "provider": "volcengine",
    "appKey": "replace-with-app-id",
    "accessKey": "replace-with-access-token",
    "resourceId": "replace-with-resource-id"
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

鉴权失败时逐项核对 APP ID、Access Token 和 Resource ID 是否来自同一个应用与已开通
服务。`access denied` 通常是 Resource ID 未开通或不匹配。上游返回的 `x-tt-logid` 会被
写入 typeless-ibus 日志，向火山引擎排查时提供该 ID，不要提供密钥。

[返回供应商索引](README.md)

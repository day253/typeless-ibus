# 阿里云百炼 ASR

同一个阿里云百炼 API Key 可以配置 typeless-ibus 的三种协议。它们的延迟、模型与可选字段
不同，请只选择一个 `provider`：

| `provider` | 模式 | 默认模型 | 可选字段 |
| --- | --- | --- | --- |
| `bailian` | 经典双工 WebSocket，实时 partial | `fun-asr-realtime` | `vocabularyId` |
| `bailian-qwen3-realtime` | Qwen Realtime WebSocket，实时 partial | `qwen3-asr-flash-realtime` | `language` |
| `bailian-fun-asr-flash` | 录音结束后批量 HTTP | `fun-asr-flash-2026-06-15` | 无 |

## 获取 API Key、地域与 Workspace

1. 登录 [阿里云百炼控制台](https://bailian.console.aliyun.com/)。
2. 在右上角先选择要调用模型的地域。API Key、API Host、Workspace 和地域必须一致。
3. 打开 **API Key** 页面，单击“创建 API Key”。建议使用默认业务空间；需要限制时再配置
   可访问模型和公网 IP 白名单。
4. 创建后立即复制弹窗里的完整 API Key 和 API Host。新版 `sk-ws` Key 的明文只显示
   一次；丢失后需要重置或重新创建。
5. 使用子业务空间或控制台给出 workspace 专属域名时，同时在业务空间页面复制
   Workspace ID。

官方入口：[获取 API Key](https://help.aliyun.com/zh/model-studio/get-api-key) ·
[语音识别模型选择](https://help.aliyun.com/zh/model-studio/asr-model/)

## 方案一：Fun-ASR 经典实时

```json
{
  "asr": {
    "provider": "bailian",
    "apiKey": "replace-with-bailian-api-key"
  }
}
```

默认 endpoint 是 `wss://dashscope.aliyuncs.com/api-ws/v1/inference/`。要使用热词，在百炼
控制台创建热词表并复制 Vocabulary ID：

```json
{
  "asr": {
    "provider": "bailian",
    "apiKey": "replace-with-bailian-api-key",
    "vocabularyId": "replace-with-vocabulary-id"
  }
}
```

`vocabularyId` 不是 Workspace ID，也不是热词表名称。热词表的创建与适用模型以官方
[识别准确率说明](https://help.aliyun.com/zh/model-studio/non-realtime-speech-recognition-user-guide)
为准。

## 方案二：Qwen3 ASR Realtime

```json
{
  "asr": {
    "provider": "bailian-qwen3-realtime",
    "apiKey": "replace-with-bailian-api-key",
    "language": "zh"
  }
}
```

删除 `language` 可让模型自动判断。使用当前 workspace 专属域名时，按控制台地域填写完整
WebSocket URL，例如北京地域：

```json
{
  "asr": {
    "provider": "bailian-qwen3-realtime",
    "endpoint": "wss://replace-with-workspace-id.cn-beijing.maas.aliyuncs.com/api-ws/v1/realtime",
    "apiKey": "replace-with-bailian-api-key",
    "model": "qwen3-asr-flash-realtime",
    "language": "zh"
  }
}
```

新加坡等地域的域名不同，请从官方
[Qwen-ASR Realtime 交互流程](https://help.aliyun.com/en/model-studio/qwen-asr-realtime-interaction-process)
复制，不要只替换地域字符串。

## 方案三：Fun-ASR-Flash 批量识别

```json
{
  "asr": {
    "provider": "bailian-fun-asr-flash",
    "apiKey": "replace-with-bailian-api-key"
  }
}
```

默认 endpoint 使用中国大陆 DashScope multimodal-generation 地址。若创建 Key 时显示的
API Host 不同，请按同一地域的模型文档拼出完整 HTTP endpoint 后写入 `endpoint`。
typeless-ibus 会把超过 180 秒的录音切片后提交。

## 验证

无论选择哪种方案，都先运行：

```bash
/usr/libexec/typeless-ibus-engine --check-asr
/usr/libexec/typeless-ibus-engine \
  --check-asr-audio /path/to/16k-mono-s16le.pcm
```

401/403 时依次核对 Key 是否完整、Key 所属地域、Workspace 的模型权限和 IP 白名单。
404 或 WebSocket 握手失败时，重点核对当前协议的完整 endpoint；不要把 OpenAI-compatible
文本 Base URL 填到三个 ASR provider 中。

[返回供应商索引](README.md)

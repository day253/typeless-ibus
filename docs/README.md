# typeless-ibus 文档

这里保存产品主页之外的安装、配置、排障、实现和维护资料。按当前任务选择入口即可，
不需要从头顺序阅读。

[English product overview](../README.md) · [中文产品介绍](../README_zh.md) ·
[更新日志](../CHANGELOG.md)

## 开始使用

1. [确认发行版支持范围](distributions.md)：已验证版本、CPU 架构和兼容边界。
2. [安装与卸载](installation.md)：选择原生 `.deb`、`.rpm`、Nix 或用户级安装。
3. [使用与配置](usage.md)：添加输入源、长按说话、调整触发键和 JSON 配置。
4. [选择 ASR 供应商](asr/README.md)：复制最小配置，获取 API Key，需要时覆盖默认值。
5. [语种选择与回退](languages.md)：系统 locale、时区、provider 能力和手动覆盖优先级。

默认 `doubao` provider 不需要账号或 API Key；想直接开始使用可先阅读
[豆包零配置 ASR](asr/doubao.md)。

## 遇到问题

- [故障排查](troubleshooting.md)：按键、麦克风、IBus、ASR 握手和真实音频诊断。
- [数据、隐私与风险](privacy.md)：音频流向、本地凭据、日志与非官方协议风险。

## 了解和维护项目

- [架构与设计](architecture.md)：IBus、音频、输入会话、ASR 和故障恢复流程。
- [ASR 供应商设计](asr-providers.md)：协议适配器、实现边界与扩展约束。
- [开发与验证](development.md)：本地检查、打包、发行版矩阵和 GitHub Actions。
- [更新日志](../CHANGELOG.md)：每个版本的功能和兼容性变化。

[返回中文产品介绍](../README_zh.md)

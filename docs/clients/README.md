# 原生客户端设计

本目录存放**多邻灶原生客户端**的产品与技术设计，供 Android 与鸿蒙（HarmonyOS NEXT）实现共用。

| 文档 | 内容 |
|------|------|
| [design.md](design.md) | 完整设计：角色与用户逻辑、信息架构、与 Web API 对齐、包内服务端 URL 与自托管分发；文末为平台差异 |
| [../requirements-phase1.md](../requirements-phase1.md) | 一期需求正文（权威）；本设计不得与之冲突 |
| [../ui/README.md](../ui/README.md) | Web 界面参考图；原生端信息结构对齐，不要求像素一致 |

**实现代码**计划放在仓库根目录 [`clients/`](../../clients/README.md)（`clients/android/`、`clients/harmony/`）。**在本文档经确认之前，不落地业务代码。**

开源仓库禁止写入私有部署信息（真实域名、节点名、密钥、内网主机等）。示例一律使用 `https://example.com`。

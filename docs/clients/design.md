# 多邻灶原生客户端设计（Android + 鸿蒙）

状态：**待确认**（确认前不写业务代码）  
范围：与一期 Web 共用同一 Axum API；原生端分发自托管站点，不上应用商店（本期）  
权威需求：[一期需求](../requirements-phase1.md)  
UI 参考：[docs/ui](../ui/README.md)

---

## 0. 目标与非目标

### 0.1 目标

1. 提供与 Web **同一后端** 的原生客户端，覆盖员工竖屏主路径，并规划食堂 / 总管横屏工作台。
2. **通用安装包**：包内可读服务端 Base URL；自托管方改配置后重新打包/签名，用户从**自家站点**下载，连的就是自家 API。
3. Android（Kotlin）与鸿蒙（ArkTS）**产品逻辑共享**，仅平台层实现差异（见文末 §8）。
4. 一期优先落地：**密码登录 + 员工报餐最小闭环**；其余画面按阶段迭代，权限与 Web 一致。

### 0.2 非目标（本期设计不展开实现）

- 上架 Google Play / 华为应用市场等（文档可预留，本期不做）。
- 从 GitHub Release 作为主分发渠道（可选镜像，主路径是站点 `/app/`）。
- 在 Swarm / CD 流水线里强制编包（可后续接；设计先定产物形态）。
- 另起一套原生专用 API 或 GraphQL。
- 短信 OTP、支付、餐补。

---

## 1. 角色与用户逻辑

一期四个角色与 Web 相同。同一账号可持有多条角色分配；客户端按 `GET /api/me` 返回的 `roles` / `permissions` **显隐导航与拦截写操作**，不在本地发明权限。

| 角色 | 代码 | 客户端形态 | 核心用户逻辑 |
|------|------|------------|--------------|
| 员工 | `employee` | **竖屏** App 主壳 | 选班制 / 个人 DSL、本周报餐（绿报 / 橙不报）、绑定多个食堂（使用范围）、餐后评分、设备与 Passkey、站内通知 |
| 食堂 / 物业 | `canteen` | **横屏**工作台（可与竖屏同包，登录后按角色切壳） | 人数总览、菜谱、选菜优化、强提醒、评分汇总、评语；范围限 `canteen_ids` |
| 管理员 | `admin` | 同上横屏壳 | 邀请码、截止后改单、评分/评语（范围食堂）；侧栏按权限显隐 |
| 总管理 | `super_admin` | 同上横屏壳 | 全局范围：邮箱开关、强提醒阈值、账号找回、LLM 渠道、审计、导出、建食堂与角色分配等 |

### 1.1 登录后路由规则

1. 调用 `GET /api/me`。
2. 若仅有 `employee`（或同时有员工能力且用户从「员工入口」打开）：进入**员工竖屏** IA。
3. 若持有 `canteen` / `admin` / `super_admin` 任一：可进入**横屏工作台**；侧栏菜单严格按权限码过滤（与需求第 11 节一致）。
4. 多角色账号：提供「切换到员工端 / 工作台」入口（设置或顶栏），默认优先上次使用的壳；无历史时：有工作台权限则进工作台，否则进员工端。
5. 无任何有效角色或会话失效（401）：回登录页并清本地会话状态。

### 1.2 员工逻辑要点（与需求对齐）

- **新用户默认不报餐**：未选班制且未完成报餐前，不自动生成报餐。
- 班制仅 **上二休四** / **自定义**；「用描述生成模板」是模板生成器，不是第三种班制。
- 预览与本周报餐：**绿色 = 报，橙色 = 不报**；切换即写回（个人 DSL / 当日餐次），不以「底部统一保存」为权威交互。
- 绑定多食堂是**报餐使用范围**，不是管理范围。
- 截止后本人不可改；仅 `meal.admin_override`（及超管）可改。
- 员工端**不展示**菜品排行；评分 −2～+2，菜/饭/汤分开。

### 1.3 食堂 / 总管逻辑要点

- 横屏 + **左侧栏**主导航；登录页无侧栏。
- 菜、饭、汤分组，禁止混成一条列表。
- 选菜优化：周视图 → 当餐选定；勾选立即保存；色阶表示暗线偏好高低，界面不展示偏好数值 / Elo。
- 强提醒不自动下架；停用须在菜品表单确认。
- 审计保留 ≥6 个月；客户端只读展示，不提供删未满期记录。

---

## 2. 信息架构（IA）

视觉参考见 `docs/ui/`，原生不要求像素级一致，**信息结构必须对齐**。

### 2.1 员工竖屏（底部三 Tab：报餐 | 评分 | 我的）

```
登录 / 注册（邀请码）
 └─ 员工壳
      ├─ 报餐
      │    ├─ 首页（未选班制提示 / 下一餐摘要 / 入口）
      │    ├─ 选择班制（上二休四 | 自定义）
      │    ├─ 用描述生成模板 → 模板预览（绿/橙切换）
      │    └─ 本周报餐（按日早中晚；可切换已绑食堂）
      ├─ 评分
      │    └─ 可评目标列表（餐后 1 周窗口；菜/饭/汤分组）
      └─ 我的
           ├─ 绑定食堂
           ├─ 设备与 Passkey / 新设备确认
           ├─ 站内通知
           └─ 退出登录
```

**MVP（第一期编码，待设计确认后）：**

1. 登录（密码）
2. 首页 / 本周报餐列表（`/api/me/home` 与/或 `/api/me/meals`）
3. 切换某日某餐报/不报（`PUT /api/meals/{date}/{slot}`）
4. 401 → 回登录

班制、模板生成、评分、Passkey、注册等列为 MVP+。

### 2.2 横屏工作台（食堂 / 管理 / 总管）

左侧栏（按权限显隐，与需求 §11 一致）：

| 菜单 | 主要权限 | 说明 |
|------|----------|------|
| 人数总览 | `menu.read` / `optimizer.read` / `meal.admin_override` 等（见需求） | 按食堂+日期看三餐人数 |
| 菜谱 | `menu.read` / `menu.write` | 列表 + 菜品表单 |
| 选菜优化 | `optimizer.read` | 周视图 / 当餐选定 |
| 强提醒 | 食堂范围查看 | 阈值页仅超管 |
| 评分汇总 | `rating.admin_read` | |
| 评语 | `comment.admin_read` | 匿名不展示账号 |
| 截止后改单 | `meal.admin_override` | |
| 邀请码 | `user.invite` | 完整码仅展示一次 |
| 数据导出 | `export.run` | 超管 |
| 审计 | `audit.read` | 超管 |
| 邮箱设置 | `system.super` | 超管 |
| 强提醒阈值 | `rating.strong_alert_config` | 超管 |
| 账号找回 | `user.recover` | 超管 |
| LLM 渠道 | `system.super` | 超管 |
| 设备 | 本账号 | 踢设备 / Passkey / 批准新设备 |

工作台 **MVP 可后置**；编码顺序建议：员工 MVP → 人数总览 + 菜谱只读 → 选菜优化 → 其余管理页。

### 2.3 导航与断网

- 离线：只读缓存可选（MVP 可不做）；写操作失败需明确错误，禁止静默丢单。
- 假数据开关（debug / 配置项 `useMock`）：无后端时可演示 IA；默认关闭。

---

## 3. 与 Web API 对齐

客户端是 **第一方原生壳**，不是浏览器，但仍须遵守服务端对写操作的 CSRF 第二道闸与会话模型。

### 3.1 传输约定

| 项 | 约定 |
|----|------|
| Base URL | 来自包内配置（§4），无尾斜杠或统一规范化 |
| 会话 | Cookie 名 `dlz_session`；客户端持久化 CookieJar（应用私有存储） |
| CSRF | 所有 **POST/PUT/PATCH/DELETE**（含登录、注册）必须带请求头 **`X-DLZ-Requested: 1`** |
| JSON | `Content-Type: application/json`（除 multipart 上传图片） |
| 错误 | HTTP 非 2xx；body 常见 `{ "error", "message" }`；401 清会话并回登录（登录接口本身的 401 除外） |
| 时钟 | 日期、周视图按**北京时间**（与需求一致） |

对照实现：`web/src/api.ts`（`credentials: "include"` + `x-dlz-requested: 1`）、`src/routes.rs`。

### 3.2 认证与会话 API

| 能力 | 方法 | 路径 | 备注 |
|------|------|------|------|
| 可用登录方式 | GET | `/api/session/methods` | 决定是否展示 magic link 等 |
| 密码登录 | POST | `/api/session/password` | body: `{ username, password }` → Set-Cookie |
| TOTP | POST | `/api/session/totp` | MVP+ |
| 登出 | POST | `/api/session/logout` | 吊销会话 |
| 注册 | POST | `/api/register` | `{ username, invite_code, password }` |
| 当前用户 | GET | `/api/me` | roles、permissions |
| Magic / Passkey / 新设备审批 | 见 routes | `/api/session/*`、`/api/login-approvals*`、`/api/passkeys*`、`/api/devices*` | MVP+ |

### 3.3 员工报餐 API（MVP 必接）

| 能力 | 方法 | 路径 | 备注 |
|------|------|------|------|
| 首页摘要 | GET | `/api/me/home` | `next`、`days[]`（含 locked） |
| 区间报餐 | GET | `/api/me/meals?start=&end=` | 含 `locked_dates` |
| 改一餐 | PUT | `/api/meals/{date}/{slot}` | `slot`: `breakfast` \| `lunch` \| `dinner`；body: `{ ordering: bool, canteen_id: uuid \| null }` |
| 我的食堂 | GET/PUT | `/api/me/canteens` | |
| 班制 | GET/PUT | `/api/me/shift` | |
| 模板 | GET/PUT/POST | `/api/me/meal-template`、`/api/meal-templates/*` | MVP+ |

### 3.4 食堂 / 管理 API（规划）

与 Web 共用，例如：

- 人数：`GET /api/canteens/:id/headcount`
- 菜品：`/api/canteens/:id/dishes`、图片上传
- 选菜：`/api/canteens/:id/selection*`
- 强提醒：`/api/canteens/:id/alerts`
- 管理：`/api/admin/*`（邀请、改单、评论、评分汇总、设置、审计、导出、找回、LLM…）

原生客户端**不新增**后端权限码；无权限则隐藏入口，调用失败按 403 展示。

### 3.5 健康检查（分发与运维）

- `GET /healthz` 或 `GET /api/health`：站点存活；客户端可选在「关于 / 诊断」中调用。

---

## 4. 包内服务端 URL 与自托管分发

### 4.1 配置形态（跨平台）

安装包内放置可读配置文件（路径见 §8），最小 schema：

```json
{
  "baseUrl": "https://example.com",
  "useMock": false
}
```

- 默认占位 **`https://example.com`**（开源仓库不得写入真实域名）。
- 应用启动时读取；非法 URL 则拒绝联网并提示「安装包未绑定有效服务器」。
- **不**在运行时让用户随意改生产 Base URL（避免指到钓鱼站）；若需调试，仅 debug 包或显式「开发者选项」。

### 4.2 自托管「改配置再打包」

目标：**谁家站点提供的包，就连谁家后端。**

推荐流水线（部署方执行，CI 可选）：

1. 取得**通用未绑定**构建产物（或源码 release 构建）。
2. 运行注入脚本，将 `baseUrl` 写成 `https://<自有域名>`（无路径或仅到站点根；API 仍为 `{baseUrl}/api/...`）。
3. **重新签名**（Android：`apksigner` / 鸿蒙：DevEco 签名配置）。
4. 将产物放到站点静态目录，例如：
   - `GET {baseUrl}/app/duolinzao.apk`（或鸿蒙对应包名）
   - 可选 `GET {baseUrl}/app/version.json`：`{ "versionCode", "versionName", "url", "sha256" }` 供应用内更新

脚本占位（实现阶段再落地，设计约定接口）：

- `clients/android/scripts/inject-server-url.sh <baseUrl> [apk-or-project]`
- `clients/harmony/scripts/inject-server-url.sh <baseUrl> [hap-or-project]`

脚本职责：改包内 / 工程内 `server.json`（或等价），并打印「需使用部署方密钥重签」的说明；**不**把私钥写进仓库。

### 4.3 只从服务端分发

- 用户打开自家多邻灶网站 → 下载页或 `/app/` → 安装。
- 应用内检查更新：只请求 **当前 `baseUrl` 同源** 的 `version.json`，校验签名证书指纹 / 包名一致后再提示安装。
- 不上架时：依赖系统「允许未知来源」等能力；文档中说明风险与证书固定建议。

### 4.4 与 Web 同域的好处

Cookie `Secure` / 同站策略在原生 OkHttp/HTTP 客户端上按绝对 Base URL 管理；只要 `baseUrl` 指向提供 API 的 HTTPS 源即可。TLS 由站点（如反向代理）终结，与现网 Web 一致。

---

## 5. 安全与隐私（客户端）

1. 开源 **禁止** 提交：真实域名、节点名、DEPLOY 密钥、docker secret、内网 IP、私人邮箱等。
2. 本地仅存：Cookie、必要用户缓存；密码不进本地持久化（系统密码柜可选，MVP 不做）。
3. 证书钉扎：可选增强；自托管证书多样，默认关闭，文档说明如何由部署方打开。
4. 日志：正式包不打印 token / cookie / 密码。
5. 截屏：评分等敏感页可按平台能力限制（可选）。

---

## 6. 分阶段交付

| 阶段 | 内容 | 出口 |
|------|------|------|
| D0 | 本设计文档评审 | 用户确认后再编码 |
| D1 | Android + 鸿蒙工程骨架 + `server.json` + inject 脚本占位 + 假数据开关 | 能装 debug / 本地预览壳 |
| D2 | 密码登录 + 员工报餐 MVP（真 API） | 对接演示环境跑通 |
| D3 | 班制 / 绑定食堂 / 通知 / 评分 | 员工主路径对齐 Web |
| D4 | 横屏工作台（人数、菜谱、选菜…） | 按权限分批 |
| D5 | `/app/` 分发页与应用内更新 | 文档 + 可选服务端静态资源 |

---

## 7. 仓库布局（确认后）

```
clients/
  README.md                 # 已有占位
  android/                  # Kotlin + Compose（待编码）
    app/src/main/assets/server.json
    scripts/inject-server-url.sh
    README.md
  harmony/                  # ArkTS 纯鸿蒙（待编码）
    entry/src/main/resources/rawfile/server.json   # 路径以实现为准
    scripts/inject-server-url.sh
    README.md
docs/clients/
  README.md
  design.md                 # 本文
```

根 `README.md` 增加指向 `docs/clients/` 与 `clients/` 的简短链接。  
**不**把无 SDK 的强制编译加入会导致 CI 变红的 job；有镜像后再加可选 job。

---

## 8. 平台差异（仅节）

以下之外，产品逻辑、IA、API、配置 schema **必须一致**。

| 点 | Android | 鸿蒙 HarmonyOS NEXT |
|----|---------|---------------------|
| 语言 / UI | Kotlin + Jetpack Compose | ArkTS + ArkUI |
| 工程 | Gradle（`clients/android`） | DevEco / hvigor（`clients/harmony`） |
| 网络 | OkHttp + CookieJar（或 Ktor） | `@ohos.net.http` / 推荐封装；自行持久化 Cookie |
| 包内配置路径 | `assets/server.json` | `resources/rawfile/server.json`（或模块等价 rawfile） |
| 注入脚本 | 改 assets；`apksigner`/`jarsigner` 重签 APK | 改 rawfile；按鸿蒙签名工具重签 HAP |
| 分发产物 | `.apk`（或 `.aab` 非本期） | `.hap` / 应用包 |
| 站点路径建议 | `/app/duolinzao.apk` | `/app/duolinzao.hap`（名可配置） |
| Passkey / WebAuthn | 系统 Credential Manager（MVP+） | 平台能力对齐情况单独评估（MVP+）；密码路径两端都有 |
| 后台推送 | 非一期依赖；站内通知拉取即可 | 同左 |
| 商店 | 本期不上架 | 本期不上架 |

---

## 9. 待确认清单（请评审）

请确认或勾改下列项后再开工编码：

1. **MVP 范围**是否即为：密码登录 + 首页/本周报餐 + 切换报/不报（真 API + mock 开关）？
2. **单安装包双壳**（员工竖屏 + 工作台横屏按角色切换）是否接受？或员工包 / 管理包拆分？
3. **禁止用户改 Base URL**（仅注入打包）是否接受？debug 例外？
4. **`/app/version.json` 应用内更新**是否列入 D5，MVP 不做？
5. 鸿蒙是否 **纯 ArkTS / NEXT**，不兼容旧 Java 鸿蒙？
6. 文档与实现目录：`docs/clients/` + `clients/{android,harmony}/` 是否最终命名？

---

## 10. 修订记录

| 日期 | 说明 |
|------|------|
| 2026-10-13 | 初稿：暂停编码，先出共享设计；Android + 鸿蒙并列，平台差异单列 |

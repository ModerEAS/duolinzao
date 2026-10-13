# 多邻灶原生客户端设计（Android）

状态：**方向已定，细节可继续修订**（确认前仍不写业务代码）  
范围：与一期 Web 共用同一 Axum API；**本期只做 Android**；自托管站点分发，不上应用商店  
权威需求：[一期需求](../requirements-phase1.md)  
UI 参考：[docs/ui](../ui/README.md)

**已拍板**

| 项 | 决定 |
|----|------|
| 平台 | **只做 Android**（Kotlin + Jetpack Compose） |
| 鸿蒙 | **暂缓 / 本期忽略**；不建 `clients/harmony/`，不排期纯血 HarmonyOS NEXT |
| MVP | Android：登录（**密码 + Passkey**）+ 员工报餐最小闭环 |
| Passkey | 使用 **Android Credential Manager / 平台 API**，**不用 WebView 套 WebAuthn 页** |
| 分发 | 服务端（部署方）改包内 Base URL 后重签，经站点 `/app/` 下发 |
| 卓易通 | 仅作可选兼容备注（§8.2），**不作为交付目标** |

---

## 0. 目标与非目标

### 0.1 目标

1. 提供与 Web **同一后端** 的 Android 客户端，覆盖员工竖屏主路径，并规划食堂 / 总管横屏工作台。
2. **通用 APK**：包内可读服务端 Base URL；自托管方在发版流程里改配置并重新签名，用户从**自家站点**下载，连的就是自家 API。
3. MVP：**密码登录 + Passkey 登录/注册（平台 API）+ 员工报餐最小闭环**；其余画面按阶段迭代，权限与 Web 一致。
4. 开源仓库示例域名仅用 `https://example.com`，禁止私有部署信息入仓。

### 0.2 非目标（本期）

- **纯血鸿蒙 / HarmonyOS NEXT / ArkTS 独立工程**（暂缓，见 §8.1）。
- 上架 Google Play / 华为空洞等应用商店（可预留，本期不做）。
- 以 GitHub Release 作为主分发渠道（可选镜像；主路径是站点 `/app/`）。
- 在 Swarm / CD 里强制编 APK（可后续接；设计先定产物与改包流程）。
- 另起原生专用 API；短信 OTP、支付、餐补。
- 用 WebView 加载站点来「假装」原生 Passkey。

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
2. 若仅有 `employee`（或用户选择员工入口）：进入**员工竖屏** IA。
3. 若持有 `canteen` / `admin` / `super_admin` 任一：可进入**横屏工作台**；侧栏按权限码过滤（需求第 11 节）。
4. 多角色：提供「员工端 / 工作台」切换；默认上次使用的壳，无历史则有工作台权限进工作台，否则进员工端。
5. 会话失效（401）：回登录并清本地 Cookie / 会话状态。

### 1.2 员工逻辑要点（与需求对齐）

- **新用户默认不报餐**：未选班制且未完成报餐前，不自动生成报餐。
- 班制仅 **上二休四** / **自定义**；「用描述生成模板」是模板生成器，不是第三种班制。
- 预览与本周报餐：**绿色 = 报，橙色 = 不报**；切换即写回。
- 绑定多食堂是**报餐使用范围**，不是管理范围。
- 截止后本人不可改；仅 `meal.admin_override`（及超管）可改。
- 员工端**不展示**菜品排行；评分 −2～+2，菜/饭/汤分开。

### 1.3 食堂 / 总管逻辑要点

- 横屏 + **左侧栏**；登录页无侧栏。
- 菜、饭、汤分组，禁止混成一条列表。
- 选菜优化：周视图 → 当餐选定；勾选立即保存；色阶表示暗线偏好，界面不展示偏好数值 / Elo。
- 强提醒不自动下架；停用须在菜品表单确认。

---

## 2. 信息架构（IA）

视觉参考见 `docs/ui/`，原生不要求像素级一致，**信息结构对齐**。

### 2.1 员工竖屏（底部三 Tab：报餐 | 评分 | 我的）

```
登录（Passkey / 密码等，顺序见 §3.2）/ 注册（邀请码）
 └─ 员工壳
      ├─ 报餐
      │    ├─ 首页
      │    ├─ 选择班制 / 描述生成模板 / 模板预览
      │    └─ 本周报餐（按日早中晚；可切换已绑食堂）
      ├─ 评分
      └─ 我的
           ├─ 绑定食堂
           ├─ 设备与 Passkey / 新设备确认
           ├─ 站内通知
           └─ 退出登录
```

### 2.2 MVP（Android 第一期编码范围）

待本设计不再阻塞后编码，**仅 Android**：

1. **登录**：密码 + **Passkey**（平台 Credential Manager；见 §3.2）
2. 首页 / 本周报餐（`/api/me/home` 与/或 `/api/me/meals`）
3. 切换某日某餐报/不报（`PUT /api/meals/{date}/{slot}`）
4. 401 → 回登录；写请求带 `X-DLZ-Requested: 1`；Cookie 持久化
5. 包内 `server.json` + inject 脚本占位 + 可选 `useMock`

班制、模板生成、评分、邀请注册、横屏工作台等列为 **MVP+**。

### 2.3 横屏工作台（规划，非 MVP）

左侧栏按权限显隐（人数总览、菜谱、选菜优化、强提醒、评分、评语、改单、邀请码、导出、审计、邮箱、阈值、找回、LLM、设备等），与需求 §11 一致。编码顺序建议：员工 MVP → 人数总览 + 菜谱 → 选菜优化 → 其余。

### 2.4 单包双壳

**接受**：一个 APK 内员工竖屏 + 工作台横屏，按角色切换。不拆「员工包 / 管理包」（除非日后商店策略要求再议）。

---

## 3. 与 Web API 对齐

### 3.1 传输约定

| 项 | 约定 |
|----|------|
| Base URL | 包内配置（§4），启动时规范化 |
| 会话 | Cookie `dlz_session`；OkHttp CookieJar（或等价）持久化到应用私有存储 |
| CSRF | 所有 POST/PUT/PATCH/DELETE（含登录）带 **`X-DLZ-Requested: 1`** |
| JSON | `Content-Type: application/json`（除图片 multipart） |
| 错误 | 非 2xx + `{ error, message }`；业务 401 清会话回登录 |
| 时钟 | 日期 / 周视图按**北京时间** |

对照：`web/src/api.ts`、`src/routes.rs`。

### 3.2 登录与 Passkey（Android 平台 API）

登录方式优先级与 Web 一致：**Passkey → TOTP → 邮箱 magic link（可关）→ 密码**。MVP 必须落地 **Passkey + 密码**；TOTP / magic link 为 MVP+。

#### 3.2.1 密码

- `POST /api/session/password`，body `{ "username", "password" }` → Set-Cookie。

#### 3.2.2 Passkey（非 WebView）

后端已有 WebAuthn 风格接口（与 Web 共用）：

| 步骤 | 方法 | 路径 |
|------|------|------|
| 登录开始 | POST | `/api/session/passkey/start` |
| 登录完成 | POST | `/api/session/passkey/finish` |
| 绑定开始 | POST | `/api/passkeys/register/start`（需已登录） |
| 绑定完成 | POST | `/api/passkeys/register/finish` |
| 解绑 | DELETE | `/api/passkeys/:id` |

**Android 实现要求：**

1. 使用 **Credential Manager**（`androidx.credentials`）及系统支持的 **publickey / passkey** 流程，把服务端下发的 challenge / `publicKey` 选项交给平台 API，再把断言结果 POST 回 finish 接口。
2. **禁止**用 WebView 打开站点登录页来完成 Passkey，避免双栈会话、Cookie 不一致和「假原生」。
3. RP ID / origin 必须与部署域名（即包内 `baseUrl` 的 host）及服务端 WebAuthn 配置一致；自托管改包时，服务端配置与 APK 内 Base URL **一起**指向同一域名。
4. 设备无 Passkey 能力或用户取消时，降级到密码（及后续 TOTP 等），不阻塞登录。
5. 新设备登录确认（`/api/login-approvals*`）与 Passkey 绑定流程按需求 §3.1；MVP 可先做「已有 Passkey 直接登录」，页内批准新设备列为 MVP+。

### 3.3 员工报餐 API（MVP 必接）

| 能力 | 方法 | 路径 | 备注 |
|------|------|------|------|
| 当前用户 | GET | `/api/me` | |
| 首页 | GET | `/api/me/home` | |
| 区间报餐 | GET | `/api/me/meals?start=&end=` | |
| 改一餐 | PUT | `/api/meals/{date}/{slot}` | body: `{ ordering, canteen_id }`；`slot`: breakfast/lunch/dinner |
| 我的食堂 | GET/PUT | `/api/me/canteens` | MVP 可先只读 |
| 班制 | GET/PUT | `/api/me/shift` | MVP+ |

### 3.4 食堂 / 管理 API（规划）

与 Web 共用（`/api/canteens/...`、`/api/admin/...` 等）。无权限则隐藏入口；不新增权限码。

---

## 4. 包内服务端 URL 与「服务端改包」分发

### 4.1 配置形态

APK 内例如 `assets/server.json`：

```json
{
  "baseUrl": "https://example.com",
  "useMock": false
}
```

- 开源默认 **`https://example.com`**。
- 启动读取；非法 URL 则提示「安装包未绑定有效服务器」，不发起业务请求。
- **正式包不提供**让终端用户随意改 Base URL 的设置项（防钓鱼）；仅 debug / 开发者选项可例外。

### 4.2 服务端（部署方）改包再分发

目标：**谁家站点提供的 APK，就连谁家 API。**

推荐流程（由部署流水线或管理员在构建机执行，**不必**在业务容器里现场编 Android）：

1. 取得通用构建产物（CI 打出的 unsigned/debug 或已用「占位域名」签过的中间包；或从源码 `./gradlew :app:assembleRelease`）。
2. 运行 `clients/android/scripts/inject-server-url.sh https://<自有域名> <apk>`：写入/替换包内 `server.json` 的 `baseUrl`。
3. 使用**部署方自己的**上传密钥 **重新签名**（`apksigner` / Play App Signing 不在本期；自托管用本地 keystore，密钥不进 git）。
4. 将最终 APK 放到站点静态目录，例如：
   - `GET {baseUrl}/app/duolinzao.apk`
   - 可选 `GET {baseUrl}/app/version.json`：`{ "versionCode", "versionName", "url", "sha256" }` 供应用内更新（校验包名与签名证书指纹）

说明：

- 「服务端改包」指**部署方掌控的发版步骤**（脚本 + 对象存储 / 站点 `/app/`），不是让 Axum 进程在请求时动态改二进制。
- 业务镜像可只负责**托管** `/app/*.apk`；改包可在 CI（传入 `DLZ_DOMAIN` 一类**部署私有**变量，勿写入开源仓库）完成后再 `scp`/上传。
- 用户只从自己打开的站点下载，从而自然绑定该站点的 API。

### 4.3 只从服务端分发

- 不上架时依赖「未知来源安装」；文档说明风险。
- 应用内更新只请求**当前 baseUrl 同源**的 `version.json`。
- GitHub Release 若有，仅作源码/通用未绑定包镜像，**不是**各租户的绑定包来源。

---

## 5. 安全与隐私

1. 开源禁止：真实域名、节点名、密钥、内网 IP、私人邮箱等。
2. 本地：Cookie 与必要缓存；密码不落盘（系统密码柜可选，非 MVP）。
3. 证书钉扎默认关（自托管证书多样）；部署方可文档化开启方式。
4. 正式包日志不打印 token / cookie / 密码。
5. Passkey 私钥不出系统实现边界；应用只持有断言结果。

---

## 6. 分阶段交付（仅 Android）

| 阶段 | 内容 | 出口 |
|------|------|------|
| D0 | 本设计评审 | 确认后开工 |
| D1 | `clients/android` 骨架 + `server.json` + inject 脚本 + mock 开关 | assembleDebug |
| D2 | 密码 + Passkey 登录 + 员工报餐 MVP（真 API） | 演示环境跑通 |
| D3 | 班制 / 绑定食堂 / 通知 / 评分 / 新设备确认 | 员工主路径对齐 Web |
| D4 | 横屏工作台分批 | 按权限 |
| D5 | `/app/` 托管与应用内更新 | 文档 + 静态资源约定 |

---

## 7. 仓库布局（确认后编码）

```
clients/
  README.md
  android/                    # 唯一本期客户端工程
    app/src/main/assets/server.json
    scripts/inject-server-url.sh
    README.md
docs/clients/
  README.md
  design.md                   # 本文
```

- **不**创建 `clients/harmony/`（鸿蒙暂缓）。
- 根 README 指向本文；不把无 SDK 的强制 Android 编译加入会红的主 CI job。

---

## 8. 平台说明

### 8.1 鸿蒙：暂缓

本期**不做**纯血鸿蒙（HarmonyOS NEXT / ArkTS）客户端，不维护双端产品逻辑对照表，不占用排期。若未来恢复，另开设计修订，再议是否复用本 Android 的 IA / API 映射。

### 8.2 卓易通（可选兼容备注，非交付目标）

部分运行环境可通过**卓易通**等方案在鸿蒙设备上运行 Android 应用。这属于**兼容层 / 旁路运行**，不是原生鸿蒙工程：

- 不保证 Passkey、后台限制、通知、安装来源等与原生 Android 一致。
- **不**纳入验收标准；文档可不提供专项适配。
- 若用户自行用卓易通安装本 APK，视为「Android 包在兼容层上的尽力而为」，问题优先在标准 Android 设备上复现。

### 8.3 Android 实现选型（摘要）

| 点 | 选择 |
|----|------|
| 语言 / UI | Kotlin + Jetpack Compose |
| 网络 | OkHttp（CookieJar）+ Retrofit 或等价 |
| Passkey | Credential Manager 等平台 API |
| 配置 | `assets/server.json` |
| 产物 | `.apk`；站点 `/app/duolinzao.apk` |

---

## 9. 已确认 / 仍可微调

**已确认**

1. MVP = **Android**；鸿蒙暂缓。
2. MVP 功能 = 密码 + **Passkey（平台 API）** + 员工报餐最小闭环。
3. 单 APK 双壳（员工 + 工作台）可接受；工作台非 MVP。
4. 正式包禁止用户改 Base URL；由部署方 inject + 重签。
5. 主分发 = 站点 `/app/`；应用内更新可放 D5。

**仍可微调（不阻塞 D0→D1）**

- Passkey 与密码在登录页的默认展示顺序（需求为 Passkey 优先）。
- MVP 是否包含「绑定 Passkey」页，或仅「用已有 Passkey 登录」。
- inject 脚本是改未打包的 `assets` 再 assemble，还是改已构建 APK 内文件后重签（两种都可，README 写清一种主路径）。

---

## 10. 修订记录

| 日期 | 说明 |
|------|------|
| 2026-10-13 | 初稿：Android + 鸿蒙并列 |
| 2026-10-13 | 修订：只做 Android；鸿蒙暂缓；MVP 含 Passkey（Credential Manager，非 WebView）；强化服务端改包分发；卓易通仅可选备注 |

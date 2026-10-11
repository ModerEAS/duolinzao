# 规模化全流程测试（@scale）设计草案

> 状态：smoke（3 食堂 × 30 人）已实现并跑通 T0–T5；`full` 生成器就绪（10 × 300），长耗时未进 CI。
> 切片用例（employee/canteen/admin/super_admin）继续管细权限；本文件管「多食堂 × 多人、全 Playwright UI」的旅程。

## 目标

- 模拟真实运营：约 **10 个食堂**、约 **300 名员工**，每人绑定食堂不同，报餐与评价不同。
- **业务操作全程 Playwright 点 UI**（建食堂、建菜、排菜、发码、注册、绑定、报餐、评分、看汇总），不用业务 API 批量灌用户/报餐。
- 用 **测试时钟时间切片** 覆盖提前很久报餐、锁前可改、锁后员工超时、餐后可评、超窗不可评等，而不干等真实日历。

## 规模档位

| 环境变量 | 食堂 | 员工 | 用途 |
|---|---|---|---|
| `DLZ_SCALE=smoke`（默认 CI） | 3 | 30 | PR / 日常 |
| `DLZ_SCALE=full` | 10 | 300 | 夜间 / 手工 |

花名册由固定种子生成，同档位可重复跑。

## 花名册字段

开跑时生成 JSON（建议路径 `e2e/scale/roster.{smoke|full}.json`，可 gitignore 生成物或提交 smoke 样例）。

### `meta`

| 字段 | 说明 |
|---|---|
| `scale` | `smoke` / `full` |
| `seed` | 随机种子 |
| `target_date` | 业务目标日（报餐/排菜/锁定所围绕的日期，`YYYY-MM-DD`，上海） |
| `lock_rule` | 文案备注：北京时间提前一天固定时点锁定次日三餐 |

### `canteens[]`

| 字段 | 说明 |
|---|---|
| `key` | 稳定键，如 `c01` |
| `name` | 展示名，如 `规模一食堂`（须唯一） |
| `canteen_user` | 食堂账号用户名 |
| `canteen_pass` | 密码 |
| `dishes` | `{菜:[…], 饭:[…], 汤:[…]}` 本食堂要建的菜名 |

### `admins[]`

| 字段 | 说明 |
|---|---|
| `user` / `pass` | 管理员账号 |
| `canteen_keys` | 管辖食堂 key 列表（有重叠有隔离） |

### `employees[]`

| 字段 | 说明 |
|---|---|
| `user` / `pass` | 员工账号 |
| `bind_canteen_keys` | 绑定的食堂（1～N） |
| `shift` | `上二休四`（一期旅程主路径；自定义可后补） |
| `meals[]` | 见下 |
| `ratings[]` | 见下 |

### `employees[].meals[]`

| 字段 | 说明 |
|---|---|
| `date` | 通常等于 `target_date`，也可含对照日 |
| `slot` | `早` / `中` / `晚` |
| `intent` | `报` / `不报` |
| `canteen_key` | 报餐食堂；`不报` 时可空或仍写绑定内食堂 |
| `at_slice` | 计划在哪个切片执行该操作：`T1` / `T2` / `T3`（T3 用于断言超时失败） |

### `employees[].ratings[]`

| 字段 | 说明 |
|---|---|
| `date` / `slot` / `canteen_key` | 对哪一餐 |
| `scores` | `{菜: -2..2, 饭:…, 汤:…}`（按该餐供应品类） |
| `comment` | 可选匿名评语 |
| `at_slice` | `T4` 成功；另可安排 `T5` 再评期望失败 |

### `expect`（对账摘要，生成时算好）

| 字段 | 说明 |
|---|---|
| `headcount[canteen_key][date][slot]` | 有效报餐人数 |
| `rating_visible[canteen_key]` | 该食堂汇总应能看到的员工 user 列表 |

## 时间切片

测试环境提供**可拨时钟**（总管测试入口或启动参数）。切片之间直接跳，不连续等。

| 切片 | 时钟含义 | UI 动作要点 | 断言要点 |
|---|---|---|---|
| **T0 筹备** | 开跑日白天（目标日前若干天） | 总管建食堂、挂食堂/管理员角色；各食堂建菜饭汤并为 `target_date` 排菜；发足够邀请码；300（或 30）人注册、绑定、选班制 | 侧栏/列表可见；邀请码一次一码 |
| **T1 窗口内早报** | 锁定点前很久 | 按花名册在未锁餐次报绿/不报橙；跨食堂员工切换食堂 | 提前很久可报；后报覆盖未锁先报 |
| **T2 锁前夕** | 锁定点前数分钟 | 抽改若干单；食堂看人数总览 | 仍可改；人数与 `expect.headcount` 一致（按范围） |
| **T3 已锁** | 刚过提前一天锁定点 | 员工再改 → 应拒；管理员在范围内改；范围外拒；食堂账号改他人 → 拒 | 锁后权限矩阵 |
| **T4 餐后可评** | 该餐已发生且仍在餐后 1 周窗 | 已报员工评分+可选评语；食堂/管理员看汇总与评语 | 范围隔离；未报不可评 |
| **T5 窗外人** | 餐后超过 1 周 | 再评 | 应拒 |

## Playwright 约束

- 标签：`@scale`；实现未完成前保留 `@wip`，默认 CI 排除。
- `smoke` 可进 CI 夜间或单独 job；`full` 默认不进 PR。
- `full` 允许多 worker 按员工区间并行；`slowMo=0`；失败才留 video。
- 复用现有 step 文案；规模循环用「按花名册…」类步骤，避免手写 300 个场景。
- 演示视频：从花名册抽少量角色录，不必全员进镜。

## 实现清单（给 Pi）

1. 花名册生成器（smoke/full）+ 字段校验。
2. 测试时钟：可从 UI 拨到切片（及后端认钟）；文档写清用法。
3. `features/scale_journey.feature` 步骤可执行；`e2e/steps/scale*.ts`。
4. 配置：`DLZ_SCALE`、加长 timeout；CI 排除或单独 workflow。
5. 本机 `smoke` 跑通后 commit、push fork、开到 ModerRAS/duolinzao 的 PR。

## 测试时钟用法（已实现）

- 后端所有锁单 / 评分窗判断都走 `state.clock.now()`；生产 `clock` 即 `OverrideClock`，未拨时行为与系统钟完全一致。
- 拨钟入口只有**测试环境**：`GET/POST /api/test/clock`，需同时满足 `DLZ_ALLOW_TEST_CLOCK=1`（`scripts/e2e.sh` 只给它起的 app 设，生产配置里永不设置）与 `super_admin`（`system.super`）。`POST {"now": "<RFC3339>"}` 拨钟，`POST {"clear": true}` 恢复系统钟。这是时间缝，不建用户、不报餐、不评分。
- 切片时刻由花名册 `slices` 给出（按 `target_date` 与 `lock_time=09:00` 算好）：T0 目标日前 5 天 10:00、T1 前 2 天 10:00、T2 前 1 天 08:55、T3 前 1 天 09:05、T4 当日 20:00（三餐窗全开）、T5 当日 +8 天 20:00（窗全关）。步骤 `测试时钟拨到切片 T1…T5` 直接 POST 对应时刻，切片之间直接跳。
- 手工拨钟：`curl -b <总管会话> -X POST localhost:8080/api/test/clock -d '{"now":"2026-10-18T08:55:00+08:00"}'`，或 `GET /api/test/clock` 查看当前钟与是否被拨过。

## 如何跑

```bash
# 切片用例（默认，不含 @scale）：bash scripts/e2e.sh
# 规模化 smoke（3 食堂 × 30 人，~25 分钟）：DLZ_SCALE=smoke bash scripts/e2e.sh
# 规模化 full（10 × 300，仅生成验证 + 手工）：DLZ_SCALE=full bash scripts/e2e.sh
# 只看生成：DLZ_SCALE=smoke node e2e/scale/roster.mjs  # 落到 e2e/scale/roster.smoke.json（git 忽略）
```

- `DLZ_SCALE` 未设时：不生成花名册、不拨钟、不碰默认库之外的任何库、`bddgen` 默认排除 `@wip` 与 `@scale`，CI 的 `e2e` job 行为不变。
- `DLZ_SCALE=smoke|full` 时：`e2e.sh` 先把同服务器上的 `duolinzao_scale` 库删库重建（拨钟会写未来时间戳，审计按 `created_at` 倒序只取 200，独立库即隔离），再生成对应花名册（固定种子：smoke 42 / full 7，可用 `DLZ_SEED` 覆盖），`bddgen` 只生成 `@scale`，单测 timeout 放宽到 30 分钟。
- CI：`e2e-scale` 单独 job 跑 smoke；`full` 太重（300 人全 UI），只保证生成器可跑，默认不进 CI，见 PR 说明。

## 与切片用例的关系

- `features/employee|canteen|admin|super_admin.feature`：细权限与单页行为（锁单矩阵、范围隔离、评分窗、强提醒等），体量小、跑得快，是 PR 门禁。
- `features/scale_journey.feature`（`@scale`）：同一套规则在多食堂 × 多人下的全链路回归（建食堂→建菜→排菜→发码→注册→绑定→报餐→评分→看汇总），体量大，走单独 job / 手工。
- 两者 overlapping 的是规则口径（锁单点、评分窗、范围），不是步骤：scale 步骤按花名册循环，断言对账用 `expect.headcount` / `rating_visible`。

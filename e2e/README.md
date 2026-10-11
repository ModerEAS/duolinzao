# e2e（playwright-bdd，可执行的 Gherkin）

仓库根 `features/*.feature` 是中文 Gherkin（`# language: zh-CN`），由
[playwright-bdd](https://github.com/vitalets/playwright-bdd) 生成为 Playwright
用例再执行。features 留在仓库根（产品文档即测试入口）；e2e 只放步骤定义与配置。

## 本地跑

```bash
cd e2e && npm install                    # 安装 playwright-bdd 等依赖
npx bddgen                               # 由 features 生成 .features-gen/（git 忽略）
npx playwright test --list               # 列出生成的用例（冒烟用）
bash scripts/e2e.sh                      # 全栈：起 app + 预置数据 + bddgen + 跑测试
```

全栈前置（见 `scripts/e2e.sh`）：Postgres 可用（`DATABASE_URL`）、
`target/debug/duolinzao` 与 `web/dist` 已构建、`npx playwright install chromium`。
`npm test` 等价于 `bddgen && playwright test`，但不负责起栈。

环境变量（`scripts/e2e.sh` 会设默认值）：

- `DLZ_E2E_INVITE_CODE`（默认 `DLZ-E2E-INVITE`）：seed 预置的邀请码
- `DLZ_E2E_CANTEEN`（默认 `一食堂`）：seed 预置的食堂

## `@wip` 约定

- 默认只生成、只跑**非 `@wip`** 场景（`playwright.config.ts` 里 `tags: 'not @wip'`）。
- `@wip` = 还没步骤定义或依赖未 seeded 能力的场景：LLM 模板生成、评分、
  设备/Passkey、新设备确认、截止后改单，以及食堂 / 管理 / 总管三个横屏文件
  （整文件 `@wip`，runner 能发现文件但默认不生成用例）。
- 想本地跑 wip：`npx bddgen --tags '@wip'`（或去掉过滤）后补步骤定义；
  缺步骤时 bddgen 会直接报错（`missingSteps: fail-on-gen`），不会静默跳过。
- 去掉某场景的 `@wip` 时，必须同时给出步骤定义并本地跑通。

## 食堂 / 管理 / 总管横屏怎么补

1. 扩展 `e2e/seed.sql`：给 canteen / admin 账号（目前只有超管 `root` + 邀请码）。
2. 在 `e2e/steps/` 加对应步骤定义（登录为 seeded 角色 + 断言侧栏显隐）。
3. 去掉对应文件/场景的 `@wip`，`npx bddgen && npx playwright test --list` 确认用例出现。

## P0-1 时间锁定说明

报餐用例不死点「下一周」第一天的「早」：`lock_time=09:00`，每周日 09:00
（北京时间）后下周一三餐已锁，按钮变「早 已锁定」且 disabled。
步骤实现（`e2e/steps/employee.steps.ts` 的 `setMeal`）是点「下一周」后，
在名字为 `^早$|^中$|^晚$`（已锁定按钮叫「早 已锁定」，正则直接排除）
的按钮里找第一个 enabled 的点。最小改动，没动业务时钟。

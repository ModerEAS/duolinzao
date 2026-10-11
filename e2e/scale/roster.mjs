// 花名册生成器：DLZ_SCALE=smoke（3 食堂 × 30 员工，默认）/ full（10 × 300），固定种子。
//   DLZ_SCALE=smoke node e2e/scale/roster.mjs
// 输出 e2e/scale/roster.<scale>.json（生成物，git 忽略），字段见 docs/testing-scale-journey.md。
// target_date 取第二周的周一（上海）：足够靠后保证该日无历史数据，又不至于太远。
// 同档位同种子结构可重复；名称带 run_tag（生成时刻）避免与旧轮次撞名。
import { mkdirSync, writeFileSync } from "node:fs";

const SCALE = process.env.DLZ_SCALE === "full" ? "full" : "smoke";
const SEED = Number(process.env.DLZ_SEED || (SCALE === "full" ? 7 : 42));
const N_CANTEEN = SCALE === "full" ? 10 : 3;
const N_EMP = SCALE === "full" ? 300 : 30;
const PASS = "Scale-pass1!";
const SLOTS = ["早", "中", "晚"];
const KINDS = ["菜", "饭", "汤"];

// mulberry32：固定种子可重复。
let st = SEED >>> 0;
function rnd() {
  st |= 0;
  st = (st + 0x6d2b79f5) | 0;
  let t = Math.imul(st ^ (st >>> 15), 1 | st);
  t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
  return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
}
const pick = (arr) => arr[Math.floor(rnd() * arr.length)];
// 食堂展示名用字母编号：绝不能含「一食堂」「二食堂」子串，否则旧切片用例的非精确 option 匹配会被污染。
const CN = ["A", "B", "C", "D", "E", "F", "G", "H", "I", "J"];

function shanghaiMondayPlusWeeks(weeks) {
  // 上海今天 → 下下周一（YYYY-MM-DD）。
  const now = new Date(new Date().toLocaleString("en-US", { timeZone: "Asia/Shanghai" }));
  const monday = new Date(now);
  monday.setDate(now.getDate() - ((now.getDay() + 6) % 7) + 7 * weeks);
  const p = (n) => String(n).padStart(2, "0");
  return `${monday.getFullYear()}-${p(monday.getMonth() + 1)}-${p(monday.getDate())}`;
}
function addDays(iso, n) {
  const [y, m, d] = iso.split("-").map(Number);
  const dt = new Date(Date.UTC(y, m - 1, d + n));
  return dt.toISOString().slice(0, 10);
}

const target = shanghaiMondayPlusWeeks(2);
const now = new Date();
const runTag = `${String(now.getMonth() + 1).padStart(2, "0")}${String(now.getDate()).padStart(2, "0")}${String(now.getHours()).padStart(2, "0")}${String(now.getMinutes()).padStart(2, "0")}`;
const at = (date, hm) => `${date}T${hm}:00+08:00`;

const canteens = [];
for (let i = 0; i < N_CANTEEN; i += 1) {
  const key = `c${String(i + 1).padStart(2, "0")}`;
  canteens.push({
    key,
    name: `规模${CN[i]}食堂·${runTag}`,
    canteen_user: `scale_${key}_${runTag}`,
    canteen_pass: PASS,
    dishes: {
      菜: [`规模${CN[i]}菜A·${runTag}`, `规模${CN[i]}菜B·${runTag}`],
      饭: [`规模${CN[i]}饭·${runTag}`],
      汤: [`规模${CN[i]}汤·${runTag}`],
    },
  });
}

// 管理员：有重叠有隔离（smoke：a1 管 c01+c02，c03 隔离；full：3 人轮叠）。
const admins = [];
if (SCALE === "smoke") {
  admins.push({ user: `scale_admin_${runTag}`, pass: PASS, canteen_keys: ["c01", "c02"] });
} else {
  const keys = canteens.map((c) => c.key);
  for (let i = 0; i < 3; i += 1) {
    admins.push({
      user: `scale_admin${i}_${runTag}`,
      pass: PASS,
      canteen_keys: keys.filter((_, j) => (j + i) % 3 !== 2),
    });
  }
}

const employees = [];
const headcount = {}; // headcount[ck][date][slot] = 报数
const ratedBy = {}; // ratedBy[ck] = Set(user)
function bumpHeadcount(ck, date, slot) {
  headcount[ck] ??= {};
  headcount[ck][date] ??= {};
  headcount[ck][date][slot] ??= 0;
  headcount[ck][date][slot] += 1;
}
const COMMENTS = ["分量足", "味道不错", "有点咸", "希望多点辣", "汤很好喝", ""];

for (let i = 0; i < N_EMP; i += 1) {
  const user = `scale_emp_${runTag}_${String(i).padStart(3, "0")}`;
  // 绑定：70% 单食堂轮排，30% 双食堂（跨食堂切换覆盖）。
  const primary = canteens[i % N_CANTEEN].key;
  const bind = [primary];
  if (rnd() < 0.3) {
    const second = canteens[(i + 1) % N_CANTEEN].key;
    if (second !== primary) bind.push(second);
  }
  // 每人 2 餐：轮排两个不同餐次，75% 报。
  const slots = [SLOTS[i % 3], SLOTS[(i + 1) % 3]];
  const meals = slots.map((slot, k) => {
    const intent = rnd() < 0.75 ? "报" : "不报";
    const ck = intent === "报" && bind.length > 1 && rnd() < 0.4 ? bind[1] : primary;
    const row = {
      date: target,
      slot,
      intent,
      canteen_key: intent === "报" ? ck : primary,
      at_slice: rnd() < 0.8 ? "T1" : "T2",
    };
    if (k === 1 && i % 10 === 9) row.at_slice = "T2"; // 保证 T2 有量
    if (intent === "报") bumpHeadcount(ck, target, slot);
    return row;
  });
  // 评分：已报餐 40% 在 T4 评（逐菜 UI 提交很贵，量够覆盖多食堂多餐次即可；留足未评供 T5 超窗断言）。
  const ratings = [];
  for (const m of meals) {
    if (m.intent !== "报" || rnd() >= 0.4) continue;
    const scores = {};
    for (const kind of KINDS) scores[kind] = Math.floor(rnd() * 5) - 2;
    const comment = pick(COMMENTS);
    ratings.push({
      date: m.date,
      slot: m.slot,
      canteen_key: m.canteen_key,
      scores,
      ...(comment ? { comment } : {}),
      anonymous: rnd() < 0.8,
      at_slice: "T4",
    });
    ratedBy[m.canteen_key] ??= new Set();
    ratedBy[m.canteen_key].add(user);
  }
  employees.push({ user, pass: PASS, bind_canteen_keys: bind, shift: "上二休四", meals, ratings });
}

const rating_visible = {};
for (const c of canteens) rating_visible[c.key] = [...(ratedBy[c.key] ?? [])].sort();

const roster = {
  meta: {
    scale: SCALE,
    seed: SEED,
    run_tag: runTag,
    target_date: target,
    lock_rule: "北京时间提前一天 09:00 锁定次日三餐（随 e2e.sh schedule.lock_time，步骤以 roster.slices 为准）",
  },
  slices: {
    T0: { now: at(addDays(target, -5), "10:00"), desc: "筹备：建食堂/菜/排菜/发码/注册/绑定/班制" },
    T1: { now: at(addDays(target, -2), "10:00"), desc: "锁定点前很久：早报" },
    T2: { now: at(addDays(target, -1), "08:55"), desc: "锁定点前数分钟：抽改+人数对账" },
    T3: { now: at(addDays(target, -1), "09:05"), desc: "刚过锁定点：员工拒/管理员范围内改" },
    T4: { now: at(target, "20:00"), desc: "餐后窗内：评分+汇总" },
    T5: { now: at(addDays(target, 8), "20:00"), desc: "餐后超 1 周：再评应拒" },
  },
  canteens,
  admins,
  employees,
  expect: { headcount, rating_visible },
};

// 字段校验（与文档口径一致才落盘）。
if (!/^\d{4}-\d{2}-\d{2}$/.test(target)) throw new Error("target_date 非法");
for (const e of employees) {
  if (!e.user || !e.pass || !e.bind_canteen_keys.length) throw new Error("employee 缺字段");
  for (const m of e.meals) {
    if (!["早", "中", "晚"].includes(m.slot) || !["报", "不报"].includes(m.intent)) throw new Error("meal 非法");
    if (!["T1", "T2", "T3"].includes(m.at_slice)) throw new Error("meal at_slice 非法");
  }
  for (const r of e.ratings) {
    for (const k of KINDS) {
      if (!Number.isInteger(r.scores[k]) || r.scores[k] < -2 || r.scores[k] > 2) throw new Error("score 非法");
    }
    if (r.at_slice !== "T4") throw new Error("rating at_slice 非法");
  }
}

mkdirSync(new URL(".", import.meta.url), { recursive: true });
const out = new URL(`./roster.${SCALE}.json`, import.meta.url);
writeFileSync(out, JSON.stringify(roster, null, 2));
console.log(`roster: ${SCALE} 食堂${N_CANTEEN} 员工${N_EMP} target=${target} seed=${SEED} -> ${out.pathname}`);

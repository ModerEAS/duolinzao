<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import {
  NavBar as VanNavBar,
  Button as VanButton,
  Cell as VanCell,
  CellGroup as VanCellGroup,
  Checkbox as VanCheckbox,
  CheckboxGroup as VanCheckboxGroup,
  Empty as VanEmpty,
  Field as VanField,
  NoticeBar as VanNoticeBar,
  Tab as VanTab,
  Tabbar as VanTabbar,
  TabbarItem as VanTabbarItem,
  Tabs as VanTabs,
  Tag as VanTag,
  showConfirmDialog,
  showFailToast,
  showSuccessToast,
} from "vant";
import {
  addDays,
  api,
  explain,
  KINDS,
  shanghaiToday,
  SLOT_NAME,
  SLOT_SHORT,
  SLOTS,
  weekdayLabel,
  weekStart,
  type Canteen,
  type Groups,
  type Kind,
  type Slot,
} from "../api";
import Thumb from "../components/Thumb.vue";
import { auth } from "../session";

type Tab = "meal" | "score" | "mine";
type Screen = "home" | "shift" | "week" | "generate" | "preview" | "mine" | "bind" | "devices" | "notices";

interface Shift {
  kind: string | null;
  anchor_date: string | null;
}
interface MealItem {
  date: string;
  slot: Slot;
  canteen_id: string | null;
  ordering: boolean;
  locked: boolean;
  color: string;
}
interface PreviewSlot {
  ordering: boolean;
  color: string;
  canteen_id: string | null;
}
interface PreviewDay {
  date: string;
  slots: Record<Slot, PreviewSlot>;
}
interface Target {
  dish_id: string;
  name: string;
  kind: Kind;
  taste_tags: string[];
  image_url: string | null;
  scored: boolean;
}
interface Device {
  id: string;
  friendly_name: string;
  user_agent: string;
  ip: string;
  session_active: boolean;
}
interface Passkey {
  id: string;
  friendly_name: string;
}
interface Approval {
  id: string;
  friendly_name: string;
  user_agent: string;
  ip: string;
}
interface Note {
  id: string;
  title: string;
  body: string;
  read_at: string | null;
  created_at: string;
}

const tab = ref<Tab>("meal");
const screen = ref<Screen>("home");
const shift = ref<Shift | null>(null);
const bound = ref<Canteen[]>([]);
const allCanteens = ref<Canteen[]>([]);
const picked = ref<string[]>([]);
const mealCanteen = ref("");
const anchor = ref(shanghaiToday());
const weekAnchor = ref(weekStart(shanghaiToday()));
const meals = ref<MealItem[]>([]);
const description = ref("");
const templateId = ref<string | null>(null);
const preview = ref<PreviewDay[]>([]);
const scoreDate = ref(shanghaiToday());
const scoreSlot = ref<Slot>("lunch");
const scoreKind = ref<Kind>("菜");
const windowOpen = ref(false);
const targets = ref<Groups<Target>>({ 菜: [], 饭: [], 汤: [] });
const drafts = reactive<Record<string, { value: number | null; comment: string; anonymous: boolean }>>({});
const devices = ref<Device[]>([]);
const passkeys = ref<Passkey[]>([]);
const pending = ref<Approval[]>([]);
const notes = ref<Note[]>([]);

const weekDays = computed(() => Array.from({ length: 7 }, (_, index) => addDays(weekAnchor.value, index)));
const shiftLabel = computed(() => {
  if (shift.value?.kind === "work2_rest4") return "上二休四";
  if (shift.value?.kind === "custom") return "自定义排班";
  return "";
});
const title = computed(() => {
  if (tab.value === "score") return "评分";
  if (screen.value === "shift") return "选择班制";
  if (screen.value === "week") return "本周报餐";
  if (screen.value === "generate") return "用描述生成模板";
  if (screen.value === "preview") return "模板预览";
  if (screen.value === "bind") return "绑定食堂";
  if (screen.value === "devices") return "设备与 Passkey";
  if (screen.value === "notices") return "站内提示";
  if (tab.value === "mine") return "我的";
  return "报餐";
});
const canBack = computed(() => screen.value !== "home" && screen.value !== "mine");
const unread = computed(() => notes.value.filter((item) => !item.read_at).length);

function toastError(error: unknown) {
  showFailToast(explain(error));
}

function cell(date: string, slot: Slot) {
  return meals.value.find((item) => item.date === date && item.slot === slot);
}

function canteenName(id: string | null) {
  return bound.value.find((item) => item.id === id)?.name ?? "";
}

function draftOf(id: string) {
  if (!drafts[id]) drafts[id] = { value: null, comment: "", anonymous: false };
  return drafts[id];
}

function back() {
  if (screen.value === "generate" || screen.value === "preview") screen.value = "shift";
  else if (tab.value === "mine") screen.value = "mine";
  else screen.value = "home";
}

function openTab(next: Tab) {
  tab.value = next;
  if (next === "meal") screen.value = "home";
  if (next === "mine") screen.value = "mine";
}

async function loadBound() {
  const body = await api<{ items: Canteen[] }>("/api/me/canteens");
  bound.value = body.items;
  if (!bound.value.some((item) => item.id === mealCanteen.value)) {
    mealCanteen.value = bound.value[0]?.id ?? "";
  }
}

async function loadShift() {
  shift.value = await api<Shift>("/api/me/shift");
}

async function loadHome() {
  await Promise.all([loadShift(), loadNotes()]);
}

async function loadWeek() {
  await loadBound();
  const start = weekAnchor.value;
  const end = addDays(start, 6);
  const body = await api<{ items: MealItem[] }>(`/api/me/meals?start=${start}&end=${end}`);
  meals.value = body.items;
}

async function loadPreview() {
  await loadBound();
  const body = await api<{ id: string | null; preview: PreviewDay[] }>("/api/me/meal-template");
  templateId.value = body.id;
  preview.value = body.preview ?? [];
}

async function loadBind() {
  const [all, mine] = await Promise.all([
    api<{ items: Canteen[] }>("/api/canteens"),
    api<{ items: Canteen[] }>("/api/me/canteens"),
  ]);
  allCanteens.value = all.items;
  picked.value = mine.items.map((item) => item.id);
  bound.value = mine.items;
}

async function loadDevices() {
  const body = await api<{ devices: Device[]; passkeys: Passkey[] }>("/api/devices");
  devices.value = body.devices;
  passkeys.value = body.passkeys;
  const approvals = await api<{ items: Approval[] }>("/api/login-approvals/pending");
  pending.value = approvals.items;
}

async function loadNotes() {
  const body = await api<{ items: Note[] }>("/api/notifications");
  notes.value = body.items;
}

async function loadScores() {
  const body = await api<{ window_open: boolean; groups: Groups<Target> }>(
    `/api/me/score-targets?date=${scoreDate.value}&slot=${scoreSlot.value}`,
  );
  windowOpen.value = body.window_open;
  targets.value = body.groups;
}

async function refresh() {
  try {
    if (tab.value === "score") {
      await loadScores();
      return;
    }
    if (screen.value === "home") await loadHome();
    else if (screen.value === "shift") {
      await Promise.all([loadShift(), loadBound()]);
    } else if (screen.value === "week") await loadWeek();
    else if (screen.value === "generate") await loadBound();
    else if (screen.value === "preview") await loadPreview();
    else if (screen.value === "bind") await loadBind();
    else if (screen.value === "devices") await loadDevices();
    else if (screen.value === "notices" || screen.value === "mine") await loadNotes();
  } catch (error) {
    toastError(error);
  }
}

async function chooseWork2() {
  if (!anchor.value) {
    showFailToast("上二休四需要锚点日");
    return;
  }
  try {
    await api("/api/me/shift", {
      method: "PUT",
      body: JSON.stringify({
        kind: "work2_rest4",
        anchor_date: anchor.value,
        canteen_id: mealCanteen.value || null,
      }),
    });
    showSuccessToast("已设为上二休四");
    screen.value = "preview";
  } catch (error) {
    toastError(error);
  }
}

async function chooseCustom() {
  try {
    await api("/api/me/shift", {
      method: "PUT",
      body: JSON.stringify({ kind: "custom", canteen_id: mealCanteen.value || null }),
    });
    showSuccessToast("已设为自定义排班");
    screen.value = "week";
  } catch (error) {
    toastError(error);
  }
}

async function generate() {
  try {
    await api("/api/meal-templates/generate", {
      method: "POST",
      body: JSON.stringify({
        description: description.value,
        anchor_date: anchor.value || null,
        canteen_id: mealCanteen.value || null,
      }),
    });
    showSuccessToast("模板已写入");
    screen.value = "preview";
  } catch (error) {
    toastError(error);
  }
}

async function toggleMeal(date: string, slot: Slot) {
  const current = cell(date, slot);
  if (current?.locked) {
    showFailToast("该餐已锁定");
    return;
  }
  const ordering = !(current?.ordering ?? false);
  if (ordering && !mealCanteen.value) {
    showFailToast("请先绑定并选择食堂");
    return;
  }
  try {
    await api(`/api/meals/${date}/${slot}`, {
      method: "PUT",
      body: JSON.stringify({ ordering, canteen_id: ordering ? mealCanteen.value : current?.canteen_id ?? null }),
    });
    await loadWeek();
  } catch (error) {
    toastError(error);
  }
}

async function togglePreview(day: PreviewDay, slot: Slot) {
  if (!templateId.value) return;
  const current = day.slots[slot];
  const ordering = !current.ordering;
  if (ordering && !mealCanteen.value) {
    showFailToast("报餐需要选择食堂");
    return;
  }
  try {
    const body = await api<{ ordering?: boolean; color: string; locked: boolean; order_updated: boolean }>(
      `/api/meal-templates/${templateId.value}/slots`,
      {
        method: "PUT",
        body: JSON.stringify({
          date: day.date,
          slot,
          ordering,
          canteen_id: ordering ? mealCanteen.value : current.canteen_id,
        }),
      },
    );
    current.ordering = ordering;
    current.color = body.color;
    if (body.locked && !body.order_updated) showFailToast("模板已改，该餐已锁定，报餐未改");
  } catch (error) {
    toastError(error);
  }
}

async function submitScore(dish: Target) {
  const draft = draftOf(dish.dish_id);
  if (draft.value === null) {
    showFailToast("请选择 −2 到 2");
    return;
  }
  try {
    await api("/api/scores", {
      method: "POST",
      body: JSON.stringify({
        meal_date: scoreDate.value,
        slot: scoreSlot.value,
        dish_id: dish.dish_id,
        value: draft.value,
        comment: draft.comment.trim() || null,
        anonymous: draft.anonymous,
      }),
    });
    showSuccessToast("已提交");
    await loadScores();
  } catch (error) {
    toastError(error);
  }
}

async function saveBind() {
  try {
    await api("/api/me/canteens", {
      method: "PUT",
      body: JSON.stringify({ canteen_ids: picked.value }),
    });
    showSuccessToast("已绑定");
    screen.value = "mine";
  } catch (error) {
    toastError(error);
  }
}

async function kick(id: string) {
  try {
    await showConfirmDialog({ title: "踢下线", message: "这台设备上的会话会结束。" });
  } catch {
    return;
  }
  try {
    await api(`/api/devices/${id}`, { method: "DELETE" });
    await loadDevices();
  } catch (error) {
    toastError(error);
  }
}

async function unbind(id: string) {
  try {
    await api(`/api/passkeys/${id}`, { method: "DELETE" });
    await loadDevices();
  } catch (error) {
    toastError(error);
  }
}

async function decide(id: string, allow: boolean) {
  try {
    await api(`/api/login-approvals/${id}/${allow ? "allow" : "deny"}`, { method: "POST" });
    showSuccessToast(allow ? "已允许" : "已拒绝");
    await loadDevices();
  } catch (error) {
    toastError(error);
  }
}

async function readNote(item: Note) {
  if (item.read_at) return;
  try {
    await api(`/api/notifications/${item.id}/read`, { method: "POST" });
    item.read_at = new Date().toISOString();
  } catch (error) {
    toastError(error);
  }
}

function mealStyle(date: string, slot: Slot) {
  const item = cell(date, slot);
  if (item?.ordering) return { background: "#2d8a45", color: "#fff", borderColor: "#2d8a45" };
  if (item) return { background: "#e6a23c", color: "#fff", borderColor: "#e6a23c" };
  return { background: "#fff", color: "#374151", borderColor: "#e5e7eb" };
}

function previewStyle(color: string) {
  if (color === "green") return { background: "#2d8a45", color: "#fff" };
  if (color === "orange") return { background: "#e6a23c", color: "#fff" };
  return { background: "#fff", color: "#374151" };
}

watch([screen, tab, weekAnchor, scoreDate, scoreSlot], refresh);
onMounted(refresh);
</script>

<template>
  <div class="emp">
    <van-nav-bar :title="title" :left-arrow="canBack" @click-left="back" />
    <div class="body">
      <template v-if="tab === 'meal' && screen === 'home'">
        <div class="hero">
          <div class="plate" />
          <h2>{{ shiftLabel || "尚未选择班制" }}</h2>
          <p>{{ shiftLabel ? "可以改班制，也可以直接改本周报餐" : "默认不报任何一餐" }}</p>
        </div>
        <van-button type="primary" block round @click="screen = 'shift'">选择班制模板</van-button>
        <van-button block round class="gap" @click="screen = 'week'">本周手动报餐</van-button>
      </template>

      <template v-else-if="tab === 'meal' && screen === 'shift'">
        <p class="hint">只有上二休四和自定义排班。用描述生成模板是子流程，不是第三种班制。</p>
        <div v-if="bound.length" class="pills">
          <button
            v-for="item in bound"
            :key="item.id"
            type="button"
            class="pill"
            :class="{ on: item.id === mealCanteen }"
            @click="mealCanteen = item.id"
          >{{ item.name }}</button>
        </div>
        <p v-else class="hint">还没绑定食堂。上二休四仍可先设锚点，报餐前请到「我的」绑定。</p>
        <section class="panel">
          <h3>上二休四</h3>
          <label class="field-label">锚点日</label>
          <input v-model="anchor" type="date" class="date" />
          <van-button type="primary" block round @click="chooseWork2">使用上二休四</van-button>
        </section>
        <section class="panel">
          <h3>自定义排班</h3>
          <p class="hint">不自动生成休息日以外的报餐，之后在本周或模板预览里逐餐填写。</p>
          <van-button block round @click="chooseCustom">使用自定义排班</van-button>
        </section>
        <van-button block plain type="primary" @click="screen = 'generate'">用描述生成模板</van-button>
      </template>

      <template v-else-if="tab === 'meal' && screen === 'generate'">
        <van-notice-bar wrapable text="按总管理已保存的全站渠道顺序生成，并写入个人模板。员工不填写 key。" />
        <van-field v-model="description" type="textarea" rows="5" maxlength="2000" show-word-limit placeholder="描述你的班制" />
        <label class="field-label">锚点日（可选）</label>
        <input v-model="anchor" type="date" class="date" />
        <van-button type="primary" block round class="gap" @click="generate">生成并预览</van-button>
      </template>

      <template v-else-if="tab === 'meal' && screen === 'preview'">
        <p class="hint">绿色是报餐，橙色是不报。点一下写回个人模板；未锁定的报餐会一起更新。</p>
        <div v-if="bound.length" class="pills">
          <button
            v-for="item in bound"
            :key="item.id"
            type="button"
            class="pill"
            :class="{ on: item.id === mealCanteen }"
            @click="mealCanteen = item.id"
          >{{ item.name }}</button>
        </div>
        <van-empty v-if="!templateId" description="还没有个人模板" />
        <article v-for="day in preview" :key="day.date" class="panel">
          <h3>{{ weekdayLabel(day.date) }} {{ day.date.slice(5) }}</h3>
          <div class="pills">
            <button
              v-for="slot in SLOTS"
              :key="slot"
              type="button"
              class="pill"
              :style="previewStyle(day.slots[slot].color)"
              @click="togglePreview(day, slot)"
            >{{ SLOT_SHORT[slot] }}</button>
          </div>
        </article>
      </template>

      <template v-else-if="tab === 'meal' && screen === 'week'">
        <van-notice-bar wrapable text="跨食堂按当前选中的食堂生效。锁定时间一到，次日三餐一起锁定。" />
        <div class="pills">
          <button type="button" class="pill" @click="weekAnchor = addDays(weekAnchor, -7)">上一周</button>
          <button type="button" class="pill" @click="weekAnchor = addDays(weekAnchor, 7)">下一周</button>
        </div>
        <div v-if="bound.length" class="pills">
          <button
            v-for="item in bound"
            :key="item.id"
            type="button"
            class="pill"
            :class="{ on: item.id === mealCanteen }"
            @click="mealCanteen = item.id"
          >{{ item.name }}</button>
        </div>
        <p v-else class="hint">还没有绑定食堂，不能报餐。</p>
        <article v-for="day in weekDays" :key="day" class="panel">
          <h3>{{ weekdayLabel(day) }} {{ day.slice(5) }}</h3>
          <div class="pills">
            <button
              v-for="slot in SLOTS"
              :key="slot"
              type="button"
              class="pill"
              :style="mealStyle(day, slot)"
              @click="toggleMeal(day, slot)"
            >
              {{ SLOT_SHORT[slot] }}
              <small v-if="cell(day, slot)?.locked">锁定</small>
            </button>
          </div>
          <p v-if="cell(day, 'breakfast')?.canteen_id || cell(day, 'lunch')?.canteen_id || cell(day, 'dinner')?.canteen_id" class="hint">
            {{ SLOTS.filter((slot) => cell(day, slot)?.ordering).map((slot) => `${SLOT_SHORT[slot]} ${canteenName(cell(day, slot)?.canteen_id ?? null)}`).join(" · ") }}
          </p>
        </article>
      </template>

      <template v-else-if="tab === 'score'">
        <div class="filters">
          <input v-model="scoreDate" type="date" class="date" />
          <div class="pills">
            <button
              v-for="slot in SLOTS"
              :key="slot"
              type="button"
              class="pill"
              :class="{ on: scoreSlot === slot }"
              @click="scoreSlot = slot"
            >{{ SLOT_NAME[slot] }}</button>
          </div>
        </div>
        <van-notice-bar text="餐后一周内可评。每道餐品单独打分，评语挂在这一道上。" />
        <van-empty v-if="!windowOpen" description="这一餐的评分窗口还没打开" />
        <van-tabs v-else v-model:active="scoreKind">
          <van-tab v-for="kind in KINDS" :key="kind" :title="kind" :name="kind">
            <van-empty v-if="targets[kind].length === 0" description="这一类没有可评的餐品" />
            <article v-for="dish in targets[kind]" :key="dish.dish_id" class="panel dish">
              <Thumb :src="dish.image_url" />
              <div class="dish-main">
                <strong>{{ dish.name }}</strong>
                <div>
                  <van-tag v-for="tag in dish.taste_tags" :key="tag" plain type="success" class="tag">{{ tag }}</van-tag>
                </div>
                <p v-if="dish.scored" class="hint">已评分，不能再改</p>
                <template v-else>
                  <div class="pills">
                    <button
                      v-for="value in [-2, -1, 0, 1, 2]"
                      :key="value"
                      type="button"
                      class="pill"
                      :class="{ on: draftOf(dish.dish_id).value === value }"
                      @click="draftOf(dish.dish_id).value = value"
                    >{{ value }}</button>
                  </div>
                  <van-field
                    v-model="draftOf(dish.dish_id).comment"
                    type="textarea"
                    rows="2"
                    maxlength="300"
                    show-word-limit
                    placeholder="可选评语，随这条评分一起提交"
                  />
                  <van-checkbox v-model="draftOf(dish.dish_id).anonymous">匿名，管理端不展示账号</van-checkbox>
                  <van-button size="small" type="primary" round @click="submitScore(dish)">提交</van-button>
                </template>
              </div>
            </article>
          </van-tab>
        </van-tabs>
      </template>

      <template v-else-if="screen === 'mine'">
        <van-cell-group inset>
          <van-cell title="账号" :value="auth.me?.username" />
          <van-cell title="绑定食堂" is-link @click="screen = 'bind'" />
          <van-cell title="设备与 Passkey" is-link @click="screen = 'devices'" />
          <van-cell title="站内提示" is-link :value="unread ? `${unread} 条未读` : ''" @click="screen = 'notices'" />
        </van-cell-group>
        <p class="hint">绑定食堂只决定报餐可用范围，不是食堂管理范围。退出登录请到设备页踢掉本机。</p>
      </template>

      <template v-else-if="screen === 'bind'">
        <p class="hint">可以绑定多个食堂。这是报餐时的使用范围。</p>
        <van-checkbox-group v-model="picked">
          <van-cell-group inset>
            <van-cell v-for="item in allCanteens" :key="item.id" :title="item.name" clickable @click="picked = picked.includes(item.id) ? picked.filter((id) => id !== item.id) : [...picked, item.id]">
              <template #right-icon>
                <van-checkbox :name="item.id" @click.stop />
              </template>
            </van-cell>
          </van-cell-group>
        </van-checkbox-group>
        <van-button type="primary" block round class="gap" @click="saveBind">保存</van-button>
      </template>

      <template v-else-if="screen === 'devices'">
        <h3 class="block-title">待确认</h3>
        <van-empty v-if="pending.length === 0" description="没有待确认的新设备" />
        <article v-for="item in pending" :key="item.id" class="panel">
          <strong>{{ item.friendly_name }}</strong>
          <p class="hint">{{ item.ip }}</p>
          <p class="hint">{{ item.user_agent }}</p>
          <div class="pills">
            <van-button size="small" type="primary" @click="decide(item.id, true)">允许</van-button>
            <van-button size="small" @click="decide(item.id, false)">拒绝</van-button>
          </div>
        </article>
        <h3 class="block-title">设备</h3>
        <article v-for="item in devices" :key="item.id" class="panel">
          <strong>{{ item.friendly_name }}</strong>
          <p class="hint">{{ item.session_active ? "在线" : "已结束" }} · {{ item.ip }}</p>
          <p class="hint">{{ item.user_agent }}</p>
          <van-button size="small" type="danger" @click="kick(item.id)">踢下线</van-button>
        </article>
        <h3 class="block-title">Passkey</h3>
        <van-empty v-if="passkeys.length === 0" description="还没有绑定 Passkey" />
        <article v-for="item in passkeys" :key="item.id" class="panel row">
          <span>{{ item.friendly_name || "Passkey" }}</span>
          <van-button size="small" type="danger" plain @click="unbind(item.id)">解绑</van-button>
        </article>
      </template>

      <template v-else-if="screen === 'notices'">
        <van-empty v-if="notes.length === 0" description="没有站内提示" />
        <van-cell
          v-for="item in notes"
          :key="item.id"
          :title="item.title"
          :label="item.body"
          :value="item.read_at ? '已读' : '未读'"
          @click="readNote(item)"
        />
      </template>
    </div>
    <van-tabbar :model-value="tab" :fixed="false" active-color="#2d8a45" inactive-color="#6b7280">
      <van-tabbar-item name="meal" @click="openTab('meal')">报餐</van-tabbar-item>
      <van-tabbar-item name="score" @click="openTab('score')">评分</van-tabbar-item>
      <van-tabbar-item name="mine" @click="openTab('mine')">我的</van-tabbar-item>
    </van-tabbar>
  </div>
</template>

<style scoped>
.emp { height: 100%; min-height: 100%; display: flex; flex-direction: column; background: #f7f8fa; color: #1f2937; }
.body { flex: 1; overflow: auto; padding: 12px 12px 20px; }
.hero { text-align: center; padding: 28px 12px 18px; }
.plate {
  width: 88px; height: 88px; margin: 0 auto 12px; border-radius: 50%;
  background: #e8f5ec; box-shadow: inset 0 0 0 10px #f4fbf6;
}
.hero h2 { margin: 0 0 6px; font-size: 20px; }
.hero p, .hint { color: #6b7280; font-size: 13px; }
.gap { margin-top: 10px; }
.panel { background: #fff; border-radius: 12px; padding: 12px; margin: 10px 0; }
.panel h3, .block-title { margin: 8px 0; font-size: 15px; }
.field-label { display: block; margin: 8px 0 4px; font-size: 13px; color: #6b7280; }
.date { width: 100%; border: 1px solid #e5e7eb; border-radius: 8px; padding: 8px; margin-bottom: 10px; font: inherit; }
.pills { display: flex; flex-wrap: wrap; gap: 8px; margin: 8px 0; }
.pill {
  border: 1px solid #e5e7eb; background: #fff; border-radius: 999px; padding: 6px 12px; font: inherit; cursor: pointer;
}
.pill.on { background: #2d8a45; color: #fff; border-color: #2d8a45; }
.pill small { margin-left: 4px; }
.dish { display: flex; gap: 10px; align-items: flex-start; }
.dish-main { flex: 1; min-width: 0; }
.tag { margin-right: 4px; }
.row { display: flex; justify-content: space-between; align-items: center; }
.filters { margin-bottom: 8px; }
</style>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Moon, Sunny } from "@element-plus/icons-vue";
import { api, shanghaiToday, SLOT_NAME, SLOTS, type Slot } from "../api";
import { auth } from "../session";

const date = ref(shanghaiToday());
const counts = ref<Record<Slot, number>>({ breakfast: 0, lunch: 0, dinner: 0 });
const locked = ref(false);

const cards = computed(() =>
  SLOTS.map((slot) => ({
    slot,
    name: SLOT_NAME[slot],
    count: counts.value[slot] ?? 0,
    icon: slot === "dinner" ? "moon" : "sunny",
    accent: slot === "breakfast" ? "#2d8a45" : slot === "lunch" ? "#e6a23c" : "#8b5cf6",
    soft: slot === "breakfast" ? "#e8f5ec" : slot === "lunch" ? "#fdf6ec" : "#f3eefc",
  })),
);

async function load() {
  if (!auth.canteenId) return;
  const body = await api<{ locked: boolean; slots: Record<Slot, number> }>(
    `/api/canteens/${auth.canteenId}/headcount?date=${date.value}`,
  );
  locked.value = body.locked;
  counts.value = body.slots;
}

watch(() => [auth.canteenId, date.value], load, { immediate: true });
</script>

<template>
  <div>
    <h1 class="page-title">就餐人数总览</h1>
    <div class="filters">
      <el-date-picker v-model="date" type="date" value-format="YYYY-MM-DD" placeholder="日期" style="width: 160px" />
      <div class="lock-pill" :class="{ off: !locked }">
        <span class="dot" /> {{ locked ? "该日三餐已锁定（一次锁定）" : "该日三餐尚未锁定" }}
      </div>
    </div>
    <div class="cards">
      <div v-for="meal in cards" :key="meal.slot" class="stat">
        <div class="meal-label">
          <span class="ico" :style="{ background: meal.soft, color: meal.accent }">
            <el-icon v-if="meal.icon === 'sunny'" :size="16"><Sunny /></el-icon>
            <el-icon v-else :size="16"><Moon /></el-icon>
          </span>
          {{ meal.name }}
        </div>
        <div class="num">{{ meal.count }}</div>
        <div class="unit">有效报餐人数</div>
        <div class="wave" :style="{ background: `linear-gradient(90deg, ${meal.accent}, transparent)` }" />
      </div>
    </div>
    <div class="hint card">
      <strong>说明</strong>
      <p>人数为当前选中日期、食堂下的有效报餐人数。锁定按提前一天的配置钟点一次性锁定次日三餐，不按餐次分别锁定。</p>
    </div>
  </div>
</template>

<style scoped>
.filters { display: flex; gap: 10px; margin-bottom: 16px; align-items: center; flex-wrap: wrap; }
.lock-pill {
  margin-left: auto; display: inline-flex; align-items: center; gap: 8px;
  background: #e8f5ec; color: #246e37; border-radius: 999px; padding: 6px 14px; font-size: 13px; font-weight: 600;
}
.lock-pill.off { background: #f3f4f6; color: #4b5563; }
.dot { width: 8px; height: 8px; border-radius: 50%; background: currentColor; }
.cards { display: grid; grid-template-columns: repeat(3, 1fr); gap: 14px; margin-bottom: 16px; }
.stat { background: #fff; border: 1px solid #e8eaed; border-radius: 14px; padding: 18px 20px; }
.meal-label { display: flex; align-items: center; gap: 8px; font-weight: 600; color: #4b5563; }
.ico { width: 28px; height: 28px; border-radius: 8px; display: inline-flex; align-items: center; justify-content: center; }
.num { font-size: 48px; font-weight: 700; margin: 10px 0 2px; line-height: 1; }
.unit { font-size: 12px; color: #9ca3af; margin-bottom: 10px; }
.wave { height: 8px; border-radius: 999px; opacity: 0.55; }
.hint { padding: 16px 18px; }
.hint p { margin: 6px 0 0; color: #6b7280; font-size: 13px; }
@media (max-width: 900px) { .cards { grid-template-columns: 1fr; } .lock-pill { margin-left: 0; } }
</style>

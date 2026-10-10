<script setup lang="ts">
import { ref, watch } from "vue";
import { InfoFilled, Refresh } from "@element-plus/icons-vue";
import { api, KINDS, type Kind } from "../api";
import WarnIcon from "../components/WarnIcon.vue";
import { auth } from "../session";

interface AlertRow {
  dish_id: string;
  name: string;
  kind: Kind;
  sample_count: number;
  visible_mean: number;
  threshold: number;
  enabled: boolean;
  message: string;
}

const tab = ref<Kind>("菜");
const groups = ref<Record<Kind, AlertRow[]>>({ 菜: [], 饭: [], 汤: [] });
const threshold = ref<number | null>(null);

async function load() {
  if (!auth.canteenId) return;
  const body = await api<{ groups: Record<Kind, AlertRow[]>; threshold: number }>(
    `/api/canteens/${auth.canteenId}/alerts`,
  );
  groups.value = body.groups;
  threshold.value = body.threshold;
}

watch(() => auth.canteenId, load, { immediate: true });
</script>

<template>
  <div>
    <h1 class="page-title">强提醒</h1>
    <div class="alert-banner">
      <div class="alert-left">
        <WarnIcon :size="28" />
        <p class="alert-text"><strong>强提醒：以下菜品可见均分低于当前阈值。</strong> 请及时核查菜品质量与排菜；系统不自动下架。</p>
      </div>
      <el-button class="refresh-btn" @click="load"><el-icon><Refresh /></el-icon> 刷新</el-button>
    </div>
    <el-tabs v-model="tab">
      <el-tab-pane v-for="kind in KINDS" :key="kind" :label="`${kind}（${groups[kind].length}）`" :name="kind" />
    </el-tabs>
    <div class="card">
      <el-table :data="groups[tab]" :row-class-name="() => 'alert-row'">
        <el-table-column label="菜品名称" min-width="220">
          <template #default="{ row }">
            <div class="dish-cell">
              <WarnIcon :size="18" />
              <div>
                <div class="name">{{ row.name }}</div>
                <el-tag size="small" effect="plain" round>{{ row.kind }}</el-tag>
              </div>
            </div>
          </template>
        </el-table-column>
        <el-table-column prop="sample_count" label="样本量" width="100" align="center" />
        <el-table-column label="可见均分" width="120" align="center">
          <template #default="{ row }"><span class="score-down">{{ Number(row.visible_mean).toFixed(2) }} ↓</span></template>
        </el-table-column>
        <el-table-column label="当前阈值" width="120" align="center">
          <template #default> ≥ {{ threshold?.toFixed(2) }} </template>
        </el-table-column>
        <el-table-column label="是否启用" width="120" align="center">
          <template #default="{ row }"><span class="enabled">{{ row.enabled ? "已启用" : "已停用" }}</span></template>
        </el-table-column>
      </el-table>
      <div class="card-footer">
        <div class="info-box">
          <el-icon :size="18"><InfoFilled /></el-icon>
          <p><strong>系统不自动下架；</strong>{{ groups[tab][0]?.message || "阈值以下菜品仍可继续出品，请人工评估后再处理。" }}</p>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.alert-banner {
  display: flex; align-items: center; justify-content: space-between; gap: 16px;
  background: #fff0ee; border: 1.5px solid #f5b4ae; border-radius: 12px; padding: 16px 18px; margin-bottom: 18px;
}
.alert-left { display: flex; align-items: center; gap: 12px; }
.alert-text { margin: 0; color: #c62828; }
.refresh-btn { border-color: #f0a8a0 !important; color: #e53935 !important; background: #fff !important; }
.card { overflow: hidden; }
.card :deep(.alert-row td) { background: #fff7f6; }
.dish-cell { display: flex; align-items: center; gap: 10px; }
.name { font-weight: 600; margin-bottom: 4px; }
.score-down, .enabled { color: #e53935; font-weight: 700; }
.card-footer { padding: 16px; }
.info-box { display: flex; gap: 8px; background: #fff5f4; border: 1px solid #fad2ce; border-radius: 10px; padding: 12px 14px; color: #9f1239; }
.info-box p { margin: 0; }
</style>

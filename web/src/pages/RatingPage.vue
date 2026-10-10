<script setup lang="ts">
import { ref, watch } from "vue";
import { api, KINDS, type Kind } from "../api";
import Thumb from "../components/Thumb.vue";
import { auth } from "../session";

interface Summary {
  dish_id: string;
  name: string;
  kind: Kind;
  image_url: string | null;
  visible_mean: number | null;
  sample_count: number;
  counts: Record<string, number>;
}

const tab = ref<Kind>("菜");
const groups = ref<Record<Kind, Summary[]>>({ 菜: [], 饭: [], 汤: [] });

async function load() {
  if (!auth.canteenId) return;
  const body = await api<{ groups: Record<Kind, Summary[]> }>(
    `/api/admin/score-summaries?canteen_id=${auth.canteenId}`,
  );
  groups.value = body.groups;
}

watch(() => auth.canteenId, load, { immediate: true });
</script>

<template>
  <div>
    <h1 class="page-title">评分汇总</h1>
    <el-tabs v-model="tab">
      <el-tab-pane v-for="kind in KINDS" :key="kind" :label="kind" :name="kind" />
    </el-tabs>
    <div class="card">
      <el-table :data="groups[tab]">
        <el-table-column label="餐品" min-width="220">
          <template #default="{ row }">
            <div class="dish"><Thumb :src="row.image_url" /><span>{{ row.name }}</span></div>
          </template>
        </el-table-column>
        <el-table-column label="可见均分" width="120">
          <template #default="{ row }">{{ row.visible_mean == null ? "—" : Number(row.visible_mean).toFixed(2) }}</template>
        </el-table-column>
        <el-table-column prop="sample_count" label="样本量" width="100" />
        <el-table-column label="−2 / −1 / 0 / 1 / 2">
          <template #default="{ row }">
            {{ row.counts["-2"] }} / {{ row.counts["-1"] }} / {{ row.counts["0"] }} / {{ row.counts["1"] }} / {{ row.counts["2"] }}
          </template>
        </el-table-column>
      </el-table>
    </div>
  </div>
</template>

<style scoped>
.card { overflow: hidden; }
.dish { display: flex; align-items: center; gap: 10px; }
</style>

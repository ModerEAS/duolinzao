<script setup lang="ts">
import { ref, watch } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { api, explain, KINDS, type Dish, type Groups, type Kind } from "../api";
import Thumb from "../components/Thumb.vue";
import { auth } from "../session";

const emit = defineEmits<{ edit: [id: string | null] }>();
const groups = ref<Groups<Dish>>({ 菜: [], 饭: [], 汤: [] });
const tab = ref<Kind>("菜");

async function load() {
  if (!auth.canteenId) return;
  const body = await api<{ groups: Groups<Dish> }>(`/api/canteens/${auth.canteenId}/dishes`);
  groups.value = body.groups;
}

async function toggle(row: Dish) {
  if (row.enabled) {
    try {
      await ElMessageBox.confirm("须显式确认；强提醒不会自动停用。确定停用该菜？", "停用确认", {
        type: "warning",
        confirmButtonText: "确认停用",
        cancelButtonText: "取消",
      });
    } catch {
      return;
    }
  }
  try {
    await api(`/api/canteens/${row.canteen_id}/dishes/${row.id}`, {
      method: "PUT",
      body: JSON.stringify({
        name: row.name,
        kind: row.kind,
        taste_tags: row.taste_tags,
        enabled: !row.enabled,
      }),
    });
    ElMessage.success(row.enabled ? "已停用" : "已启用");
    await load();
  } catch (error) {
    ElMessage.error(explain(error));
  }
}

watch(() => auth.canteenId, load, { immediate: true });
</script>

<template>
  <div>
    <div class="head">
      <h1 class="page-title">菜谱</h1>
      <el-button type="primary" @click="emit('edit', null)">新建菜品</el-button>
    </div>
    <el-tabs v-model="tab">
      <el-tab-pane v-for="kind in KINDS" :key="kind" :label="kind" :name="kind" />
    </el-tabs>
    <div class="card">
      <el-table :data="groups[tab]">
        <el-table-column label="名称" min-width="240">
          <template #default="{ row }">
            <div class="dish">
              <Thumb :src="row.image_url" round />
              <div>
                <div class="name">{{ row.name }}</div>
                <div class="tags">
                  <el-tag v-for="tag in row.taste_tags" :key="tag" size="small" effect="plain" round>{{ tag }}</el-tag>
                </div>
              </div>
            </div>
          </template>
        </el-table-column>
        <el-table-column label="类型" width="90">
          <template #default="{ row }"><el-tag effect="plain">{{ row.kind }}</el-tag></template>
        </el-table-column>
        <el-table-column label="启用状态" width="120">
          <template #default="{ row }">
            <span :style="{ color: row.enabled ? '#2d8a45' : '#6b7280' }">{{ row.enabled ? "已启用" : "已停用" }}</span>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="160">
          <template #default="{ row }">
            <el-button link type="primary" @click="emit('edit', row.id)">编辑</el-button>
            <el-button link :type="row.enabled ? 'danger' : 'primary'" @click="toggle(row)">
              {{ row.enabled ? "停用" : "启用" }}
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </div>
  </div>
</template>

<style scoped>
.head { display: flex; align-items: center; justify-content: space-between; }
.card { overflow: hidden; }
.dish { display: flex; align-items: center; gap: 10px; }
.name { font-weight: 600; margin-bottom: 4px; }
.tags { display: flex; gap: 4px; flex-wrap: wrap; }
</style>

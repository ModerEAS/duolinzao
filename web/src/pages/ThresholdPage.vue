<script setup lang="ts">
import { onMounted, ref } from "vue";
import { ElMessage } from "element-plus";
import { api, explain } from "../api";

const mean = ref(0);
const samples = ref(0);

async function load() {
  const body = await api<{ mean_threshold: number; min_samples: number }>("/api/admin/settings/score-alert");
  mean.value = body.mean_threshold;
  samples.value = body.min_samples;
}

async function save() {
  try {
    await api("/api/admin/settings/score-alert", {
      method: "PUT",
      body: JSON.stringify({ mean_threshold: Number(mean.value), min_samples: Number(samples.value) }),
    });
    ElMessage.success("已保存");
  } catch (error) {
    ElMessage.error(explain(error));
  }
}

onMounted(load);
</script>

<template>
  <div>
    <h1 class="page-title">强提醒阈值</h1>
    <div class="card box">
      <el-alert type="warning" show-icon :closable="false" title="保存后只影响之后的提醒，不会自动下架。" />
      <el-form label-position="top" style="max-width: 360px">
        <el-form-item label="可见均分阈值">
          <el-input-number v-model="mean" :step="0.1" :precision="2" />
        </el-form-item>
        <el-form-item label="样本量下限">
          <el-input-number v-model="samples" :min="0" :step="1" />
        </el-form-item>
        <el-button type="primary" @click="save">保存</el-button>
      </el-form>
    </div>
  </div>
</template>

<style scoped>
.box { padding: 18px; display: flex; flex-direction: column; gap: 16px; }
</style>

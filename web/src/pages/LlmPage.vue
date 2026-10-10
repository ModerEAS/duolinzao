<script setup lang="ts">
import { onMounted, ref } from "vue";
import { ElMessage } from "element-plus";
import { api, explain } from "../api";

interface Channel {
  id?: string;
  kind: string;
  base_url: string;
  model: string;
  api_key: string;
  enabled: boolean;
  key_configured: boolean;
}

const kinds = [
  { value: "openai_chat", label: "OpenAI 旧接口" },
  { value: "openai_responses", label: "OpenAI 新接口" },
  { value: "anthropic", label: "Anthropic" },
  { value: "local_script", label: "本地脚本" },
];

const channels = ref<Channel[]>([]);

function blank(): Channel {
  return { kind: "openai_chat", base_url: "", model: "", api_key: "", enabled: true, key_configured: false };
}

async function load() {
  const body = await api<{ channels: Channel[] }>("/api/admin/llm-channels");
  channels.value = body.channels.map((item) => ({
    id: item.id,
    kind: item.kind,
    base_url: item.base_url ?? "",
    model: item.model ?? "",
    api_key: "",
    enabled: item.enabled,
    key_configured: item.key_configured,
  }));
}

function move(index: number, delta: number) {
  const next = index + delta;
  if (next < 0 || next >= channels.value.length) return;
  const list = channels.value.slice();
  const [item] = list.splice(index, 1);
  list.splice(next, 0, item);
  channels.value = list;
}

async function save() {
  for (const item of channels.value) {
    if (item.kind !== "local_script" && !item.id && !item.api_key.trim()) {
      ElMessage.error("新的远程渠道需要填写 key");
      return;
    }
  }
  try {
    const body = await api<{ channels: Channel[] }>("/api/admin/llm-channels", {
      method: "PUT",
      body: JSON.stringify({
        channels: channels.value.map((item) => ({
          id: item.id,
          kind: item.kind,
          base_url: item.base_url || null,
          model: item.model || null,
          api_key: item.api_key.trim() ? item.api_key.trim() : null,
          enabled: item.enabled,
        })),
      }),
    });
    channels.value = body.channels.map((item) => ({
      id: item.id,
      kind: item.kind,
      base_url: item.base_url ?? "",
      model: item.model ?? "",
      api_key: "",
      enabled: item.enabled,
      key_configured: item.key_configured,
    }));
    ElMessage.success("渠道已保存");
  } catch (error) {
    ElMessage.error(explain(error));
  }
}

onMounted(load);
</script>

<template>
  <div>
    <div class="head">
      <h1 class="page-title">LLM 渠道</h1>
      <el-button @click="channels.push(blank())">添加渠道</el-button>
    </div>
    <p class="muted">顺序即调用顺序。停用的渠道会跳过。key 只在填写时更新，页面不回显原文。</p>
    <div v-for="(item, index) in channels" :key="item.id || index" class="card box">
      <div class="row">
        <strong>第 {{ index + 1 }} 位</strong>
        <el-button size="small" :disabled="index === 0" @click="move(index, -1)">上移</el-button>
        <el-button size="small" :disabled="index === channels.length - 1" @click="move(index, 1)">下移</el-button>
        <el-switch v-model="item.enabled" active-text="启用" inactive-text="停用" />
        <el-button link type="danger" @click="channels.splice(index, 1)">移除</el-button>
      </div>
      <el-form label-position="top">
        <el-form-item label="种类">
          <el-select v-model="item.kind" style="width: 240px">
            <el-option v-for="kind in kinds" :key="kind.value" :label="kind.label" :value="kind.value" />
          </el-select>
        </el-form-item>
        <el-form-item v-if="item.kind !== 'local_script'" label="Base URL">
          <el-input v-model="item.base_url" placeholder="兼容服务地址" />
        </el-form-item>
        <el-form-item v-if="item.kind !== 'local_script'" label="模型名">
          <el-input v-model="item.model" />
        </el-form-item>
        <el-form-item v-if="item.kind !== 'local_script'" label="Key">
          <el-input v-model="item.api_key" type="password" show-password :placeholder="item.key_configured ? '已配置，留空则保持' : '尚未配置'" />
        </el-form-item>
      </el-form>
    </div>
    <el-button type="primary" @click="save">保存顺序与渠道</el-button>
  </div>
</template>

<style scoped>
.head { display: flex; justify-content: space-between; align-items: center; }
.box { padding: 16px; margin-bottom: 12px; }
.row { display: flex; gap: 8px; align-items: center; margin-bottom: 8px; }
</style>

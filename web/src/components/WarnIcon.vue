<script setup lang="ts">
withDefaults(defineProps<{ size?: number | string }>(), { size: 24 });
</script>

<template>
  <svg class="warn-icon" :width="size" :height="size" viewBox="0 0 24 24" fill="none" aria-hidden="true">
    <path d="M12 3.1 L21.7 20.2 H2.3 Z" fill="#ffe8e6" stroke="#e53935" stroke-width="1.85" stroke-linejoin="round" />
    <rect x="10.85" y="8.2" width="2.3" height="6.4" rx="1.05" fill="#e53935" />
    <circle cx="12" cy="17.35" r="1.35" fill="#e53935" />
  </svg>
</template>

<style scoped>
.warn-icon { display: inline-block; flex-shrink: 0; vertical-align: middle; }
</style>

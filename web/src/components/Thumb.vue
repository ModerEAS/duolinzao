<script setup lang="ts">
defineProps<{ src?: string | null; round?: boolean }>();
</script>

<template>
  <img v-if="src" :src="src" alt="" class="thumb" :class="{ round }" />
  <div v-else class="thumb ph" :class="{ round }" />
</template>

<style scoped>
.round { border-radius: 50%; }
</style>

<script setup lang="ts">
defineProps<{ title: string; description?: string }>()
</script>

<template>
  <header class="page-header">
    <div class="page-header-copy">
      <h1>{{ title }}</h1>
      <p v-if="description">{{ description }}</p>
    </div>
    <div v-if="$slots.default" class="page-header-actions"><slot /></div>
  </header>
</template>

<style scoped>
.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin: 0 auto 24px;
  padding-bottom: 0;
  width: 100%;
  max-width: 1040px;
}
.page-header-copy { min-width: 0; }
h1 { margin: 0; font-size: 24px; line-height: 1.3; font-weight: 600; color: var(--ink); }
p { margin: 4px 0 0; font-size: 14px; line-height: 1.5; color: var(--muted); }
.page-header-actions { display: flex; align-items: center; gap: 8px; flex-shrink: 0; }
@media (max-width: 740px) {
  .page-header { gap: 12px; margin-bottom: 20px; }
}
</style>

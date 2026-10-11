<script setup lang="ts">
defineProps<{ open?: boolean }>()
</script>

<template>
  <svg class="disclosure-chevron" :class="{ open }" width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true">
    <path d="m6 9 6 6 6-6" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" />
  </svg>
</template>

<style scoped>
.disclosure-chevron { display: block; flex-shrink: 0; color: var(--subtle); transition: transform 180ms var(--ease-out); }
.disclosure-chevron.open { transform: rotate(180deg); }
</style>

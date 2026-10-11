<script setup lang="ts">
defineProps<{ open: boolean }>()
</script>

<template>
  <div class="app-collapsible" :class="{ open }" :inert="!open" :aria-hidden="!open">
    <div class="app-collapsible-clip"><div class="app-collapsible-content"><slot /></div></div>
  </div>
</template>

<style scoped>
.app-collapsible { display: grid; grid-template-rows: 0fr; transition: grid-template-rows 180ms var(--ease-out); }
.app-collapsible.open { grid-template-rows: 1fr; }
.app-collapsible-clip { min-height: 0; overflow: hidden; }
.app-collapsible-content { min-width: 0; }
@media (prefers-reduced-motion: reduce) { .app-collapsible { transition: none; } }
</style>

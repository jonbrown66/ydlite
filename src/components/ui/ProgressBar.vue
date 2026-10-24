<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  /** Download progress 0-100, or null while the total is unknown. */
  percent: number | null
  state: 'running' | 'paused' | 'completed'
  label: string
}>()

const clamped = computed(() => props.percent === null ? null : Math.min(100, Math.max(0, props.percent)))
</script>

<template>
  <div
    class="progress-bar"
    :class="[`is-${state}`, { 'is-indeterminate': clamped === null }]"
    role="progressbar"
    :aria-label="label"
    :aria-valuenow="clamped === null ? undefined : Math.round(clamped)"
    aria-valuemin="0"
    aria-valuemax="100"
  >
    <div class="track">
      <div class="fill" :style="{ width: `${clamped ?? 0}%` }" />
    </div>
  </div>
</template>

<style scoped>
.progress-bar {
  --bar: var(--primary);
  --bar-soft: var(--primary-soft);
  margin-block: 10px 0;
}

.progress-bar.is-paused { --bar: var(--subtle); --bar-soft: var(--surface-hover); }
.progress-bar.is-completed { --bar: var(--success); --bar-soft: var(--success-soft); }

.track {
  position: relative;
  block-size: 6px;
  overflow: hidden;
  border-radius: 99px;
  background: var(--bar-soft);
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--bar) 10%, transparent);
}

.fill {
  position: relative;
  block-size: 100%;
  overflow: hidden;
  border-radius: inherit;
  background: var(--bar);
  transition: width 240ms var(--ease-out), background 200ms ease-out;
}

/* A subtle sheen marks an active transfer without a separate spinner. */
.is-running .fill::after {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: inherit;
  background: linear-gradient(90deg, transparent, color-mix(in srgb, white 32%, transparent), transparent);
  animation: bar-sheen 1.8s var(--ease-out) infinite;
}

.is-indeterminate .fill {
  inline-size: 32%;
  animation: bar-indeterminate 1.4s ease-in-out infinite;
}

.is-paused .fill { transition: none; }
.is-completed .fill { transition: width 240ms var(--ease-out), background 200ms ease-out; }

@keyframes bar-sheen {
  0% { transform: translateX(-100%); }
  60%, 100% { transform: translateX(220%); }
}

@keyframes bar-indeterminate {
  0% { transform: translateX(-110%); }
  100% { transform: translateX(320%); }
}

@media (prefers-reduced-motion: reduce) {
  .is-running .fill::after,
  .is-indeterminate .fill { animation: none; }
  .is-indeterminate .fill { inline-size: 32%; transform: none; opacity: 0.6; }
  .fill { transition: none; }
}
</style>

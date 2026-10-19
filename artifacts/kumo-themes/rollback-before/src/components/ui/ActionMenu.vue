<script setup lang="ts">
import { nextTick, onBeforeUnmount, onDeactivated, ref, useId } from 'vue'
import { Button } from './button'
import moreIcon from 'iconoir/icons/more-horiz.svg?url'
const moreIconStyle = { '--icon-url': `url("${moreIcon}")` }
const props = defineProps<{ label: string; items: Array<{ id: string; label: string; danger?: boolean; disabled?: boolean }> }>()
const emit = defineEmits<{ select: [id: string] }>()
const id = useId()
const open = ref(false)
const trigger = ref<HTMLButtonElement | null>(null)
const panel = ref<HTMLElement | null>(null)
const style = ref<Record<string, string>>({})
async function show(event: Event) {
  if (open.value) { close(); return }
  trigger.value = event.currentTarget as HTMLButtonElement
  const box = trigger.value.getBoundingClientRect()
  const width = Math.min(264, innerWidth - 16)
  const height = props.items.length * 36 + 8
  style.value = { left: `${Math.max(8, Math.min(box.right - width, innerWidth - width - 8))}px`, top: `${box.bottom + height + 8 <= innerHeight ? box.bottom + 6 : Math.max(8, box.top - height - 6)}px` }
  open.value = true
  await nextTick()
  const actualHeight = panel.value?.getBoundingClientRect().height ?? height
  style.value.top = `${box.bottom + actualHeight + 8 <= innerHeight ? box.bottom + 6 : Math.max(8, box.top - actualHeight - 6)}px`
  const first = panel.value?.querySelector<HTMLButtonElement>('button:not(:disabled)')
  if (first) first.focus()
  else panel.value?.focus()
  document.addEventListener('pointerdown', outside, true)
  window.addEventListener('resize', close)
  window.addEventListener('scroll', scroll, true)
}
function close() {
  if (!open.value) return
  open.value = false
  document.removeEventListener('pointerdown', outside, true)
  window.removeEventListener('resize', close)
  window.removeEventListener('scroll', scroll, true)
  trigger.value?.focus({ preventScroll: true })
}
function outside(event: Event) { if (!panel.value?.contains(event.target as Node) && !trigger.value?.contains(event.target as Node)) close() }
function scroll(event: Event) { if (!panel.value?.contains(event.target as Node)) close() }
function choose(value: string) { close(); emit('select', value) }
function keydown(event: KeyboardEvent) {
  const buttons = Array.from(panel.value?.querySelectorAll<HTMLButtonElement>('button:not(:disabled)') ?? [])
  if (!buttons.length && event.key !== 'Escape' && event.key !== 'Tab') return
  const index = buttons.indexOf(document.activeElement as HTMLButtonElement)
  if (event.key === 'Escape' || event.key === 'Tab') { if (event.key === 'Escape') event.preventDefault(); close(); return }
  if (['ArrowDown','ArrowUp','Home','End'].includes(event.key)) {
    event.preventDefault()
    const next = event.key === 'Home' ? 0 : event.key === 'End' ? buttons.length - 1 : (index + (event.key === 'ArrowDown' ? 1 : -1) + buttons.length) % buttons.length
    buttons[next]?.focus()
  }
}
onDeactivated(close)
onBeforeUnmount(close)
</script>

<template>
  <Button size="icon" variant="ghost" :aria-label="label" aria-haspopup="menu" :aria-expanded="open" :aria-controls="open ? id : undefined" @click="show" @keydown.down.prevent="show" @keydown.up.prevent="show"><i class="icon" aria-hidden="true" :style="moreIconStyle" /></Button>
  <Teleport to="body">
    <Transition name="menu">
      <div v-if="open" :id="id" ref="panel" class="action-menu" role="menu" tabindex="-1" :aria-label="label" :style="style" @keydown="keydown">
        <button v-for="item in items" :key="item.id" role="menuitem" tabindex="-1" :disabled="item.disabled" :class="{ danger: item.danger }" @click="choose(item.id)">{{ item.label }}</button>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.action-menu { position: fixed; z-index: 500; width: min(264px, calc(100vw - 16px)); max-height: calc(100dvh - 16px); overflow-y: auto; padding: 4px; border-radius: 12px; background: var(--surface); box-shadow: 0 0 0 1px var(--border), var(--shadow-float); }
.action-menu button { display: block; width: 100%; min-height: 36px; padding: 0 12px; border: 0; border-radius: 8px; background: transparent; color: var(--ink); font-size: 14px; text-align: left; }
.action-menu button:hover, .action-menu button:focus { background: var(--surface-hover); }
.action-menu button:focus-visible { outline-offset: -2px; }
.action-menu button.danger { color: var(--danger); }
.action-menu button.danger:hover, .action-menu button.danger:focus { background: var(--danger-soft); }
.action-menu button:disabled { opacity: 0.45; cursor: not-allowed; }
.action-menu button:disabled:hover { background: transparent; }
.menu-enter-active, .menu-leave-active { transition: opacity 100ms ease-out, transform 120ms var(--ease-out); }
.menu-enter-from, .menu-leave-to { opacity: 0; transform: translateY(-4px); }
@media (prefers-reduced-motion: reduce) { .menu-enter-active, .menu-leave-active { transition: none; } }
</style>

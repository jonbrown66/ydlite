<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import checkIcon from 'iconoir/icons/check.svg?url'
import chevronIcon from 'iconoir/icons/nav-arrow-down.svg?url'

export type AppSelectOption = {
  value: string | number
  label: string
}

const props = defineProps<{
  modelValue: string | number
  options: AppSelectOption[]
  disabled?: boolean
  ariaLabel?: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string | number]
}>()

const root = ref<HTMLElement | null>(null)
const trigger = ref<HTMLButtonElement | null>(null)
const menu = ref<HTMLElement | null>(null)
const open = ref(false)
const activeIndex = ref(0)
const selectedIndex = computed(() => {
  const index = props.options.findIndex(option => option.value === props.modelValue)
  return index < 0 ? 0 : index
})
const selectedLabel = computed(() =>
  props.options[selectedIndex.value]?.label ?? '请选择',
)

function iconStyle(url: string) {
  return { '--select-icon': `url("${url}")` }
}

function show() {
  if (props.disabled) return
  activeIndex.value = selectedIndex.value
  open.value = true
  scrollActiveOption()
}

function close(restoreFocus = false) {
  open.value = false
  if (restoreFocus) void nextTick(() => trigger.value?.focus())
}

function toggle() {
  if (open.value) close()
  else show()
}

function choose(option: AppSelectOption) {
  emit('update:modelValue', option.value)
  close(true)
}

function move(step: number) {
  if (!props.options.length) return
  activeIndex.value = (activeIndex.value + step + props.options.length) % props.options.length
  scrollActiveOption()
}

function scrollActiveOption() {
  void nextTick(() => {
    const option = menu.value?.querySelectorAll<HTMLElement>('.app-select-option')[activeIndex.value]
    option?.scrollIntoView({ block: 'nearest' })
  })
}

function onKeydown(event: KeyboardEvent) {
  if (props.disabled) return
  if (!open.value && ['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(event.key)) {
    event.preventDefault()
    show()
    return
  }
  if (!open.value) return
  if (event.key === 'ArrowDown') {
    event.preventDefault()
    move(1)
  } else if (event.key === 'ArrowUp') {
    event.preventDefault()
    move(-1)
  } else if (event.key === 'Home') {
    event.preventDefault()
    activeIndex.value = 0
    scrollActiveOption()
  } else if (event.key === 'End') {
    event.preventDefault()
    activeIndex.value = Math.max(0, props.options.length - 1)
    scrollActiveOption()
  } else if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault()
    const option = props.options[activeIndex.value]
    if (option) choose(option)
  } else if (event.key === 'Escape') {
    event.preventDefault()
    close(true)
  } else if (event.key === 'Tab') {
    close()
  }
}

function onPointerDown(event: PointerEvent) {
  if (!root.value?.contains(event.target as Node)) close()
}

onMounted(() => document.addEventListener('pointerdown', onPointerDown))
onBeforeUnmount(() => document.removeEventListener('pointerdown', onPointerDown))
</script>

<template>
  <div ref="root" class="app-select" :class="{ open, disabled }">
    <button
      ref="trigger"
      class="app-select-trigger"
      type="button"
      :disabled="disabled"
      :aria-label="ariaLabel"
      aria-haspopup="listbox"
      :aria-expanded="open"
      :aria-activedescendant="open ? `select-option-${activeIndex}` : undefined"
      @click="toggle"
      @keydown="onKeydown"
    >
      <span>{{ selectedLabel }}</span>
      <i class="select-chevron" :style="iconStyle(chevronIcon)" />
    </button>

    <Transition name="select-pop">
      <div v-if="open" ref="menu" class="app-select-menu" role="listbox" :aria-label="ariaLabel">
        <button
          v-for="(option, index) in options"
          :id="`select-option-${index}`"
          :key="option.value"
          class="app-select-option"
          :class="{ selected: option.value === modelValue, active: index === activeIndex }"
          type="button"
          role="option"
          :aria-selected="option.value === modelValue"
          @mouseenter="activeIndex = index"
          @click="choose(option)"
        >
          <span>{{ option.label }}</span>
          <i v-if="option.value === modelValue" class="select-check" :style="iconStyle(checkIcon)" />
        </button>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.app-select {
  position: relative;
  width: 100%;
  min-width: 0;
}

.app-select-trigger {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  width: 100%;
  height: 42px;
  padding: 0 11px 0 13px;
  border: 1px solid var(--workspace-border);
  border-radius: 8px;
  outline: 0;
  background: var(--workspace-surface);
  color: var(--workspace-ink);
  font-size: 13px;
  font-weight: 600;
  text-align: left;
  cursor: pointer;
  transition: border-color 140ms ease-out, background-color 140ms ease-out, box-shadow 140ms ease-out;
}

.app-select-trigger:hover:not(:disabled) {
  border-color: var(--workspace-border-strong);
  background: var(--workspace-surface-muted);
}

.app-select.open .app-select-trigger,
.app-select-trigger:focus-visible {
  border-color: var(--workspace-accent);
  background: var(--workspace-surface);
  box-shadow: 0 0 0 3px color-mix(in oklch, var(--workspace-accent) 14%, transparent);
}

.app-select-trigger:disabled {
  opacity: 0.48;
  cursor: not-allowed;
}

.select-chevron,
.select-check {
  display: block;
  flex: 0 0 auto;
  width: 15px;
  height: 15px;
  background: currentColor;
  mask: var(--select-icon) center / contain no-repeat;
  -webkit-mask: var(--select-icon) center / contain no-repeat;
}

.select-chevron {
  color: var(--workspace-subtle);
  transition: transform 160ms var(--workspace-ease);
}

.app-select.open .select-chevron {
  transform: rotate(180deg);
}

.app-select-menu {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
  left: 0;
  z-index: 40;
  display: grid;
  gap: 2px;
  padding: 5px;
  max-height: min(248px, calc(100vh - 96px));
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
  scrollbar-color: var(--workspace-border-strong) transparent;
  scrollbar-width: thin;
  border: 1px solid var(--workspace-border);
  border-radius: 9px;
  background: var(--workspace-surface);
  box-shadow: var(--workspace-shadow);
}

.app-select-menu::-webkit-scrollbar {
  width: 7px;
}

.app-select-menu::-webkit-scrollbar-thumb {
  border: 2px solid transparent;
  border-radius: 99px;
  background: var(--workspace-border-strong);
  background-clip: padding-box;
}

.app-select-option {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  width: 100%;
  min-height: 36px;
  padding: 0 9px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--workspace-muted);
  font-size: 12px;
  text-align: left;
  cursor: pointer;
}

.app-select-option.active {
  background: var(--workspace-surface-muted);
  color: var(--workspace-ink);
}

.app-select-option.selected {
  color: var(--workspace-accent);
  font-weight: 700;
}

.select-check {
  color: var(--workspace-accent);
}

.select-pop-enter-active,
.select-pop-leave-active {
  transition: opacity 120ms ease-out, transform 150ms var(--workspace-ease);
}

.select-pop-enter-from,
.select-pop-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}

@media (prefers-reduced-motion: reduce) {
  .select-chevron,
  .select-pop-enter-active,
  .select-pop-leave-active {
    transition: none;
  }
}
</style>

<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, useId, watch } from 'vue'

const props = defineProps<{ open: boolean; title: string; busy?: boolean }>()
const emit = defineEmits<{ close: [] }>()
const titleId = useId()
const panel = ref<HTMLElement | null>(null)
let previousFocus: HTMLElement | null = null
const inertElements = new Map<HTMLElement, boolean>()

function focusable() {
  return Array.from(panel.value?.querySelectorAll<HTMLElement>(
    'button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), a[href], [tabindex="0"]',
  ) ?? []).filter(element => element.getClientRects().length > 0)
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    event.preventDefault()
    event.stopPropagation()
    if (!props.busy) emit('close')
  } else if (event.key === 'Tab') {
    const elements = focusable()
    const first = elements[0]
    const last = elements.at(-1)
    if (!first) { event.preventDefault(); panel.value?.focus(); return }
    if (event.shiftKey && (document.activeElement === first || document.activeElement === panel.value)) {
      event.preventDefault(); last?.focus()
    } else if (!event.shiftKey && (document.activeElement === last || document.activeElement === panel.value)) {
      event.preventDefault(); first.focus()
    }
  }
}

function release() {
  document.removeEventListener('keydown', onKeydown, true)
  inertElements.forEach((wasInert, element) => { element.inert = wasInert })
  inertElements.clear()
  if (previousFocus?.isConnected) previousFocus.focus()
  previousFocus = null
}

watch(() => props.open, async open => {
  if (!open) { release(); return }
  previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null
  await nextTick()
  if (!props.open || !panel.value) return
  for (const child of Array.from(document.body.children)) {
    if (child instanceof HTMLElement && !child.contains(panel.value)) {
      inertElements.set(child, child.inert)
      child.inert = true
    }
  }
  document.addEventListener('keydown', onKeydown, true)
  panel.value.focus()
}, { immediate: true })

onBeforeUnmount(release)
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="modal-backdrop" @click.self="!busy && emit('close')">
      <section ref="panel" class="modal" role="dialog" aria-modal="true" :aria-labelledby="titleId" :aria-busy="busy" tabindex="-1">
        <h2 :id="titleId">{{ title }}</h2>
        <slot />
      </section>
    </div>
  </Teleport>
</template>

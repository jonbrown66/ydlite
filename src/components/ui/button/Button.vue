<script setup lang="ts">
import type { HTMLAttributes } from 'vue'

type Variant = 'default' | 'primary' | 'outline' | 'ghost' | 'danger'
type Size = 'default' | 'sm' | 'icon'

const props = withDefaults(defineProps<{
  variant?: Variant
  size?: Size
  class?: HTMLAttributes['class']
  type?: 'button' | 'submit' | 'reset'
}>(), { type: 'button' })

const VARIANTS: Record<Variant, string> = {
  default: '',
  primary: 'primary',
  outline: 'outline',
  ghost: 'ghost',
  danger: 'danger',
}

function classes() {
  return [
    'button',
    VARIANTS[props.variant ?? 'default'],
    props.size === 'sm' ? 'sm' : props.size === 'icon' ? 'icon-only' : '',
    props.class,
  ].filter(Boolean).join(' ')
}
</script>

<template>
  <button :type="type" :class="classes()"><slot /></button>
</template>

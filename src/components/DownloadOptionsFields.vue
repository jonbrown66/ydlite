<script setup lang="ts">
import { computed } from 'vue'
import { AppSelect } from '@/components/ui/select'
import type { FileOptions } from '@/types'
const props = defineProps<{ modelValue: FileOptions; disabled?: boolean }>()
const emit = defineEmits<{ 'update:modelValue': [value: FileOptions] }>()
const naming = computed({ get: () => props.modelValue.naming, set: value => emit('update:modelValue', { ...props.modelValue, naming: value }) })
const conflict = computed({ get: () => props.modelValue.conflict, set: value => emit('update:modelValue', { ...props.modelValue, conflict: value }) })
const quality = computed({ get: () => props.modelValue.quality, set: value => emit('update:modelValue', { ...props.modelValue, quality: value }) })
</script>
<template>
  <div class="output-options">
    <label><span>Name</span><AppSelect v-model="naming" :disabled="disabled" aria-label="Filename" :options="[{value:'title',label:'Title'},{value:'authorTitle',label:'Author + title'},{value:'dateTitle',label:'Date + title'}]" /></label>
    <label><span>If exists</span><AppSelect v-model="conflict" :disabled="disabled" aria-label="Existing files" :options="[{value:'skip',label:'Skip'},{value:'number',label:'Keep both'},{value:'overwrite',label:'Replace'}]" /></label>
    <label><span>Quality</span><AppSelect v-model="quality" :disabled="disabled" aria-label="Quality preference" :options="[{value:'compatible',label:'Compatible · MP4'},{value:'quality',label:'Best available'}]" /></label>
  </div>
</template>
<style scoped>
.output-options { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 210px), 1fr)); gap: 14px; }
label { display: grid; gap: 6px; min-width: 0; font-size: 13px; }
</style>

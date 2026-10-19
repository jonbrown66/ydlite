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
    <label><span>Filename</span><AppSelect v-model="naming" :disabled="disabled" aria-label="Filename template" :options="[{value:'title',label:'Title'},{value:'authorTitle',label:'Author + title'},{value:'dateTitle',label:'Upload date + title'}]" /></label>
    <label><span>Already downloaded</span><AppSelect v-model="conflict" :disabled="disabled" aria-label="Duplicate handling" :options="[{value:'skip',label:'Keep existing file · skip'},{value:'number',label:'Save another copy · number'},{value:'overwrite',label:'Replace existing file'}]" /></label>
    <label><span>Automatic format selection</span><AppSelect v-model="quality" :disabled="disabled" aria-label="Format preference" :options="[{value:'compatible',label:'Compatibility · prefer MP4'},{value:'quality',label:'Quality · best available'}]" /></label>
    <p>Chinese titles are preserved. A media and format ID prevents unrelated files from sharing a name.</p>
    <p v-if="conflict === 'overwrite'" class="overwrite-note" role="status">Existing files for this media and format will be replaced. Downloads ask for confirmation before starting.</p>
  </div>
</template>
<style scoped>
.output-options { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 210px), 1fr)); gap: 14px; }
label { display: grid; gap: 6px; min-width: 0; font-size: 14px; }
p { grid-column: 1 / -1; margin: 0; color: var(--muted); font-size: 14px; line-height: 1.6; }
.overwrite-note { color: var(--danger); }
</style>

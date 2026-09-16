<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import SubtitleWorkspace from '@/SubtitleWorkspace.vue'

const route = useRoute()
const router = useRouter()
const sourcePath = computed(() => typeof route.query.source === 'string' ? route.query.source : '')
const projectId = computed(() => typeof route.query.project === 'string' ? route.query.project : '')

function clearInitialSource() {
  if (route.query.source) void router.replace({ name: 'subtitles' })
}

function clearInitialProject() {
  if (route.query.project) void router.replace({ name: 'subtitles' })
}
</script>

<template>
  <SubtitleWorkspace
    :initial-source-path="sourcePath"
    :initial-project-id="projectId"
    @consumed-initial="clearInitialSource"
    @consumed-project="clearInitialProject"
  />
</template>

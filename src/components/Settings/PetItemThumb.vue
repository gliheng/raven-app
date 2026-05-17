<script setup lang="ts">
import { ref, watch } from "vue"
import { invoke } from "@tauri-apps/api/core"

const props = defineProps<{ slug: string }>()
const bgImage = ref("")

watch(() => props.slug, async (slug) => {
  if (!slug) { bgImage.value = ""; return }
  try {
    const data = await invoke<{ data: string }>("read_pet_spritesheet", { slug })
    bgImage.value = `url('${data.data}')`
  } catch { bgImage.value = "" }
}, { immediate: true })
</script>

<template>
  <div
    class="w-6 h-6 bg-cover bg-no-repeat shrink-0 rounded"
    :style="{
      backgroundImage: bgImage,
      backgroundSize: '800% 900%',
      backgroundPosition: '0% 0%',
      imageRendering: 'pixelated',
    }"
  />
</template>

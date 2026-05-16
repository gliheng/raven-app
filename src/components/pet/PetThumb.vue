<script setup lang="ts">
import { ref, onMounted } from "vue"

const props = defineProps<{
  slug: string
  loader: (slug: string) => Promise<string>
}>()

const bgImage = ref("")

onMounted(async () => {
  const data = await props.loader(props.slug)
  if (data) {
    bgImage.value = `url('${data}')`
  }
})
</script>

<template>
  <div
    class="thumb"
    :style="{ backgroundImage: bgImage }"
  />
</template>

<style scoped>
.thumb {
  width: 40px;
  height: 40px;
  image-rendering: pixelated;
  background-repeat: no-repeat;
  background-size: 800% 900%;
  background-position: 0% 0%;
  background-color: rgba(255, 255, 255, 0.04);
  border-radius: 4px;
}
</style>

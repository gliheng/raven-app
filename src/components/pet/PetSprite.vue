<script setup lang="ts">
import { computed } from "vue"

const props = defineProps<{
  backgroundPosition: string
  spritesheetUrl: string
  dragging: boolean
}>()

const petStyle = computed(() => ({
  backgroundPosition: props.backgroundPosition,
  backgroundImage: `url('${props.spritesheetUrl}')`,
}))
</script>

<template>
  <div
    class="pet"
    :class="{ dragging }"
    :style="petStyle"
  >
    <slot />
  </div>
</template>

<style scoped>
.pet {
  aspect-ratio: 192 / 208;
  width: 4.5rem;
  image-rendering: pixelated;
  background-image: url('spritesheet.webp');
  background-repeat: no-repeat;
  background-size: 800% 900%;
  background-position: 0% 0%;
  cursor: grab;
}
.pet.dragging {
  cursor: grabbing;
}
</style>

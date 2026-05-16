<script setup lang="ts">
import { ref, computed, onMounted, nextTick } from "vue"
import { invoke } from "@tauri-apps/api/core"
import type { PetInfo, SpritesheetData } from "@/types/pet"
import PetThumb from "@/components/pet/PetThumb.vue"

const COLUMNS = 3
const ROW_HEIGHT = 64
const ROW_BUFFER = 2

defineProps<{
  activeSlug: string | null
}>()

const emit = defineEmits<{
  close: []
  select: [slug: string]
}>()

const pets = ref<PetInfo[]>([])
const search = ref("")
const visible = ref(false)
const scrollerRef = ref<HTMLElement | null>(null)
const scrollTop = ref(0)
const thumbnailCache = new Map<string, string>()

const filtered = computed(() => {
  if (!search.value) return pets.value
  const q = search.value.toLowerCase()
  return pets.value.filter(
    (p) =>
      p.slug.toLowerCase().includes(q) ||
      p.displayName.toLowerCase().includes(q)
  )
})

const totalRows = computed(() => Math.ceil(filtered.value.length / COLUMNS))

const visibleRange = computed(() => {
  const start = Math.max(
    0,
    Math.floor(scrollTop.value / ROW_HEIGHT) - ROW_BUFFER
  )
  const end = Math.min(
    totalRows.value,
    Math.ceil((scrollTop.value + 300) / ROW_HEIGHT) + ROW_BUFFER
  )
  return { start, end }
})

const visiblePets = computed(() => {
  const { start, end } = visibleRange.value
  const slice = filtered.value.slice(start * COLUMNS, end * COLUMNS)
  return {
    items: slice,
    start: start * COLUMNS,
    offsetTop: start * ROW_HEIGHT,
  }
})

async function loadThumb(slug: string): Promise<string> {
  if (thumbnailCache.has(slug)) return thumbnailCache.get(slug)!
  try {
    const data = await invoke<SpritesheetData>("read_pet_spritesheet", { slug })
    thumbnailCache.set(slug, data.data)
    return data.data
  } catch {
    return ""
  }
}

onMounted(async () => {
  try {
    pets.value = await invoke<PetInfo[]>("list_pets")
  } catch (e) {
    console.error("Failed to list pets:", e)
  }
  visible.value = true
  await nextTick()
  if (scrollerRef.value) {
    scrollerRef.value.focus()
  }
})

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") {
    emit("close")
  }
}

function onSelect(slug: string) {
  emit("select", slug)
}
</script>

<template>
  <div
    v-if="visible"
    class="menu"
    @keydown="onKeydown"
  >
    <input
      v-model="search"
      placeholder="Search pets..."
      autofocus
    />
    <div class="scroller" ref="scrollerRef" @scroll="scrollTop = ($event.target as HTMLElement).scrollTop">
      <div class="spacer" :style="{ height: totalRows * ROW_HEIGHT + 'px' }" />
      <div
        class="viewport"
        :style="{ transform: `translateY(${visiblePets.offsetTop}px)` }"
      >
        <div
          v-for="pet in visiblePets.items"
          :key="pet.slug"
          class="cell"
          :class="{ active: pet.slug === activeSlug }"
          :title="pet.displayName"
          @click="onSelect(pet.slug)"
        >
          <PetThumb :slug="pet.slug" :loader="loadThumb" />
          <div class="label">{{ pet.displayName }}</div>
        </div>
        <div v-if="filtered.length === 0" class="empty">
          No pets found
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.menu {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  background: rgba(20, 20, 22, 0.96);
  color: #f0f0f0;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  padding: 6px;
  font-size: 10px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
  z-index: 999;
  backdrop-filter: blur(16px);
  display: flex;
  flex-direction: column;
  gap: 6px;
  pointer-events: auto;
}
.menu input {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.08);
  color: #f0f0f0;
  border-radius: 5px;
  padding: 4px 8px;
  font-size: 10px;
  outline: none;
  font-family: inherit;
}
.menu input:focus {
  border-color: rgba(255, 255, 255, 0.2);
}
.scroller {
  position: relative;
  height: 240px;
  overflow-y: auto;
  overflow-x: hidden;
}
.scroller::-webkit-scrollbar {
  width: 6px;
}
.scroller::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.1);
  border-radius: 3px;
}
.spacer {
  width: 100%;
  pointer-events: none;
}
.viewport {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 4px;
  will-change: transform;
}
.cell {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 4px 2px;
  border-radius: 5px;
  cursor: pointer;
  gap: 2px;
  min-width: 0;
  height: 60px;
  box-sizing: border-box;
}
.cell:hover {
  background: rgba(255, 255, 255, 0.08);
}
.cell.active {
  background: rgba(0, 122, 255, 0.18);
  outline: 1px solid rgba(0, 122, 255, 0.4);
}
.label {
  font-size: 8px;
  color: rgba(255, 255, 255, 0.7);
  width: 100%;
  text-align: center;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.empty {
  color: rgba(255, 255, 255, 0.3);
  text-align: center;
  padding: 12px 0;
  font-size: 9px;
}
</style>

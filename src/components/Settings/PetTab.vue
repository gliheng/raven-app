<script setup lang="ts">
import { ref, onMounted, watch } from "vue"
import { usePetStore } from "@/stores/pet"
import type { PetInfo } from "@/types/pet"

const petStore = usePetStore()
const pets = ref<PetInfo[]>([])

onMounted(async () => {
  pets.value = await petStore.listPets()
})

async function togglePet() {
  await petStore.togglePet()
}

watch(() => petStore.activeSlug, async (slug) => {
  if (slug) {
    await petStore.setActive(slug)
    if (petStore.enabled) {
      await petStore.showPet()
    }
  }
})
</script>

<template>
  <div class="space-y-4 mt-4">
    <div class="flex items-center justify-between">
      <div>
        <p class="font-medium">Desktop Pet</p>
        <p class="text-sm text-gray-500">Show a pixel-art pet on your desktop</p>
      </div>
      <USwitch
        :model-value="petStore.enabled"
        @update:model-value="togglePet"
      />
    </div>

    <div v-if="petStore.enabled" class="space-y-3">
      <USeparator />

      <div>
        <p class="text-sm font-medium mb-1">Active Pet</p>
        <USelect
          :model-value="petStore.activeSlug ?? undefined"
          value-key="value"
          :items="pets.map(p => ({ label: p.displayName, value: p.slug }))"
          :disabled="pets.length === 0"
          placeholder="Select a pet..."
          @update:model-value="(v: string | undefined) => { if (v) petStore.activeSlug = v }"
        />
        <p v-if="pets.length === 0" class="text-xs text-gray-500 mt-1">
          No pets found. Install pets via <code>npx petdex install &lt;slug&gt;</code>
        </p>
      </div>

      <div class="bg-gray-50 dark:bg-gray-800 rounded-lg p-3 text-xs text-gray-500 space-y-1">
        <p>Pet spritesheets are loaded from <code>~/.petdex/pets/</code></p>
        <p>Install new pets with: <code>npx petdex install &lt;slug&gt;</code></p>
        <p>Right-click the pet to switch between installed pets</p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from "vue"
import { usePetStore } from "@/stores/pet"
import PetItemThumb from "@/components/Settings/PetItemThumb.vue"

const petStore = usePetStore()
const petItems = ref<{ label: string; value: string; slug: string }[]>([])

onMounted(async () => {
  const pets = await petStore.listPets()
  petItems.value = pets.map(p => ({ label: p.displayName, value: p.slug, slug: p.slug }))
})

async function togglePet() {
  await petStore.togglePet()
}

watch(() => petStore.activeSlug, (slug) => {
  if (slug) {
    petStore.setActive(slug)
  }
})

function onSelect(v: string | undefined) {
  if (v) petStore.activeSlug = v
}
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
          value-key="value"
          placeholder="Select a pet..."
          :model-value="petStore.activeSlug ?? undefined"
          :items="petItems"
          :disabled="petItems.length === 0"
          :ui="{ content: 'min-w-fit' }"
          @update:model-value="onSelect"
        >
          <template #item-leading="{ item }">
            <PetItemThumb :slug="item.slug" />
          </template>
          <template #leading>
            <PetItemThumb v-if="petStore.activeSlug" :slug="petStore.activeSlug" />
          </template>
        </USelect>
        <p v-if="petItems.length === 0" class="text-xs text-gray-500 mt-1">
          No pets found. Install pets via <code>npx petdex install &lt;slug&gt;</code>
        </p>
      </div>

      <div class="bg-gray-50 dark:bg-gray-800 rounded-lg p-3 text-xs text-gray-500 space-y-1">
        <p>Pet spritesheets are loaded from <code>~/.petdex/pets/</code></p>
        <p>Install new pets with: <code>npx petdex install &lt;slug&gt;</code></p>
      </div>
    </div>
  </div>
</template>

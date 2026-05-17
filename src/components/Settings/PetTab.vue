<script setup lang="ts">
import { ref, onMounted, watch } from "vue"
import { usePetStore } from "@/stores/pet"
import PetItemThumb from "@/components/Settings/PetItemThumb.vue"
import PetBrowserModal from "@/components/Settings/PetBrowserModal.vue"

const petStore = usePetStore()
const petItems = ref<{ label: string; value: string; slug: string }[]>([])
const browserOpen = ref(false)

onMounted(async () => {
  await refreshPetItems()
})

async function refreshPetItems() {
  const pets = await petStore.listPets()
  petItems.value = pets.map(p => ({ label: p.displayName, value: p.slug, slug: p.slug }))
}

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

function onPetInstalled() {
  refreshPetItems()
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
          No pets installed yet. Browse and install pets below.
        </p>
      </div>

      <UButton
        variant="soft"
        color="primary"
        icon="i-lucide-store"
        class="w-full"
        @click="browserOpen = true"
      >
        Browse Pets
      </UButton>

      <div class="bg-gray-50 dark:bg-gray-800 rounded-lg p-3 text-xs text-gray-500 space-y-1">
        <p>Pet spritesheets are loaded from <code>~/.codex/pets/</code> or the app data directory</p>
        <p>You can also install pets with: <code>npx petdex install &lt;slug&gt;</code></p>
      </div>

      <PetBrowserModal v-model:open="browserOpen" @installed="onPetInstalled" />
    </div>
  </div>
</template>

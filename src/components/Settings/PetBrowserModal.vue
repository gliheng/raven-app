<script setup lang="ts">
import { ref, computed, watch } from "vue"
import { invoke } from "@tauri-apps/api/core"
import { usePetStore } from "@/stores/pet"

interface ManifestPet {
  slug: string
  displayName: string
  kind: string
  submittedBy: string | null
  spritesheetUrl: string
  petJsonUrl: string
  zipUrl: string | null
}

const open = defineModel<boolean>("open", { default: false })

const emit = defineEmits<{
  installed: []
}>()

const petStore = usePetStore()
const pets = ref<ManifestPet[]>([])
const loading = ref(false)
const error = ref("")
const search = ref("")
const installing = ref<Set<string>>(new Set())
const installedSlugs = ref<Set<string>>(new Set())

const filteredPets = computed(() => {
  const q = search.value.toLowerCase().trim()
  if (!q) return pets.value
  return pets.value.filter(
    p => p.displayName.toLowerCase().includes(q) || p.slug.toLowerCase().includes(q)
  )
})

const installedCount = computed(() => installedSlugs.value.size)

async function fetchManifest() {
  loading.value = true
  error.value = ""
  try {
    const data = await invoke<{ pets: ManifestPet[] }>("fetch_pet_manifest")
    pets.value = data.pets
  } catch (e: any) {
    error.value = e?.message || "Failed to load pets"
  } finally {
    loading.value = false
  }
}

function refreshInstalledSlugs() {
  installedSlugs.value = new Set(petStore.pets.map(p => p.slug))
}

async function installPet(pet: ManifestPet) {
  installing.value.add(pet.slug)
  try {
    await invoke("install_pet", {
      slug: pet.slug,
      spritesheetUrl: pet.spritesheetUrl,
      petJsonUrl: pet.petJsonUrl,
    })
    installedSlugs.value.add(pet.slug)
    await petStore.refreshPets()
    emit("installed")
  } catch (e: any) {
    console.error("Install failed:", e)
  } finally {
    installing.value.delete(pet.slug)
  }
}

watch(open, (v) => {
  if (v) {
    refreshInstalledSlugs()
    fetchManifest()
  }
});
</script>

<template>
  <UModal
    v-model:open="open"
    title="Browse Pets"
    description="Discover and install desktop pets from Petdex"
    :ui="{ content: 'max-w-2xl' }"
  >
    <template #content>
      <div class="p-6 space-y-4">
        <div class="flex items-center gap-3">
          <UInput
            v-model="search"
            class="flex-1"
            placeholder="Search pets..."
            icon="i-lucide-search"
            :disabled="loading"
          />
          <span class="text-xs text-gray-500 whitespace-nowrap">
            {{ installedCount }} / {{ pets.length }} installed
          </span>
        </div>

        <div v-if="loading" class="flex items-center justify-center py-12">
          <UIcon name="i-lucide-loader-2" class="animate-spin size-6 text-gray-400" />
        </div>

        <div v-else-if="error" class="text-center py-12">
          <p class="text-sm text-red-500">{{ error }}</p>
          <UButton variant="soft" size="sm" class="mt-3" @click="fetchManifest">Retry</UButton>
        </div>

        <div v-else-if="filteredPets.length === 0" class="text-center py-12">
          <p class="text-sm text-gray-500">
            {{ search ? "No pets match your search" : "No pets available" }}
          </p>
        </div>

        <div v-else class="grid grid-cols-2 sm:grid-cols-3 gap-3 max-h-[50vh] overflow-y-auto pr-1">
          <div
            v-for="pet in filteredPets"
            :key="pet.slug"
            class="group relative rounded-lg border border-gray-200 dark:border-gray-700 p-3 flex flex-col items-center gap-2 hover:border-primary-400 dark:hover:border-primary-500 transition-colors"
          >
            <div
              class="w-16 h-16 rounded-md bg-contain bg-no-repeat bg-center"
              :style="{
                backgroundImage: `url('${pet.spritesheetUrl}')`,
                backgroundSize: '800% 900%',
                backgroundPosition: '0% 0%',
                imageRendering: 'pixelated',
              }"
            />
            <div class="text-center min-w-0 w-full">
              <p class="text-sm font-medium truncate">{{ pet.displayName }}</p>
              <div class="flex items-center justify-center gap-1 mt-1">
                <UBadge size="xs" variant="subtle" color="neutral">{{ pet.kind }}</UBadge>
                <span v-if="pet.submittedBy" class="text-[10px] text-gray-400 truncate">
                  by {{ pet.submittedBy }}
                </span>
              </div>
            </div>

            <UButton
              v-if="installedSlugs.has(pet.slug)"
              size="xs"
              variant="soft"
              color="success"
              icon="i-lucide-check"
              disabled
              class="w-full"
            >
              Installed
            </UButton>
            <UButton
              v-else
              size="xs"
              variant="soft"
              color="primary"
              icon="i-lucide-download"
              :loading="installing.has(pet.slug)"
              :disabled="installing.has(pet.slug)"
              class="w-full"
              @click="installPet(pet)"
            >
              Install
            </UButton>
          </div>
        </div>
      </div>
    </template>
  </UModal>
</template>

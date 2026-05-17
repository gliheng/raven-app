import { ref } from "vue"
import { defineStore } from "pinia"
import { invoke } from "@tauri-apps/api/core"
import { eventBus } from "@/utils/eventBus"

export interface PetInfo {
  slug: string
  displayName: string
}

export const usePetStore = defineStore("pet", () => {
  const enabled = ref(false)
  const activeSlug = ref<string | null>(null)
  const pets = ref<PetInfo[]>([])

  async function initialize() {
    try {
      const state = await invoke<{ activeSlug: string | null; enabled: boolean | null }>("get_pet_state")
      activeSlug.value = state.activeSlug ?? null
      if (state.enabled && activeSlug.value) {
        enabled.value = true
        eventBus.emit("set_pet", { enabled: true })
      }
    } catch {}
  }

  async function listPets(): Promise<PetInfo[]> {
    try {
      pets.value = await invoke<PetInfo[]>("list_pets")
      return pets.value
    } catch {
      return []
    }
  }

  function setActive(slug: string) {
    activeSlug.value = slug
    eventBus.emit("set_pet", { slug })
  }

  function showPet() {
    enabled.value = true
    eventBus.emit("set_pet", { enabled: true })
  }

  function hidePet() {
    enabled.value = false
    eventBus.emit("set_pet", { enabled: false })
  }

  function togglePet() {
    if (enabled.value) hidePet()
    else showPet()
  }

  return { enabled, activeSlug, pets, initialize, listPets, setActive, showPet, hidePet, togglePet }
})

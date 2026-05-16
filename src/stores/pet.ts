import { ref } from "vue"
import { defineStore } from "pinia"
import { invoke } from "@tauri-apps/api/core"
import type { PetInfo } from "@/types/pet"

export const usePetStore = defineStore("pet", () => {
  const enabled = ref(false)
  const activeSlug = ref<string | null>(null)
  const pets = ref<PetInfo[]>([])
  const position = ref({ x: 0, y: 0 })

  async function initialize() {
    try {
      const state = await invoke<{
        activeSlug: string | null
        x: number | null
        y: number | null
      }>("get_pet_state")
      activeSlug.value = state.activeSlug ?? null
      position.value = {
        x: state.x ?? 0,
        y: state.y ?? 0,
      }
    } catch (e) {
      console.error("[PetStore] Failed to initialize:", e)
    }
  }

  async function listPets(): Promise<PetInfo[]> {
    try {
      pets.value = await invoke<PetInfo[]>("list_pets")
      return pets.value
    } catch (e) {
      console.error("[PetStore] Failed to list pets:", e)
      return []
    }
  }

  async function setActive(slug: string) {
    activeSlug.value = slug
    await invoke("save_pet_state", {
      state: { slug, x: position.value.x, y: position.value.y },
    })
  }

  async function savePosition(x: number, y: number) {
    position.value = { x, y }
    await invoke("save_pet_state", {
      state: { slug: activeSlug.value, x, y },
    })
  }

  async function showPet() {
    enabled.value = true
    await invoke("create_pet_cmd")
  }

  async function hidePet() {
    enabled.value = false
    await invoke("destroy_pet_cmd")
  }

  async function togglePet() {
    if (enabled.value) {
      await hidePet()
    } else {
      await showPet()
    }
  }

  return {
    enabled,
    activeSlug,
    pets,
    position,
    initialize,
    listPets,
    setActive,
    savePosition,
    showPet,
    hidePet,
    togglePet,
  }
})

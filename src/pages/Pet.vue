<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from "vue"
import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import { getCurrentWindow, PhysicalPosition } from "@tauri-apps/api/window"
import type { SpritesheetData, PetAnimationState } from "@/types/pet"
import PetSprite from "@/components/pet/PetSprite.vue"
import PetBubble from "@/components/pet/PetBubble.vue"
import PetPicker from "@/components/pet/PetPicker.vue"
import { usePet } from "@/composables/usePet"

const TICK_MS = 16
const FRICTION = 0.88
const MIN_VEL = 65
const MAX_DURATION = 900
const SAMPLE_WINDOW_MS = 100
const THRESHOLD = 4

const { backgroundPosition, play } = usePet()

const petRef = ref<HTMLElement | null>(null)
const stageRef = ref<HTMLElement | null>(null)
const spritesheetUrl = ref("")
const showPicker = ref(false)
const activeSlug = ref<string | null>(null)

const dragging = ref(false)
let lastX = 0
let lastY = 0
let samples: { x: number; y: number; t: number }[] = []
let resetTimer: ReturnType<typeof setTimeout> | null = null
let momentumTimer: ReturnType<typeof setTimeout> | null = null
let rafId: number | null = null
let pendingDx = 0
let pendingDy = 0
let currentPos = { x: 0, y: 0 }

const bubbleText = ref("")
const bubbleAgentSource = ref<string | undefined>(undefined)

let unlistenStateChange: UnlistenFn | null = null
let unlistenBubble: UnlistenFn | null = null

async function loadPet(slug: string) {
  try {
    const data = await invoke<SpritesheetData>("read_pet_spritesheet", { slug })
    spritesheetUrl.value = data.data
  } catch (e) {
    console.error("[PetPage] Failed to load spritesheet:", e)
  }
}

async function moveWindow(dx: number, dy: number) {
  pendingDx += dx
  pendingDy += dy
  if (!rafId) {
    rafId = requestAnimationFrame(applyMove)
  }
}

async function applyMove() {
  rafId = null
  if (pendingDx !== 0 || pendingDy !== 0) {
    currentPos.x += pendingDx
    currentPos.y += pendingDy
    pendingDx = 0
    pendingDy = 0
    try {
      const win = getCurrentWindow()
      await win.setPosition(
        new PhysicalPosition(Math.round(currentPos.x), Math.round(currentPos.y))
      )
    } catch {
      // ignore
    }
  }
}

function pushSample(e: PointerEvent) {
  const t = performance.now()
  samples.push({ x: e.screenX, y: e.screenY, t })
  samples = samples.filter((s) => t - s.t <= SAMPLE_WINDOW_MS)
}

function computeVelocity(): { x: number; y: number } | null {
  if (samples.length < 2) return null
  const last = samples[samples.length - 1]
  const first = samples.find((s) => last.t - s.t > 16)
  if (!first) return null
  const dtSec = (last.t - first.t) / 1000
  if (dtSec <= 0) return null
  return {
    x: (last.x - first.x) / dtSec,
    y: (last.y - first.y) / dtSec,
  }
}

function cancelMomentum() {
  if (momentumTimer != null) {
    clearTimeout(momentumTimer)
    momentumTimer = null
  }
}

function throwWithVelocity(vx: number, vy: number) {
  if (
    !Number.isFinite(vx) ||
    !Number.isFinite(vy) ||
    (vx === 0 && vy === 0)
  )
    return
  cancelMomentum()
  let elapsed = 0
  const tick = () => {
    momentumTimer = null
    elapsed += TICK_MS
    moveWindow((vx * TICK_MS) / 1000, (vy * TICK_MS) / 1000)
    if (vx >= MIN_VEL) play("running-right")
    else if (vx <= -MIN_VEL) play("running-left")
    vx *= FRICTION
    vy *= FRICTION
    if (elapsed >= MAX_DURATION || Math.hypot(vx, vy) < MIN_VEL) {
      play("waving")
      if (resetTimer) clearTimeout(resetTimer)
      resetTimer = setTimeout(() => play("idle"), 1200)
      return
    }
    momentumTimer = setTimeout(tick, TICK_MS)
  }
  momentumTimer = setTimeout(tick, TICK_MS)
}

function onPointerDown(e: PointerEvent) {
  if (e.button !== 0) return
  showPicker.value = false
  dragging.value = true
  lastX = e.screenX
  lastY = e.screenY
  samples = []
  pushSample(e)
  if (petRef.value) {
    petRef.value.setPointerCapture(e.pointerId)
  }
  play("jumping")
  if (resetTimer) {
    clearTimeout(resetTimer)
    resetTimer = null
  }
  cancelMomentum()
  e.preventDefault()
}

function onPointerMove(e: PointerEvent) {
  if (!dragging.value) return
  const dx = e.screenX - lastX
  const dy = e.screenY - lastY
  lastX = e.screenX
  lastY = e.screenY
  pushSample(e)
  moveWindow(dx, dy)
  if (dx >= THRESHOLD) play("running-right")
  else if (dx <= -THRESHOLD) play("running-left")
}

function endDrag(e: PointerEvent) {
  if (!dragging.value) return
  dragging.value = false
  try {
    if (petRef.value) {
      petRef.value.releasePointerCapture(e.pointerId)
    }
  } catch {
    // ignore
  }
  const v = computeVelocity()
  if (v != null && Math.hypot(v.x, v.y) >= MIN_VEL) {
    throwWithVelocity(v.x, v.y)
  } else {
    play("waving")
    resetTimer = setTimeout(() => play("idle"), 1200)
  }
  // Save position
  invoke("save_pet_state", {
    state: {
      slug: activeSlug.value,
      x: currentPos.x,
      y: currentPos.y,
    },
  }).catch(() => {})
}

function onContextMenu() {
  showPicker.value = !showPicker.value
}

async function onSelectPet(slug: string) {
  showPicker.value = false
  activeSlug.value = slug
  await loadPet(slug)
  await invoke("save_pet_state", {
    state: {
      slug,
      x: currentPos.x,
      y: currentPos.y,
    },
  })
}

onMounted(async () => {
  // Restore position and slug
  try {
    const state = await invoke<{
      activeSlug: string | null
      x: number | null
      y: number | null
    }>("get_pet_state")
    activeSlug.value = state.activeSlug ?? null
    const win = getCurrentWindow()
    const pos = await win.outerPosition()
    currentPos = {
      x: state.x ?? pos.x,
      y: state.y ?? pos.y,
    }
    if (state.x != null || state.y != null) {
      await win.setPosition(
        new PhysicalPosition(Math.round(currentPos.x), Math.round(currentPos.y))
      )
    }

    if (activeSlug.value) {
      await loadPet(activeSlug.value)
    }
  } catch (e) {
    console.error("[PetPage] init error:", e)
  }

  // Listen for state changes from main window
  unlistenStateChange = await listen<{
    state: PetAnimationState
    duration?: number
  }>("pet:state-change", (event) => {
    if (dragging.value || momentumTimer != null) return
    if (resetTimer) {
      clearTimeout(resetTimer)
      resetTimer = null
    }
    play(event.payload.state)
  })

  // Listen for bubble updates
  unlistenBubble = await listen<{
    text: string
    agent_source?: string
  }>("pet:bubble", (event) => {
    bubbleText.value = event.payload.text
    bubbleAgentSource.value = event.payload.agent_source
    nextTick()
  })
})

onUnmounted(() => {
  if (resetTimer) {
    clearTimeout(resetTimer)
  }
  cancelMomentum()
  if (rafId) {
    cancelAnimationFrame(rafId)
  }
  if (unlistenStateChange) unlistenStateChange()
  if (unlistenBubble) unlistenBubble()
})
</script>

<template>
  <div class="stage" ref="stageRef">
    <PetSprite
      ref="petRef"
      :background-position="backgroundPosition"
      :spritesheet-url="spritesheetUrl"
      :dragging="dragging"
      @pointerdown="onPointerDown"
      @pointermove="onPointerMove"
      @pointerup="endDrag"
      @pointercancel="endDrag"
      @contextmenu.prevent="onContextMenu"
    />
    <PetBubble
      :text="bubbleText"
      :agent-source="bubbleAgentSource"
      :pet-element="petRef"
    />
    <PetPicker
      v-if="showPicker"
      :active-slug="activeSlug"
      @select="onSelectPet"
      @close="showPicker = false"
    />
  </div>
</template>

<style>
html,
body {
  margin: 0;
  padding: 0;
  background: transparent;
  overflow: hidden;
  width: 100%;
  height: 100%;
  font-family: -apple-system, system-ui, sans-serif;
}
body {
  -webkit-user-select: none;
  user-select: none;
}
</style>

<style scoped>
.stage {
  position: fixed;
  top: 34px;
  left: 8px;
}
</style>

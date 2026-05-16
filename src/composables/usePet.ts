import { ref, onUnmounted } from "vue"
import type { PetAnimationState, StateDef, FrameDef } from "@/types/pet"

const COLS = 8
const ROWS = 9

const STATES: Record<string, StateDef> = {
  idle: {
    row: 0,
    frames: [
      { c: 0, r: 0, d: 280 },
      { c: 1, r: 0, d: 110 },
      { c: 2, r: 0, d: 110 },
      { c: 3, r: 0, d: 140 },
      { c: 4, r: 0, d: 140 },
      { c: 5, r: 0, d: 320 },
    ],
    slow: 6,
  },
  "running-right": { row: 1, count: 8, dur: 120, last: 220 },
  "running-left": { row: 2, count: 8, dur: 120, last: 220 },
  waving: { row: 3, count: 4, dur: 140, last: 280 },
  jumping: { row: 4, count: 5, dur: 140, last: 280 },
  failed: { row: 5, count: 8, dur: 140, last: 240 },
  waiting: { row: 6, count: 6, dur: 150, last: 260 },
  running: { row: 7, count: 6, dur: 120, last: 220 },
  review: { row: 8, count: 6, dur: 150, last: 280 },
}

function buildFrames(s: StateDef): FrameDef[] {
  if (s.frames) {
    const slow = s.slow || 1
    return s.frames.map((f) => ({ c: f.c, r: s.row, d: f.d * slow }))
  }
  return Array.from({ length: s.count! }, (_, i) => ({
    c: i,
    r: s.row,
    d: i === s.count! - 1 ? s.last! : s.dur!,
  }))
}

function pos(c: number, r: number): string {
  return `${(c / (COLS - 1)) * 100}% ${(r / (ROWS - 1)) * 100}%`
}

export function usePet() {
  const currentState = ref<PetAnimationState>("idle")
  const backgroundPosition = ref("0% 0%")
  let stateTimer: ReturnType<typeof setTimeout> | null = null

  function play(state: PetAnimationState) {
    if (state === currentState.value) return
    currentState.value = state
    if (stateTimer) {
      clearTimeout(stateTimer)
      stateTimer = null
    }
    const def = STATES[state] || STATES.idle
    const frames = buildFrames(def)
    let i = 0
    backgroundPosition.value = pos(frames[0].c, frames[0].r)
    if (frames.length === 1) return
    const tick = () => {
      stateTimer = setTimeout(() => {
        i = (i + 1) % frames.length
        backgroundPosition.value = pos(frames[i].c, frames[i].r)
        tick()
      }, frames[i].d)
    }
    tick()
  }

  onUnmounted(() => {
    if (stateTimer) {
      clearTimeout(stateTimer)
      stateTimer = null
    }
  })

  return {
    currentState,
    backgroundPosition,
    play,
    STATES,
    COLS,
    ROWS,
  }
}

export type { PetAnimationState }

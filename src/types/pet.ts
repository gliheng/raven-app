export interface PetInfo {
  slug: string
  displayName: string
}

export interface PetState {
  activeSlug: string | null
  x: number | null
  y: number | null
}

export interface SpritesheetData {
  data: string
  slug: string
}

export type PetAnimationState =
  | "idle"
  | "running-right"
  | "running-left"
  | "waving"
  | "jumping"
  | "failed"
  | "waiting"
  | "running"
  | "review"

export interface FrameDef {
  c: number
  r: number
  d: number
}

export interface StateDef {
  row: number
  frames?: FrameDef[]
  count?: number
  dur?: number
  last?: number
  slow?: number
}

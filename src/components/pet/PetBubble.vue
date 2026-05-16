<script setup lang="ts">
import { ref, watch, nextTick } from "vue"

const AGENT_AVATARS: Record<string, string> = {
  "claude-code": "/pet/agents/claude-code.svg",
  "codex": "/pet/agents/codex.svg",
  "gemini": "/pet/agents/gemini.svg",
  "opencode": "/pet/agents/opencode.svg",
  "antigravity": "/pet/agents/antigravity.svg",
}

const props = defineProps<{
  text: string
  agentSource?: string
  petElement: HTMLElement | null
}>()

const visible = ref(false)
const position = ref({ left: 0, top: 0 })

function agentAvatarSrc(source: string | undefined): string {
  return source && AGENT_AVATARS[source]
    ? AGENT_AVATARS[source]
    : "/pet/agents/fallback.svg"
}

function positionBubble(el: HTMLElement) {
  if (!props.petElement || !el) return
  const rect = props.petElement.getBoundingClientRect()
  const bw = el.offsetWidth || 100
  const bh = el.offsetHeight || 22
  const ww = window.innerWidth
  const gap = 14
  const petCenterX = rect.left + rect.width / 2
  const desiredLeft = petCenterX - bw / 2
  const left = Math.max(4, Math.min(ww - bw - 4, desiredLeft))
  const top = rect.top - bh - gap
  position.value = { left, top: Math.max(2, top) }
}

watch(
  () => props.text,
  async () => {
    if (props.text) {
      visible.value = true
      await nextTick()
      const el = document.getElementById("pet-bubble")
      if (el) positionBubble(el)
    } else {
      visible.value = false
    }
  },
  { immediate: true }
)
</script>

<template>
  <div
    v-if="visible"
    id="pet-bubble"
    :style="{
      position: 'fixed',
      left: position.left + 'px',
      top: position.top + 'px',
      padding: '4px 8px',
      borderRadius: '10px',
      background: '#ffffff',
      color: '#111',
      fontWeight: 600,
      fontSize: '11px',
      fontFamily: 'system-ui, -apple-system, sans-serif',
      lineHeight: 1.2,
      boxShadow: '0 2px 6px rgba(0,0,0,0.30)',
      textAlign: 'left',
      whiteSpace: 'normal',
      maxWidth: '190px',
      display: 'flex',
      alignItems: 'center',
      gap: '6px',
      pointerEvents: 'none',
      zIndex: 5,
    }"
  >
    <img
      :src="agentAvatarSrc(agentSource)"
      :alt="(agentSource || 'Agent') + ' avatar'"
      style="width:20px;height:20px;flex:0 0 auto;object-fit:cover;display:block;"
    />
    <span style="display:block;min-width:0;word-break:keep-all;overflow-wrap:break-word;">
      {{ text }}
    </span>
  </div>
</template>

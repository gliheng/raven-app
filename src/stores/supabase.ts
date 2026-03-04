import { ref, readonly, watch } from 'vue'
import { defineStore } from 'pinia'
import { debounce } from 'lodash-es'
import { useAuthStore } from '@/stores/auth'
import * as supabaseDb from '@/db/supabase'
import type { ChatSettings, WebSearchSettings, ImageModelSettings } from '@/db/supabase'
import type { ChatModelConfig, AgentConfig, McpServer } from '@/stores/settings'

export type PulledSettings = Awaited<ReturnType<typeof supabaseDb.fetchAllSettings>>
export type OnPullCallback = (settings: PulledSettings) => Promise<void> | void

export const useSupabaseStore = defineStore('supabase', () => {
  const authStore = useAuthStore()

  const syncEnabled = ref(false)
  const isSyncing = ref(false)
  const lastSyncTime = ref<Date | null>(null)
  const syncError = ref<Error | null>(null)
  const isOnline = ref(navigator.onLine)

  let onPullCallback: OnPullCallback | null = null

  function onPull(cb: OnPullCallback) {
    onPullCallback = cb
  }

  // Update online status
  window.addEventListener('online', () => { isOnline.value = true })
  window.addEventListener('offline', () => { isOnline.value = false })

  // Enable sync when user is authenticated
  watch(() => authStore.isAuthenticated, (authenticated) => {
    syncEnabled.value = authenticated
    if (authenticated) {
      pullSettings()
    }
  }, { immediate: true })

  // Pull settings from Supabase (called on login)
  async function pullSettings() {
    if (!syncEnabled.value || isSyncing.value) return

    isSyncing.value = true
    syncError.value = null

    try {
      const settings = await supabaseDb.fetchAllSettings()
      if (settings && onPullCallback) {
        await onPullCallback(settings)
      }
      lastSyncTime.value = new Date()
      console.log('[Supabase] Settings pulled successfully')
      return settings
    } catch (err) {
      syncError.value = err as Error
      console.error('[Supabase] Failed to pull settings:', err)
    } finally {
      isSyncing.value = false
    }
  }

  // Push to Supabase (debounced wrapper)
  async function pushSettings(settings: {
    chat?: ChatSettings
    websearch?: WebSearchSettings
    model?: Record<string, ChatModelConfig>
    deletedModelKeys?: string[]
    agent?: Record<string, AgentConfig>
    deletedAgentKeys?: string[]
    mcp?: Record<string, McpServer>
    deletedMcpIds?: string[]
    imageModel?: Record<string, ImageModelSettings>
    deletedImageModelKeys?: string[]
  }) {
    if (!syncEnabled.value || isSyncing.value) return
    if (!isOnline.value) {
      console.log('[Supabase] Offline - settings will sync when connected')
      return
    }

    isSyncing.value = true
    syncError.value = null

    console.log('[Supabase] Pushing settings:', settings)
    try {
      const promises = []

      if (settings.model) {
        promises.push(supabaseDb.syncModelSettings(settings.model))
      }
      if (settings.deletedModelKeys) {
        promises.push(supabaseDb.deleteModelSettings(settings.deletedModelKeys))
      }
      if (settings.agent) {
        promises.push(supabaseDb.syncAgentSettings(settings.agent))
      }
      if (settings.deletedAgentKeys) {
        promises.push(supabaseDb.deleteAgentSettings(settings.deletedAgentKeys))
      }
      if (settings.chat) {
        promises.push(supabaseDb.syncChatSettings(settings.chat))
      }
      if (settings.websearch) {
        promises.push(supabaseDb.syncWebSearchSettings(settings.websearch))
      }
      if (settings.mcp) {
        promises.push(supabaseDb.syncMcpServers(settings.mcp))
      }
      if (settings.deletedMcpIds) {
        promises.push(supabaseDb.deleteMcpServers(settings.deletedMcpIds))
      }
      if (settings.imageModel) {
        promises.push(supabaseDb.syncImageModelSettings(settings.imageModel))
      }
      if (settings.deletedImageModelKeys) {
        promises.push(supabaseDb.deleteImageModelSettings(settings.deletedImageModelKeys))
      }

      await Promise.all(promises)
      lastSyncTime.value = new Date()
      console.log('[Supabase] Settings synced successfully')
    } catch (err) {
      syncError.value = err as Error
      console.error('[Supabase] Failed to sync settings:', err)
    } finally {
      isSyncing.value = false
    }
  }

  // Debounced version for reactive updates
  const debouncedPushSettings = debounce(pushSettings, 2000)

  return {
    syncEnabled: readonly(syncEnabled),
    isSyncing: readonly(isSyncing),
    isOnline: readonly(isOnline),
    lastSyncTime,
    syncError,
    onPull,
    pullSettings,
    pushSettings,
    debouncedPushSettings,
  }
})

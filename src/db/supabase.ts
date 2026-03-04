import { supabase, type UserSettingRow } from '@/lib/supabase'
import type { ChatModelConfig, AgentConfig, McpServer } from '@/stores/settings'

export interface ChatSettings {
  chatModel: string
  mcpServers: string[]
}

export interface WebSearchSettings {
  apiKey: string
}

export interface ImageModelSettings {
  apiKey: string
}

// ============== MODEL SETTINGS ==============

export async function syncModelSettings(settings: Record<string, ChatModelConfig>) {
  const { data: { user } } = await supabase.auth.getUser()
  if (!user) throw new Error('User not authenticated')

  const promises = Object.entries(settings).map(([key, config]) => {
    const id = `model::${key}`
    return (supabase
      .from('user_settings') as any)
      .upsert({
        id,
        user_id: user.id,
        type: 'model',
        key,
        value: JSON.stringify(config),
        updated_at: new Date().toISOString(),
      })
  })

  await Promise.all(promises)
}

export async function fetchModelSettings(): Promise<Record<string, ChatModelConfig>> {
  const { data: { user } } = await supabase.auth.getUser()
  if (!user) throw new Error('User not authenticated')

  const { data, error } = await supabase
    .from('user_settings')
    .select('*')
    .eq('user_id', user.id)
    .eq('type', 'model')
    .returns<UserSettingRow[]>()

  if (error) throw error

  const settings: Record<string, ChatModelConfig> = {}
  for (const row of data ?? []) {
    settings[row.key] = JSON.parse(row.value) as ChatModelConfig
  }
  return settings
}

export async function deleteModelSettings(settingKeys: string[]): Promise<void> {
  const { data: { user } } = await supabase.auth.getUser()
  if (!user) throw new Error('User not authenticated')

  if (settingKeys.length === 0) return

  const ids = settingKeys.map(key => `model::${key}`)

  const { error } = await supabase
    .from('user_settings')
    .delete()
    .in('id', ids)
    .eq('user_id', user.id)
    .eq('type', 'model')

  if (error) throw error
}

// ============== AGENT SETTINGS ==============

export async function syncAgentSettings(settings: Record<string, AgentConfig>) {
  const { data: { user } } = await supabase.auth.getUser()
  if (!user) throw new Error('User not authenticated')

  const promises = Object.entries(settings).map(([key, config]) => {
    const id = `agent::${key}`
    return (supabase
      .from('user_settings') as any)
      .upsert({
        id,
        user_id: user.id,
        type: 'agent',
        key,
        value: JSON.stringify(config),
        updated_at: new Date().toISOString(),
      })
  })

  await Promise.all(promises)
}

export async function fetchAgentSettings(): Promise<Record<string, AgentConfig>> {
  const { data: { user } } = await supabase.auth.getUser()
  if (!user) throw new Error('User not authenticated')

  const { data, error } = await supabase
    .from('user_settings')
    .select('*')
    .eq('user_id', user.id)
    .eq('type', 'agent')
    .returns<UserSettingRow[]>()

  if (error) throw error

  const settings: Record<string, AgentConfig> = {}
  for (const row of data ?? []) {
    settings[row.key] = JSON.parse(row.value) as AgentConfig
  }
  return settings
}

export async function deleteAgentSettings(settingKeys: string[]): Promise<void> {
  const { data: { user } } = await supabase.auth.getUser()
  if (!user) throw new Error('User not authenticated')

  if (settingKeys.length === 0) return

  const ids = settingKeys.map(key => `agent::${key}`)

  const { error } = await supabase
    .from('user_settings')
    .delete()
    .in('id', ids)
    .eq('user_id', user.id)
    .eq('type', 'agent')

  if (error) throw error
}

// ============== CHAT SETTINGS ==============

export async function syncChatSettings(settings: ChatSettings) {
  const { data: { user } } = await supabase.auth.getUser()
  if (!user) throw new Error('User not authenticated')

  await (supabase
    .from('user_settings') as any)
    .upsert({
      id: 'chat::default',
      user_id: user.id,
      type: 'chat',
      key: 'default',
      value: JSON.stringify(settings),
      updated_at: new Date().toISOString(),
    })
}

export async function fetchChatSettings(): Promise<ChatSettings | null> {
  const { data: { user } } = await supabase.auth.getUser()
  if (!user) throw new Error('User not authenticated')

  const { data, error } = await supabase
    .from('user_settings')
    .select('value')
    .eq('user_id', user.id)
    .eq('type', 'chat')
    .eq('key', 'default')
    .single()

  if (error) {
    if (error.code === 'PGRST116') return null // Not found
    throw error
  }

  return JSON.parse((data as any).value) as ChatSettings
}

// ============== WEB SEARCH SETTINGS ==============

export async function syncWebSearchSettings(settings: WebSearchSettings) {
  const { data: { user } } = await supabase.auth.getUser()
  if (!user) throw new Error('User not authenticated')

  await (supabase
    .from('user_settings') as any)
    .upsert({
      id: 'websearch::default',
      user_id: user.id,
      type: 'websearch',
      key: 'default',
      value: JSON.stringify(settings),
      updated_at: new Date().toISOString(),
    })
}

export async function fetchWebSearchSettings(): Promise<WebSearchSettings | null> {
  const { data: { user } } = await supabase.auth.getUser()
  if (!user) throw new Error('User not authenticated')

  const { data, error } = await supabase
    .from('user_settings')
    .select('value')
    .eq('user_id', user.id)
    .eq('type', 'websearch')
    .eq('key', 'default')
    .single()

  if (error) {
    if (error.code === 'PGRST116') return null
    throw error
  }

  return JSON.parse((data as any).value) as WebSearchSettings
}

// ============== MCP SERVER SETTINGS ==============

export async function syncMcpServers(servers: Record<string, McpServer>) {
  const { data: { user } } = await supabase.auth.getUser()
  if (!user) throw new Error('User not authenticated')

  const promises = Object.values(servers).map((server) => {
    const id = `mcp::${server.id}`
    return (supabase
      .from('user_settings') as any)
      .upsert({
        id,
        user_id: user.id,
        type: 'mcp',
        key: server.id,
        value: JSON.stringify(server),
        updated_at: new Date().toISOString(),
      })
  })

  await Promise.all(promises)
}

export async function fetchMcpServers(): Promise<Record<string, McpServer>> {
  const { data: { user } } = await supabase.auth.getUser()
  if (!user) throw new Error('User not authenticated')

  const { data, error } = await supabase
    .from('user_settings')
    .select('*')
    .eq('user_id', user.id)
    .eq('type', 'mcp')
    .returns<UserSettingRow[]>()

  if (error) throw error

  const servers: Record<string, McpServer> = {}
  for (const row of data ?? []) {
    servers[row.key] = JSON.parse(row.value) as McpServer
  }
  return servers
}

export async function deleteMcpServers(serverIds: string[]): Promise<void> {
  const { data: { user } } = await supabase.auth.getUser()
  if (!user) throw new Error('User not authenticated')

  if (serverIds.length === 0) return

  const ids = serverIds.map(id => `mcp::${id}`)

  const { error } = await supabase
    .from('user_settings')
    .delete()
    .in('id', ids)
    .eq('user_id', user.id)
    .eq('type', 'mcp')

  if (error) throw error
}

// ============== IMAGE MODEL SETTINGS ==============

export async function syncImageModelSettings(settings: Record<string, { apiKey: string }>) {
  const { data: { user } } = await supabase.auth.getUser()
  if (!user) throw new Error('User not authenticated')

  const promises = Object.entries(settings).map(([key, config]) => {
    const id = `imageModel::${key}`
    return (supabase
      .from('user_settings') as any)
      .upsert({
        id,
        user_id: user.id,
        type: 'imageModel',
        key,
        value: JSON.stringify(config),
        updated_at: new Date().toISOString(),
      })
  })

  await Promise.all(promises)
}

export async function fetchImageModelSettings(): Promise<Record<string, { apiKey: string }>> {
  const { data: { user } } = await supabase.auth.getUser()
  if (!user) throw new Error('User not authenticated')

  const { data, error } = await supabase
    .from('user_settings')
    .select('*')
    .eq('user_id', user.id)
    .eq('type', 'imageModel')
    .returns<UserSettingRow[]>()

  if (error) throw error

  const settings: Record<string, { apiKey: string }> = {}
  for (const row of data ?? []) {
    settings[row.key] = JSON.parse(row.value) as { apiKey: string }
  }
  return settings
}

export async function deleteImageModelSettings(settingKeys: string[]): Promise<void> {
  const { data: { user } } = await supabase.auth.getUser()
  if (!user) throw new Error('User not authenticated')

  if (settingKeys.length === 0) return

  const ids = settingKeys.map(key => `imageModel::${key}`)

  const { error } = await supabase
    .from('user_settings')
    .delete()
    .in('id', ids)
    .eq('user_id', user.id)
    .eq('type', 'imageModel')

  if (error) throw error
}

// ============== BATCH OPERATIONS ==============

export async function fetchAllSettings() {
  const { data: { user } } = await supabase.auth.getUser()
  if (!user) throw new Error('User not authenticated')

  const { data, error } = await supabase
    .from('user_settings')
    .select('*')
    .eq('user_id', user.id)
    .returns<UserSettingRow[]>()

  if (error) throw error

  const result: Record<string, any> = {
    model: {},
    agent: {},
    chat: null,
    websearch: null,
    mcp: {},
    imageModel: {},
  }

  for (const row of data ?? []) {
    const parsed = JSON.parse(row.value)
    if (row.type === 'chat' || row.type === 'websearch') {
      result[row.type] = parsed
    } else {
      result[row.type][row.key] = parsed
    }
  }

  return result
}

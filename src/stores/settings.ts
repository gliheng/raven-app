import { ref, watch } from "vue";
import { defineStore } from "pinia";
import { defaultsDeep, debounce } from 'lodash-es';
import * as settingsDb from "@/db/settings";
import type { McpServer } from "@/types/mcp";
import { useMcpStore } from "@/stores/mcp";
import { useSupabaseStore } from '@/stores/supabase';

// Re-export McpServer for use in other modules
export type { McpServer };

export interface ChatModelConfig {
  apiKey: string;
  models: string[];
}

const defaultModelSettings: Record<string, ChatModelConfig> = {
  deepseek: {
    apiKey: "",
    models: [],
  },
  minimax: {
    apiKey: "",
    models: [],
  },
  zai: {
    apiKey: "",
    models: [],
  },
  openrouter: {
    apiKey: "",
    models: [],
  },
  siliconflow: {
    apiKey: "",
    models: [],
  },
};

const defaultImageModelSettings: Record<string, { apiKey: string }> = {
  openai: {
    apiKey: "",
  },
  stability: {
    apiKey: "",
  },
  siliconflow: {
    apiKey: "",
  },
};

export interface AgentConfig {
  useCustomModel?: boolean;
  baseUrl?: string;
  model?: string;
  apiKey?: string;
}

const defaultAgentSettings: Record<string, AgentConfig> = {
  codex: {
    useCustomModel: false,
    baseUrl: "",
    model: "",
    apiKey: "",
  },
  gemini: {
    useCustomModel: false,
    baseUrl: "",
    model: "",
    apiKey: "",
  },
  claude: {
    useCustomModel: false,
    baseUrl: "",
    model: "",
    apiKey: "",
  },
  qwen: {
    useCustomModel: false,
    baseUrl: "",
    model: "",
    apiKey: "",
  },
  opencode: {
    useCustomModel: false,
    model: "",
  },
};

const defaultChatSettings = {
  chatModel: 'deepseek::deepseek-chat',
  mcpServers: [] as string[],
};

const defaultWebSearchSettings = {
  apiKey: '',
};

const defaultMcpSettings: Record<string, McpServer> = {};

export async function loadAgentSettings() {
  const dbSettings = await settingsDb.getAllAgentSettings();
  return defaultsDeep({}, dbSettings, defaultAgentSettings) as Record<string, AgentConfig>;
}

export async function loadModelSettings() {
  const dbSettings = await settingsDb.getAllModelSettings();
  return defaultsDeep({}, dbSettings, defaultModelSettings) as Record<string, ChatModelConfig>;
}

export async function loadChatSettings() {
  const dbSettings = await settingsDb.getChatSettings();
  return defaultsDeep({}, dbSettings, defaultChatSettings) as typeof defaultChatSettings;
}

export async function loadWebSearchSettings() {
  const dbSettings = await settingsDb.getWebSearchSettings();
  return defaultsDeep({}, dbSettings, defaultWebSearchSettings) as typeof defaultWebSearchSettings;
}

export async function loadMcpSettings() {
  const dbSettings = await settingsDb.getAllMcpServers();
  return defaultsDeep({}, dbSettings, defaultMcpSettings) as Record<string, McpServer>;
}

export async function loadImageModelSettings() {
  const dbSettings = await settingsDb.getAllImageModelSettings();
  const settings: Record<string, { apiKey: string }> = {};
  for (const [key, value] of Object.entries(dbSettings)) {
    settings[key] = { apiKey: value.apiKey };
  }
  return defaultsDeep({}, settings, defaultImageModelSettings) as Record<string, { apiKey: string }>;
}

export const useSettingsStore = defineStore("settings", () => {
  const modelSettings = ref<Record<string, ChatModelConfig>>({});
  const agentSettings = ref<Record<string, AgentConfig>>({});
  const imageSettings = ref<Record<string, { apiKey: string }>>({});
  const chatSettings = ref<typeof defaultChatSettings>({} as typeof defaultChatSettings);
  const webSearchSettings = ref<typeof defaultWebSearchSettings>({} as typeof defaultWebSearchSettings);
  const mcpServers = ref<Record<string, McpServer>>({});
  const isRestartingMcp = ref(false);
  const isMerging = ref(false);

  // Supabase sync integration
  const syncStore = useSupabaseStore();

  // Merge remote settings pulled from Supabase into local state
  syncStore.onPull(async (remote) => {
    isMerging.value = true
    try {
      if (remote.model) {
        modelSettings.value = defaultsDeep({}, remote.model, modelSettings.value)
        await settingsDb.writeAllModelSettings(modelSettings.value)
      }
      if (remote.agent) {
        agentSettings.value = defaultsDeep({}, remote.agent, agentSettings.value)
        await settingsDb.writeAllAgentSettings(agentSettings.value)
      }
      if (remote.chat) {
        chatSettings.value = defaultsDeep({}, remote.chat, chatSettings.value)
        await settingsDb.writeChatSettings(chatSettings.value)
      }
      if (remote.websearch) {
        webSearchSettings.value = defaultsDeep({}, remote.websearch, webSearchSettings.value)
        await settingsDb.writeWebSearchSettings(webSearchSettings.value)
      }
      if (remote.mcp) {
        mcpServers.value = defaultsDeep({}, remote.mcp, mcpServers.value)
        for (const [, server] of Object.entries(mcpServers.value)) {
          await settingsDb.writeMcpServer(server)
        }
      }
      if (remote.imageModel) {
        const merged = defaultsDeep({}, remote.imageModel, imageSettings.value) as Record<string, { apiKey: string }>;
        imageSettings.value = merged;
        await settingsDb.writeAllImageModelSettings(merged);
      }
      console.log('[Settings] Merged remote settings from Supabase')
    } finally {
      isMerging.value = false
    }
  })

  // Start MCP servers from settings
  async function initializeMcpServers() {
    const enabledServers = Object.entries(mcpServers.value)
      .filter(([, server]) => server.enabled)
      .map(([id, server]) => [id, server.config] as const);

    if (enabledServers.length > 0) {
      try {
        const mcpStore = useMcpStore();
        await mcpStore.startServers(Object.fromEntries(enabledServers));
        console.log(`[MCP] Started ${enabledServers.length} servers`);
      } catch (error) {
        console.error("[MCP] Failed to start servers:", error);
      }
    }
  }

  async function initialize() {
    modelSettings.value = await loadModelSettings();
    agentSettings.value = await loadAgentSettings();
    chatSettings.value = await loadChatSettings();
    webSearchSettings.value = await loadWebSearchSettings();
    mcpServers.value = await loadMcpSettings();
    imageSettings.value = await loadImageModelSettings();

    // Start MCP servers after settings are loaded
    await initializeMcpServers();
  }

  watch(modelSettings, async (v, oldV) => {
    if (isMerging.value) return;

    // Detect deleted settings
    const oldKeys = new Set(Object.keys(oldV ?? {}));
    const deletedKeys = [...oldKeys].filter(key => !(key in v));

    // Persist to database (write current settings)
    await settingsDb.writeAllModelSettings(v);

    // Sync to Supabase (including deletions)
    if (syncStore.syncEnabled) {
      syncStore.debouncedPushSettings({ model: v, deletedModelKeys: deletedKeys });
    }
  }, {
    deep: true,
  });

  watch(agentSettings, async (v, oldV) => {
    if (isMerging.value) return;

    // Detect deleted settings
    const oldKeys = new Set(Object.keys(oldV ?? {}));
    const deletedKeys = [...oldKeys].filter(key => !(key in v));

    // Persist to database (write current settings)
    await settingsDb.writeAllAgentSettings(v);

    // Sync to Supabase (including deletions)
    if (syncStore.syncEnabled) {
      syncStore.debouncedPushSettings({ agent: v, deletedAgentKeys: deletedKeys });
    }
  }, {
    deep: true,
  });

  watch(chatSettings, async (v) => {
    if (isMerging.value) return;
    await settingsDb.writeChatSettings(v);
    if (syncStore.syncEnabled) {
      syncStore.debouncedPushSettings({ chat: v });
    }
  }, {
    deep: true,
  });

  watch(webSearchSettings, async (v) => {
    if (isMerging.value) return;
    await settingsDb.writeWebSearchSettings(v);
    if (syncStore.syncEnabled) {
      syncStore.debouncedPushSettings({ websearch: v });
    }
  }, {
    deep: true,
  });

  watch(imageSettings, async (v, oldV) => {
    if (isMerging.value) return;

    // Detect deleted settings
    const oldKeys = new Set(Object.keys(oldV ?? {}));
    const deletedKeys = [...oldKeys].filter(key => !(key in v));

    // Persist to database (write current settings)
    await settingsDb.writeAllImageModelSettings(v);

    // Sync to Supabase (including deletions)
    if (syncStore.syncEnabled) {
      syncStore.debouncedPushSettings({ imageModel: v, deletedImageModelKeys: deletedKeys });
    }
  }, {
    deep: true,
  });

  // Debounced MCP server restart function
  const restartMcpServers = debounce(async () => {
    // Skip if already restarting
    if (isRestartingMcp.value) {
      return;
    }

    isRestartingMcp.value = true;
    try {
      const mcpStore = useMcpStore();
      await mcpStore.stopAllServers();
      await initializeMcpServers();
    } catch (error) {
      console.error("[MCP] Failed to restart servers:", error);
    } finally {
      isRestartingMcp.value = false;
    }
  }, 500);

  watch(mcpServers, async (v, oldV) => {
    if (isMerging.value) return;

    // Detect deleted servers
    const oldIds = new Set(Object.keys(oldV ?? {}));
    const deletedIds = [...oldIds].filter(id => !(id in v));

    // Persist to database (including deletions)
    for (const deletedId of deletedIds) {
      await settingsDb.deleteMcpServer(deletedId);
    }
    for (const [, server] of Object.entries(v)) {
      await settingsDb.writeMcpServer(server);
    }

    // Sync to Supabase (including deletions)
    if (syncStore.syncEnabled) {
      syncStore.debouncedPushSettings({ mcp: v, deletedMcpIds: deletedIds });
    }

    // Restart MCP servers with debouncing
    restartMcpServers();
  }, {
    deep: true,
  });

  return {
    modelSettings,
    agentSettings,
    chatSettings,
    webSearchSettings,
    mcpServers,
    imageSettings,
    initialize,
  };
});

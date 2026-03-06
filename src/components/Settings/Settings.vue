<script setup lang="ts">
import type { TabsItem } from "@nuxt/ui";
import ModelsTab from "./ModelsTab.vue";
import TavilyTab from "./TavilyTab.vue";
import McpTab from "./McpTab.vue";
import ProfileTab from "./ProfileTab.vue";

const tabItems = [
  {
    label: "Profile",
    icon: "i-lucide-circle-user",
    slot: "profile" as const,
  },
  {
    label: "Models",
    icon: "i-mdi-form-select",
    slot: "models" as const,
  },
  {
    label: "MCP",
    icon: "i-mdi-server",
    description: "",
    slot: "mcp" as const,
  },
  {
    label: "Web Search",
    icon: "i-lucide-search",
    slot: "tavily" as const,
  },
] satisfies TabsItem[];
</script>

<template>
  <UTabs :items="tabItems" orientation="vertical" variant="link" :ui="{
    root: 'size-full flex-1 items-stretch gap-0',
    list: 'self-stretch bg-elevated min-w-40 p-0',
    indicator: 'hidden',
    content: 'pl-4 flex flex-col',
  }">
    <template #list-leading>
      <h1 class="text-xl font-semibold pl-3 py-4">Settings</h1>
    </template>
    <template #profile>
      <h1 class="text-xl font-semibold py-4">Profile</h1>
      <div class="flex-1 min-h-0 overflow-y-auto">
        <div class="pr-4 py-4">
          <ProfileTab />
        </div>
      </div>
    </template>
    <template #models>
      <h1 class="text-xl font-semibold py-4">Models</h1>
      <div class="flex-1 min-h-0 overflow-y-auto">
        <div class="pr-4 py-4">
          <ModelsTab />
        </div>
      </div>
    </template>
    <template #tavily>
      <h1 class="text-xl font-semibold py-4">Tavily Search</h1>
      <div class="flex-1 min-h-0 overflow-y-auto">
        <div class="pr-4 py-4">
          <TavilyTab />
        </div>
      </div>
    </template>
    <template #mcp>
      <h1 class="text-xl font-semibold py-4">MCP Servers</h1>
      <div class="flex-1 min-h-0 overflow-y-auto">
        <div class="pr-4 py-4">
          <McpTab />
        </div>
      </div>
    </template>
  </UTabs>
</template>

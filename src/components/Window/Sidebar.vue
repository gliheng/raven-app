<script setup lang="ts">
import { useRouter } from "vue-router";
import { nanoid } from "nanoid";
import { tv } from "tailwind-variants";
import { useColorMode } from "@vueuse/core";
import { invoke } from "@tauri-apps/api/core";
import { useSidebarStore } from "@/stores/sidebar";
import { Agent, writeAgent } from "@/db";
import AgentModal from "../AgentChat/AgentModal.vue";
import type { AgentFormData } from "../AgentChat/AgentModal.vue";


const buttonStyle = tv({
  base: "items-center w-full",
});

const router = useRouter();
const sidebarStore = useSidebarStore();
const mode = useColorMode();

const getIconSrc = (mode: string, collapsed?: boolean) => {
  if (collapsed) {
    return mode === "dark" ? "/raven-icon-light.png" : "/raven-icon-dark.png";
  }
  return mode === "dark" ? "/raven-light.png" : "/raven-dark.png";
};

sidebarStore.load();

function addChat() {
  const id = nanoid();
  router.push({
    name: "chat",
    params: { id },
  });
}

const overlay = useOverlay();
const agentCreateModal = overlay.create(AgentModal);

async function addAgent() {
  const agent = await agentCreateModal.open() as AgentFormData | null;
  if (!agent) {
    return;
  }

  const id = nanoid();
  
  let directory = agent.directory;
  
  if (!directory) {
    try {
      directory = await invoke<string>('create_agent_directory', { agentId: id });
    } catch (error) {
      console.error('Failed to create agent directory:', error);
      return;
    }
  }
  
  const fullAgent: Agent = {
    ...agent,
    directory,
    id,
    createdAt: new Date(),
    updatedAt: new Date(),
  };
  
  await writeAgent(fullAgent);
  sidebarStore.loadAgents();
  router.push({
    name: "agent",
    params: {
      id,
    },
  });
}

function addImage() {
  router.push({
    name: "image",
  });
}

function agentUrl(agent: Agent) {
  return `/agent/${agent.id}`;
}

function openAgent(agent: Agent) {
  router.push({
    name: "agent",
    params: {
      id: agent.id,
    },
  });
}

function goToDocuments() {
  router.push({ name: 'documents' });
}

function goToJournal() {
  router.push({ name: 'journal' });
}

const platform = navigator.platform;
const isMac = platform.startsWith("Mac");</script>

<template>
  <UDashboardSidebar
    resizable
    collapsible
    :default-size="20"
    :min-size="10"
    :max-size="30"
    :ui="{
      root: 'bg-elevated',
      body: 'gap-2',
      footer: 'flex-col items-stretch',
    }"
  >
    <template #resize-handle="{ onMouseDown, onTouchStart, onDoubleClick }">
      <UDashboardResizeHandle
        class="after:absolute after:inset-y-0 after:right-0 after:w-px hover:after:bg-(--ui-border-accented) after:transition"
        @mousedown="onMouseDown"
        @touchstart="onTouchStart"
        @dblclick="onDoubleClick"
      />
    </template>
    <template #header="{ collapsed }">
      <h1
        class="text-3xl font-bold flex flex-row gap-4 items-center justify-center select-none flex-1"
        :class="{ 'pt-8': isMac }"
        data-tauri-drag-region
      >
        <img
          class="w-40 h-10 pointer-events-none object-contain"
          :src="getIconSrc(mode, collapsed)"
          alt="Logo"
        />
      </h1>
    </template>

    <template #default="{ collapsed }">
      <UTooltip text="New Chat" :content="{
        side: 'right',
      }">
        <UButton
          :label="collapsed ? undefined : 'New Chat'"
          icon="i-lucide-message-circle"
          variant="subtle"
          color="neutral"
          @click="addChat"
        />
      </UTooltip>

      <UCollapsible class="flex flex-col gap-2" default-open>
        <UTooltip text="Agent" :content="{
          side: 'right',
        }">
          <UButton
            :label="collapsed ? undefined : 'Agent'"
            :icon="collapsed ? undefined : 'i-lucide-brain'"
            class="group"
            variant="subtle"
            color="neutral"
            trailing-icon="i-lucide-chevron-down"
            block
            :ui="{
              trailingIcon:
                'group-data-[state=open]:rotate-180 transition-transform duration-200',
            }"
          />
        </UTooltip>
        <template #content>
          <section class="space-y-1">
            <UTooltip
              v-for="agent in sidebarStore.agents"
              :key="agent.id"
              :text="agent.name"
              :content="{
                side: 'right',
              }"
            >
              <UButton
                :icon="agent.icon"
                :label="collapsed ? undefined : agent.name"
                :class="buttonStyle({})"
                color="neutral"
                variant="soft"
                active-color="primary"
                active-variant="solid"
                :active="$route.path === agentUrl(agent)"
                @click="openAgent(agent)"
              />
            </UTooltip>
            <UTooltip text="Add Agent" :content="{
              side: 'right',
            }">
              <UButton
                class="w-full"
                :label="collapsed ? undefined : 'Add'"
                icon="i-lucide-plus"
                color="neutral"
                variant="soft"
                @click="addAgent"
              />
            </UTooltip>
          </section>
        </template>
      </UCollapsible>

      <UTooltip text="Create Document" :content="{
        side: 'right',
      }">
        <UButton
          :label="collapsed ? undefined : 'Document'"
          icon="i-lucide-file-text"
          variant="subtle"
          color="neutral"
          @click="goToDocuments"
        />
      </UTooltip>

      <UTooltip text="Generate Image with AI" :content="{
        side: 'right',
      }">
        <UButton
          :label="collapsed ? undefined : 'Image'"
          icon="i-lucide-image"
          variant="subtle"
          color="neutral"
          @click="addImage"
        />
      </UTooltip>
    </template>

    <template #footer="{ collapsed }">
      <UTooltip text="Journal" :content="{
        side: 'right',
      }">
        <UButton
          :label="collapsed ? undefined : 'Journal'"
          icon="i-lucide-book-open"
          variant="subtle"
          color="neutral"
          block
          @click="goToJournal"
        />
      </UTooltip>
      <UTooltip text="Settings" :content="{
        side: 'right',
      }">
        <UButton
          icon="i-mdi-cog"
          color="neutral"
          :label="collapsed ? undefined : 'Settings'"
          variant="subtle"
          block
          @click="router.push({ name: 'settings' })"
        />
      </UTooltip>
    </template>
  </UDashboardSidebar>
</template>

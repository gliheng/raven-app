<script setup lang="ts">
import { ref, toRaw, computed, type Component } from "vue";
import IconEdit from "@/components/IconEdit.vue";
import type { AgentProgram } from "@/db";
import { AGENT_PROGRAMS } from "@/constants";
import ICustomOpencode from '~icons/custom/opencode';
import ICustomQwen from '~icons/custom/qwen';
import ICustomCodex from '~icons/custom/codex';
import ICustomClaude from '~icons/custom/claude';
import ICustomGemini from '~icons/custom/gemini';

const iconMap: Record<string, string | Component> = {
  'codex': ICustomCodex,
  'gemini': ICustomGemini,
  'claude': ICustomClaude,
  'qwen': ICustomQwen,
  'opencode': ICustomOpencode,
};
const defaultIcon = 'i-lucide-bot';

interface AgentProgramSelectItem {
  value: string;
  label: string;
  icon: string | Component;
}

const programs = computed(() =>
  AGENT_PROGRAMS.map(p => ({
    ...p,
    icon: iconMap[p.value] || defaultIcon,
  }) satisfies AgentProgramSelectItem)
);

export interface AgentFormData {
  name: string;
  icon: string;
  program: AgentProgram;
  directory: string;
}

const emit = defineEmits<{
  close: [agent: AgentFormData];
}>();

const agent = ref<AgentFormData>({
  name: 'New Agent',
  icon: 'i-lucide-brain',
  program: 'opencode' as const,
  directory: '',
});

const selectedProgram = computed(() =>
  programs.value.find(program => program.value === agent.value.program)
);

// Directory selection
const selectDirectory = async () => {
  try {
    // Use Tauri's dialog API to select directory
    const { open } = await import('@tauri-apps/plugin-dialog');
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Select working directory'
    });

    if (selected) {
      agent.value.directory = selected as string;
    }
  } catch (error) {
    console.error('Failed to select directory:', error);
  }
};

const nameError = ref('');
const directoryError = ref('');

const validate = (): boolean => {
  nameError.value = '';
  directoryError.value = '';

  if (!agent.value.name.trim()) {
    nameError.value = 'Name is required';
  }
  if (!agent.value.directory.trim()) {
    directoryError.value = 'Working directory is required';
  }

  return !nameError.value && !directoryError.value;
};

const createAgent = () => {
  if (!validate()) return;
  emit('close', toRaw(agent.value) as AgentFormData);
};
</script>

<template>
  <UModal title="Create Agent" description="Configure your AI agent with a name, program, and working directory">
    <template #content>
      <div class="size-full p-6 space-y-4">
        <hgroup class="flex flex-row gap-2 items-center">
          <IconEdit v-model:icon="agent.icon" />
          <UInput class="flex-1" v-model="agent.name" />
        </hgroup>
        <p v-if="nameError" class="text-red-500 text-sm">{{ nameError }}</p>
        <section>
          <h2 class="text-lg mb-2">Agent Program</h2>
          <USelect
            v-model="agent.program"
            class="w-full"
            color="neutral"
            variant="subtle"
            size="xl"
            trailing-icon="i-lucide-chevrons-up-down"
            value-key="value"
            :items="programs"
            placeholder="Select an agent program"
          >
            <template #leading="{ ui }">
              <component
                :is="selectedProgram?.icon"
                v-if="selectedProgram?.icon && typeof selectedProgram.icon !== 'string'"
                :class="ui.leadingIcon()"
              />
              <UIcon
                v-else-if="typeof selectedProgram?.icon === 'string'"
                :name="selectedProgram.icon"
                :class="ui.leadingIcon()"
              />
            </template>
            <template #item-leading="{ item, ui }">
              <component
                :is="item.icon"
                v-if="item.icon && typeof item.icon !== 'string'"
                :class="ui.itemLeadingIcon()"
              />
              <UIcon
                v-else-if="typeof item.icon === 'string'"
                :name="item.icon"
                :class="ui.itemLeadingIcon()"
              />
            </template>
          </USelect>
        </section>
        <section>
          <h2 class="text-lg mb-2">Working Directory</h2>
          <div class="flex gap-2">
            <UInput
              class="flex-1"
              placeholder="Select a directory..."
              :model-value="agent.directory"
              readonly
            />
            <UButton
              icon="i-lucide-folder"
              label="Browse"
              @click="selectDirectory"
            />
          </div>
          <p v-if="directoryError" class="text-red-500 text-sm mt-1">{{ directoryError }}</p>
        </section>
        <section class="flex justify-center mt-10">
          <UButton
            class="text-xl px-8 py-4"
            color="primary"
            size="xl"
            icon="i-lucide-wand-sparkles"
            @click="createAgent"
          >Create</UButton>
        </section>
      </div>
    </template>
  </UModal>
</template>

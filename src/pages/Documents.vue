<script setup lang="ts">
import { ref, computed, onActivated } from "vue";
import { useRouter } from "vue-router";
import { useDocumentsStore } from "@/stores/documents";
import { useTabsStore } from "@/stores/tabs";
import { nanoid } from "nanoid";
import DocumentConfig from "@/components/Document/DocumentConfig.vue";
import { confirm } from '@tauri-apps/plugin-dialog';
import type { TabsItem } from "@nuxt/ui";

const documentsStore = useDocumentsStore();
const tabsStore = useTabsStore();
const router = useRouter();

const filter = ref<'all' | 'note' | 'chart'>('all');

const overlay = useOverlay();
const documentCreateModal = overlay.create(DocumentConfig);

onActivated(() => {
  tabsStore.openTab('/documents', 'Documents');
  documentsStore.loadDocuments();
});

const tabItems = [
  {
    label: "All",
    icon: "i-lucide-file-text",
    value: "all",
  },
  {
    label: "Notes",
    icon: "i-lucide-notebook-text",
    value: "note",
  },
  {
    label: "Charts",
    icon: "i-lucide-git-compare",
    value: "chart",
  },
] satisfies TabsItem[];

const filteredDocuments = computed(() => {
  if (filter.value === 'all') {
    return documentsStore.documents;
  } else if (filter.value === 'note') {
    return documentsStore.notes;
  } else {
    return documentsStore.charts;
  }
});

async function addDocument() {
  const type = await documentCreateModal.open() as "note" | "chart" | null;
  if (!type) {
    return;
  }

  const id = nanoid();
  const name = type === 'note' ? 'New Note' : 'New Chart';
  tabsStore.openTab(`/${type}/${id}`, name);
  router.push({
    name: type,
    params: {
      id,
    },
  });
}

function openDocument(document: { id: string; name: string; type: 'note' | 'chart' }) {
  tabsStore.openTab(`/${document.type}/${document.id}`, document.name);
  router.push({
    name: document.type,
    params: {
      id: document.id,
    },
  });
}

function formatDate(date: Date) {
  return new Date(date).toLocaleDateString('en-US', {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  });
}

async function deleteDocument(id: string, event: Event) {
  event.stopPropagation();

  const confirmed = await confirm(
    'This action cannot be reverted. Are you sure you want to delete this document?',
    { title: 'Delete Document', kind: 'warning' }
  );

  if (!confirmed) {
    return;
  }

  await documentsStore.deleteDocument(id);
}
</script>

<template>
  <div class="size-full flex flex-col">
    <div class="flex items-center justify-between border-b border-white/10 px-6 py-4">
      <h1 class="text-lg font-semibold">Documents</h1>
      <UButton
        icon="i-lucide-plus"
        label="Create New"
        variant="subtle"
        @click="addDocument"
      />
    </div>

    <div class="flex-1 flex flex-col overflow-hidden">
      <div class="px-6 pt-4">
        <UTabs v-model="filter" :items="tabItems" variant="link" />
      </div>

      <div class="flex-1 overflow-auto p-6">
        <UEmpty
          v-if="filteredDocuments.length === 0"
          icon="i-lucide-file-text"
          title="No documents found"
          description="Create your first document to get started!"
          class="w-fit mx-auto"
        >
          <template #actions>
            <UButton
              icon="i-lucide-plus"
              label="Create New"
              @click="addDocument"
            />
          </template>
        </UEmpty>
        <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <UCard
            v-for="document in filteredDocuments"
            :key="document.id"
            class="cursor-pointer transition-all hover:scale-[1.02] group relative"
            @click="openDocument(document)"
          >
            <div class="flex items-start gap-3">
              <div class="flex-shrink-0">
                <UIcon
                  :name="document.icon"
                  class="text-2xl"
                />
              </div>
              <div class="flex-1 min-w-0">
                <div class="flex items-start justify-between gap-2">
                  <h3 class="font-semibold text-base truncate">{{ document.name }}</h3>
                  <UBadge
                    :color="document.type === 'note' ? 'primary' : 'warning'"
                    :icon="document.type === 'note' ? 'i-lucide-notebook-text' : 'i-lucide-git-compare'"
                    variant="soft"
                    size="sm"
                  >
                    {{ document.type }}
                  </UBadge>
                </div>
                <div class="flex flex-col gap-1 mt-2 text-sm">
                  <span class="flex items-center gap-1 text-muted">
                    <UIcon name="i-lucide-calendar-plus" class="text-xs" />
                    Created {{ formatDate(document.createdAt) }}
                  </span>
                  <span class="flex items-center gap-1 text-muted">
                    <UIcon name="i-lucide-calendar-clock" class="text-xs" />
                    Updated {{ formatDate(document.updatedAt) }}
                  </span>
                </div>
              </div>
            </div>
            <UButton
              icon="i-lucide-trash-2"
              color="error"
              variant="solid"
              size="sm"
              class="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity"
              @click="deleteDocument(document.id, $event)"
            />
          </UCard>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onActivated } from "vue";
import { useRoute } from "vue-router";
import { throttle } from "lodash-es";
import { getDocument, writeDocument, updateDocument } from "@/db";
import { NoteEditor } from "@/components/NoteEditor";
import NameEdit from "@/components/NameEdit.vue";
import IconEdit from "@/components/IconEdit.vue";
import DocumentsSlideover from "@/components/Document/DocumentsSlideover.vue";

import { useTabsStore } from "@/stores/tabs";
import { downloadFile } from "@/utils/file";

const toast = useToast();

const tabsStore = useTabsStore();
const route = useRoute();

const initialData = await getDocument(route.params.id as string);
const note = ref(Object.assign({
  content: '&nbsp;',
  name: 'New Note',
  icon: 'i-lucide-sticky-note',
}, initialData));

let created = !!initialData;

const throttledWatcher = throttle(async (note) => {
  if (!created) {
    await writeDocument({
      id: route.params.id as string,
      type: 'note',
      name: note.name,
      icon: note.icon,
      content: note.content,
      createdAt: new Date(),
      updatedAt: new Date(),
    });
    created = true;
  } else {
    await updateDocument(
      route.params.id as string,
      {
        type: 'note',
        content: note.content,
        name: note.name,
        icon: note.icon,
        updatedAt: new Date(),
      },
    );
  }
}, 1000);

watch(note, throttledWatcher, {
  deep: true,
});

function downloadNote() {
  const blob = new Blob([note.value.content], { type: "text/markdown" });
  const file = new File([blob], note.value.name + ".md", { type: "text/markdown" });
  downloadFile(file);
  toast.add({
    title: 'Note downloaded',
    description: `${note.value.name}.md has been downloaded`,
    icon: 'i-lucide-download',
    color: 'success'
  });
}

onActivated(() => {
  tabsStore.openTab(`/note/${route.params.id}`, note.value.name);
});
</script>

<template>
  <div class="flex-1 flex flex-col min-h-0 justify-center relative">
    <div class="absolute top-2 right-2 z-10 flex flex-row gap-2">
      <UButton
        icon="i-lucide-download"
        color="neutral"
        variant="subtle"
        @click="downloadNote"
      />
      <DocumentsSlideover />
    </div>
    <div class="flex-1 min-h-0 overflow-y-auto">
      <hgroup class="flex flex-row gap-2 items-center mx-32 mt-16 pl-[32px]">
        <IconEdit v-model:icon="note.icon" />
        <NameEdit v-model:name="note.name" />
      </hgroup>
      <div class="mx-32">
        <NoteEditor v-model="note.content" />
      </div>
    </div>
  </div>
</template>

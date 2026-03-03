<script setup lang="ts">
import { toRefs, computed, onMounted } from "vue";
import { useJournalStore } from "@/stores/journal";
import { NoteEditor } from "@/components/NoteEditor";
import JournalToolbar from "./JournalToolbar.vue";
import JournalCalendar from "./JournalCalendar.vue";
import JournalRecent from "./JournalRecent.vue";
import { throttle } from "lodash-es";

const EMPTY_DOC = '&nbsp;';

const journalStore = useJournalStore();
const { currentDate, currentJournal, recentJournals } = toRefs(journalStore);

await journalStore.loadCurrentJournal();

onMounted(async () => {
  await journalStore.loadRecentJournals();
});

function goToPreviousDay() {
  journalStore.goToPreviousDay();
}

function goToNextDay() {
  journalStore.goToNextDay();
}

function goToToday() {
  journalStore.goToToday();
}

function selectDate(date: Date) {
  journalStore.goToDate(date);
}

const throttledSave = throttle(async (content: string) => {
  const dayDate = currentDate.value;
  await journalStore.updateJournalContent(dayDate, content);
}, 1000);

const currentJournalContent = computed({
  get: () => currentJournal.value?.content || EMPTY_DOC, // &nbsp; is codemirror's empty placeholder
  set: (content: string) => {
    throttledSave(content == EMPTY_DOC ? '' : content);
  },
});
</script>

<template>
  <div class="flex h-full overflow-hidden">
    <div class="flex-1 flex flex-col min-w-0 overflow-hidden">
      <JournalToolbar
        :current-date="currentDate"
        @previous="goToPreviousDay"
        @next="goToNextDay"
        @today="goToToday"
      />
      
      <div class="flex-1 min-h-0 overflow-y-auto">
        <div class="mx-10 h-full flex">
          <NoteEditor
            v-if="currentJournal"
            v-model="currentJournalContent"
            :key="currentDate.toISOString()"
            :date="currentDate"
          />
          
          <div v-else class="flex flex-1 items-center justify-center">
            <Spinner size="lg" />
          </div>
        </div>
      </div>
    </div>
    
    <div class="w-[320px] min-w-[320px] border-l border-gray-200 dark:border-gray-700 flex flex-col overflow-hidden lg:flex hidden">
      <div class="overflow-y-auto flex-1">
        <JournalCalendar
          :current-date="currentDate"
          :highlighted-dates="Array.from(journalStore.currentMonthDates)"
          @date-select="selectDate"
        />
        <JournalRecent
          :recent-journals="recentJournals"
          @date-select="selectDate"
        />
      </div>
    </div>
  </div>
</template>

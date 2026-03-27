<script setup lang="ts">
import { computed } from "vue";
import { CalendarDate, getLocalTimeZone } from '@internationalized/date';

const props = defineProps<{
  currentDate: Date;
  highlightedDates: string[] | Set<string>;
  dateColors?: Record<string, string>;
}>();

const emit = defineEmits<{
  dateSelect: [date: Date];
}>();

const calendarDate = computed(() => {
  const d = props.currentDate;
  return new CalendarDate(d.getFullYear(), d.getMonth() + 1, d.getDate());
});

function handleDateSelect(value: any) {
  if (value && typeof value.toDate === 'function') {
    const date = value.toDate(getLocalTimeZone());
    if (date instanceof Date) {
      emit('dateSelect', date);
    }
  }
}

function getDateISO(date: any): string | null {
  if (!date || typeof date !== 'object') return null;
  const year = date.year;
  const month = String(date.month).padStart(2, '0');
  const day = String(date.day).padStart(2, '0');
  return `${year}-${month}-${day}`;
}

function hasJournalEntry(date: any): boolean {
  const dateISO = getDateISO(date);
  if (!dateISO) return false;

  const dates = Array.isArray(props.highlightedDates)
    ? props.highlightedDates
    : Array.from(props.highlightedDates);

  return dates.includes(dateISO);
}

function getDateColor(date: any): string | undefined {
  const dateISO = getDateISO(date);
  if (!dateISO || !props.dateColors) return undefined;
  return props.dateColors[dateISO];
}

function isCurrent(date: any): boolean {
  const dateISO = getDateISO(date);
  if (!dateISO) return false;
  const currentISO = `${props.currentDate.getFullYear()}-${String(props.currentDate.getMonth() + 1).padStart(2, '0')}-${String(props.currentDate.getDate()).padStart(2, '0')}`;
  return dateISO === currentISO;
}

const colorMap: Record<string, string> = {
  red: 'bg-red-500',
  orange: 'bg-orange-500',
  amber: 'bg-amber-500',
  yellow: 'bg-yellow-500',
  lime: 'bg-lime-500',
  green: 'bg-green-500',
  emerald: 'bg-emerald-500',
  teal: 'bg-teal-500',
  cyan: 'bg-cyan-500',
  sky: 'bg-sky-500',
  blue: 'bg-blue-500',
  indigo: 'bg-indigo-500',
  violet: 'bg-violet-500',
  purple: 'bg-purple-500',
  fuchsia: 'bg-fuchsia-500',
  pink: 'bg-pink-500',
  rose: 'bg-rose-500',
};
</script>

<template>
  <div class="p-4">
    <UCalendar
      v-model="calendarDate"
      @update:model-value="handleDateSelect"
    >
      <template #day="{ day }">
        <div v-if="hasJournalEntry(day)" class="flex flex-col items-center">
          <UChip
            size="2xs"
            color="primary"
            variant="subtle"
          >
            {{ day.day }}
          </UChip>
          <div
            v-if="getDateColor(day)"
            class="absolute -bottom-0.5 w-4 h-1 rounded"
            :class="colorMap[getDateColor(day) ?? ''] || 'bg-primary-500'"
          />
        </div>

        <span
          v-else-if="isCurrent(day)"
          class="font-semibold"
        >
          {{ day.day }}
        </span>

        <span v-else>
          {{ day.day }}
        </span>
      </template>
    </UCalendar>
  </div>
</template>

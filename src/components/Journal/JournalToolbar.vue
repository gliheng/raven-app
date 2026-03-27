<script setup lang="ts">
import { computed } from "vue";
import moment from "moment";

const props = defineProps<{
  currentDate: Date;
  currentColor?: string;
}>();

const emit = defineEmits<{
  previous: [];
  next: [];
  today: [];
  colorChange: [color: string | undefined];
}>();

const dateDisplay = computed(() => {
  return moment(props.currentDate).format('MMM D, YYYY');
});

const colors = [
  { name: 'Default', value: undefined, class: 'text-gray-400' },
  { name: 'Red', value: 'red', class: 'text-red-500' },
  { name: 'Orange', value: 'orange', class: 'text-orange-500' },
  { name: 'Amber', value: 'amber', class: 'text-amber-500' },
  { name: 'Yellow', value: 'yellow', class: 'text-yellow-500' },
  { name: 'Lime', value: 'lime', class: 'text-lime-500' },
  { name: 'Green', value: 'green', class: 'text-green-500' },
  { name: 'Emerald', value: 'emerald', class: 'text-emerald-500' },
  { name: 'Teal', value: 'teal', class: 'text-teal-500' },
  { name: 'Cyan', value: 'cyan', class: 'text-cyan-500' },
  { name: 'Sky', value: 'sky', class: 'text-sky-500' },
  { name: 'Blue', value: 'blue', class: 'text-blue-500' },
  { name: 'Indigo', value: 'indigo', class: 'text-indigo-500' },
  { name: 'Violet', value: 'violet', class: 'text-violet-500' },
  { name: 'Purple', value: 'purple', class: 'text-purple-500' },
  { name: 'Fuchsia', value: 'fuchsia', class: 'text-fuchsia-500' },
  { name: 'Pink', value: 'pink', class: 'text-pink-500' },
  { name: 'Rose', value: 'rose', class: 'text-rose-500' },
];

const currentColorObj = computed(() => {
  return colors.find(c => c.value === props.currentColor) || colors[0];
});

function selectColor(color: string | undefined) {
  emit('colorChange', color);
}
</script>

<template>
  <div class="flex items-center justify-between py-3 px-6 border-b border-white/10">
    <div class="flex items-center gap-2">
      <UButton
        icon="i-lucide-chevron-left"
        variant="ghost"
        color="neutral"
        size="sm"
        @click="emit('previous')"
      />
      <UButton
        :label="dateDisplay"
        variant="subtle"
        color="neutral"
        size="sm"
      />
      <UButton
        icon="i-lucide-chevron-right"
        variant="ghost"
        color="neutral"
        size="sm"
        @click="emit('next')"
      />
    </div>
    
    <div class="flex items-center gap-2">
      <UDropdownMenu
        :items="colors.map(color => ({
          label: color.name,
          icon: color.value ? `i-mdi-circle` : 'i-mdi-circle-outline',
          ui: {
            itemLeadingIcon: color.class,
          },
          onSelect: () => selectColor(color.value),
        }))"
      >
        <UButton
          variant="ghost"
          color="neutral"
          size="sm"
          :icon="currentColorObj.value ? 'i-mdi-circle' : 'i-mdi-circle-outline'"
          :ui="{
            leadingIcon: currentColorObj.class,
          }"
          :style1="currentColorObj.value ? `--ui-neutral: var(${currentColorObj.value})` : undefined"
        />
      </UDropdownMenu>
      
      <UButton
        label="Today"
        variant="outline"
        color="primary"
        size="sm"
        @click="emit('today')"
      />
    </div>
  </div>
</template>

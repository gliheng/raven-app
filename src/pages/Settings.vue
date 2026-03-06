<script setup lang="ts">
import { onActivated } from "vue";
import { useTabsStore } from "@/stores/tabs";
import Settings from "@/components/Settings/Settings.vue";

const tabsStore = useTabsStore();

onActivated(() => {
  tabsStore.openTab("/settings", "Settings");
});
</script>

<template>
  <Settings />
</template>

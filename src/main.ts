import { createApp } from "vue";
import { createPinia, Pinia } from "pinia";
import ui from "@nuxt/ui/vue-plugin";
import { init as initNative } from "./native";
import { router } from "./router";
import App from "./App.vue";
import { init as initDb } from "./db";
import { initSupabase } from "./lib/supabase";
import { useChatsStore } from "./stores/chats";
import { useAuthStore } from "./stores/auth";
import "./assets/style.css";
import { useSettingsStore } from "./stores/settings";
import { usePetStore } from "./stores/pet";

(async () => {
  const app = createApp(App);
  const pinia = createPinia();

  await initDb();
  await initSupabase();
  await initNative();
  await initDataStores(pinia);

  app.use(pinia);
  app.use(router);
  app.use(ui);
  app.mount("#app");
})();

async function initDataStores(pinia: Pinia) {
  const chatsStore = useChatsStore(pinia);
  chatsStore.loadChats();

  console.log('Initializing settings store...');
  const settingsStore = useSettingsStore();
  await settingsStore.initialize();

  if (navigator.onLine) {
    // Initialize auth store
    console.log('Initializing auth store...');
    const authStore = useAuthStore(pinia);
    await authStore.initialize();
  }

  // Initialize pet store
  const petStore = usePetStore();
  await petStore.initialize();
  if (petStore.enabled && petStore.activeSlug) {
    await petStore.showPet();
  }
}

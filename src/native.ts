import { TrayIcon } from "@tauri-apps/api/tray";
import { defaultWindowIcon } from "@tauri-apps/api/app";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Menu, MenuItem, CheckMenuItem } from "@tauri-apps/api/menu";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event"
import { usePetStore } from "@/stores/pet";
import { eventBus } from "@/utils/eventBus";

let petToggle: CheckMenuItem | null = null;

function syncMenu() {
  if (!petToggle) return;
  const s = usePetStore();
  petToggle.setChecked(s.enabled).catch(() => {});
  petToggle.setText(s.enabled ? "Hide Pet" : "Show Pet").catch(() => {});
}

export async function init() {
  const showWindow = await MenuItem.new({
    text: "Show Window",
    action: async () => {
      const win = getCurrentWindow();
      if (await win.isMinimized()) await win.unminimize();
      if (!await win.isVisible()) await win.show();
      await win.setFocus();
    },
  });

  petToggle = await CheckMenuItem.new({
    text: "Show Pet",
    checked: false,
    enabled: true,
    action: async () => {
      try {
        await usePetStore().togglePet();
      } catch (e) {
        console.error("[Tray] Pet toggle failed:", e);
      }
    },
  });

  eventBus.on("set_pet", async ({ enabled, slug }) => {
    if (enabled !== undefined) {
      if (enabled) {
        await invoke("create_pet_cmd");
        await invoke("save_pet_state", { state: { enabled: true } });
      } else {
        await invoke("destroy_pet_cmd");
        await invoke("save_pet_state", { state: { enabled: false } });
      }
    }
    if (slug !== undefined) {
      await invoke("save_pet_state", { state: { slug } });
      emit("pet:reload", {})
    }
    syncMenu();
  });

  const menu = await Menu.new({ items: [showWindow, petToggle] });
  await TrayIcon.new({ icon: (await defaultWindowIcon()) as any, menu });
}

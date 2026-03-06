import { createRouter, createWebHashHistory } from "vue-router";
import { nanoid } from "nanoid";
import NotFound from "./pages/NotFound.vue";
import DefaultLayout from "./layouts/DefaultLayout.vue";

export const router = createRouter({
  routes: [
    {
      name: "home",
      path: "/",
      component: DefaultLayout,
      redirect() {
        const id = nanoid();
        sessionStorage.setItem("lastChatId", id);
        return { name: "chat", params: { id } };
      },
      children: [
        { name: "chat", path: "chat/:id", component: () => import("./pages/Chat.vue")  },
        { name: "agent", path: "agent/:id", component: () => import("./pages/Agent.vue")  },
        { name: "note", path: "note/:id", component: () => import("./pages/Note.vue") },
        { name: "chart", path: "chart/:id", component: () => import("./pages/Chart.vue") },
        { name: "image", path: "image", component: () => import("./pages/Image.vue") },
        { name: "documents", path: "documents", component: () => import("./pages/Documents.vue") },
        { name: "journal", path: "journal", component: () => import("./pages/Journal.vue") },
        { name: "settings", path: "settings", component: () => import("./pages/Settings.vue") },
        { name: "acpdebug", path: "acpdebug", component: () => import("./pages/AcpDebug.vue") },
        { name: "editor", path: "/editor", component: () => import("./pages/TestEditor.vue") },
        { name: "msglist", path: "/msglist", component: () => import("./pages/TestMessageList.vue") },
      ],
    },
    { name: "404", path: "/:pathMatch(.*)*", component: NotFound },
  ],
  history: createWebHashHistory(),
});

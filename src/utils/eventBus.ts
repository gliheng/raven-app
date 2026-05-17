import mitt from "mitt";

export const eventBus = mitt<{
  lightbox: string | string[];
  artifact: string;
  acp_message_sent: { program: string; message: any; timestamp: number };
  acp_message_received: { program: string; message: string; timestamp: number };
  chat_created: { id: string; topic: string };
  chat_updated: { id: string };
  toast: { title: string; description: string; icon: string; color: string };
  tab_notify: { path: string };
  set_pet: { enabled?: boolean; slug?: string };
}>();

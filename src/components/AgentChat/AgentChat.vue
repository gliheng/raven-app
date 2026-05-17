<script setup lang="ts">
import { ref, onMounted, onUnmounted, PropType, computed, useTemplateRef, watch } from "vue";
import { AnimatePresence } from "motion-v";
import { Chat, updateChat, writeChat, type Agent } from "@/db";
import ChatBox from "@/components/ChatBox.vue";
import MessageList from "./MessageList.vue";
import ModeSelector from "./ModeSelector.vue";
import SlashCommandMenu from "./SlashCommandMenu.vue";
import ContextDisplay from "./ContextDisplay.vue";
import { generateTopic } from "@/llm/prompt";
import { useTabsStore } from "@/stores/tabs";
import { useAcp } from "@/composables/useAcp";
import { eventBus } from "@/utils/eventBus";
import { invoke } from "@tauri-apps/api/core";
import Mention from "@tiptap/extension-mention";
import MentionMenu from "./MentionMenu.vue";
import { type Editor } from "@tiptap/core";
import {
  type EmbeddedResourceResource,
  type ContentBlock,
} from "@agentclientprotocol/sdk";
import { useFloating, offset, flip, shift, autoUpdate } from "@floating-ui/vue";
import mime from "mime";
import { isTextFile, type FileAttachment } from "@/utils/file";
import { emit } from "@tauri-apps/api/event";
import { usePetStore } from "@/stores/pet";

const toast = useToast();

const props = defineProps({
  agent: {
    type: Object as PropType<Agent>,
    required: true,
  },
  chatId: {
    type: String,
    required: true,
  },
  chat: {
    type: Object as PropType<Chat>,
    required: false,
  },
});

const isInitialized = ref(false);
const input = ref("");
const error = ref<string | null>(null);
const sessionId = ref<string | null>(props.chat?.ext?.sessionId ?? null);
const chatBoxRef = ref<InstanceType<typeof ChatBox> | null>(null);
const expanded = ref(false);
const selectionContextRef = useTemplateRef<InstanceType<typeof ContextDisplay> | null>("selectionContextRef");

const { client, messages, status, currentModeId, availableModes, availableCommands } = useAcp({
  chatId: props.chatId,
  agent: props.agent,
  onInvoke(method, params) {
    if (method === 'session/update') {
      if (params.update.sessionUpdate === "agent_message_chunk") {
        status.value = 'streaming';
      }
    }
  },
});

// Pet integration: drive pet animations from agent status
const petStore = usePetStore();
watch(status, (newStatus) => {
  if (!petStore.enabled || !newStatus) return
  const map: Record<string, string> = {
    submitted: "running",
    streaming: "review",
    ready: "idle",
    error: "failed",
  }
  const petState = map[newStatus]
  if (petState) emit("pet:state-change", { state: petState })
})

const nonInteractive = computed(
  () => status.value == "submitted" || status.value == "streaming",
);
const hasModes = computed(() => availableModes.value.length > 0);
const hasCommands = computed(() => availableCommands.value.length > 0);

const viewWidth = computed(() =>
  expanded.value ? undefined : Math.min(screen.width / 3, 600),
);

const showMessageList = computed(() => {
  return messages.value.length || status.value != "ready";
});

const artifactKey = computed(() => `workspace::${props.agent.directory}`);

async function handleModeChange(modeId: string) {
  if (!client) return;
  try {
    currentModeId.value = modeId;
    await client.connection.setSessionMode({
      sessionId: sessionId.value!,
      modeId,
    });
  } catch (err) {
    console.error('Failed to set mode:', err);
    error.value = `Failed to set mode: ${JSON.stringify(err, null, 2)}`;
  }
}

function handleMentionInsert() {
  if (!chatBoxRef.value) return;

  const editor = (chatBoxRef.value as any).editor;
  if (!editor) return;

  const { state } = editor;
  const { from } = state.selection;

  // Check if cursor is at the start or if previous character is a space
  const textBefore = state.doc.textBetween(Math.max(0, from - 1), from);
  const needsSpace = from > 0 && textBefore !== ' ' && textBefore !== '\n';

  // Insert space before @ if needed
  const textToInsert = needsSpace ? ' @' : '@';
  chatBoxRef.value.insertText(textToInsert);
}

function handleSlashInsert() {
  if (!chatBoxRef.value) return;

  const editor = (chatBoxRef.value as any).editor;
  if (!editor) return;

  const { state } = editor;
  const { from } = state.selection;

  // Check if cursor is at the start or if previous character is a space
  const textBefore = state.doc.textBetween(Math.max(0, from - 1), from);
  const needsSpace = from > 0 && textBefore !== ' ' && textBefore !== '\n';

  // Insert space before / if needed
  const textToInsert = needsSpace ? ' /' : '/';
  chatBoxRef.value.insertText(textToInsert);
}

const start = async () => {
  try {
    error.value = null;
    console.log("Starting agent initialization...");

    await client.initialize();

    const enableLoadSession = client.hasCapability("loadSession");

    if (enableLoadSession && props.chat && sessionId.value) {
      status.value = 'streaming';
      await client.sessionLoad(sessionId.value);
      status.value = 'ready';
    } else {
      const ret = await client.sessionNew();
      sessionId.value = ret.sessionId;
    }

    console.log("Agent initialized successfully");
    isInitialized.value = true;
  } catch (err) {
    console.error("Failed to launch agent:", err);
    error.value = `Failed to launch agent: ${JSON.stringify(err, null, 2)}`;
    status.value = "error";
  }
};

const stop = async () => {
  try {
    error.value = null;
    console.log("Stopping agent...");

    if (client && isInitialized.value) {
      await client.dispose();
    }

    status.value = "ready";
    isInitialized.value = false;
    console.log("Agent stopped successfully");
  } catch (err) {
    console.error("Failed to stop agent:", err);
    error.value = `Failed to stop agent: ${err}`;
    console.log(`Error stopping agent: ${JSON.stringify(err, null, 2)}`);
    status.value = "error";
  }
};

/**
 * Find all mention nodes in the editor JSON (recursive)
 */
function findMentions(json: any): string[] {
  const mentions: string[] = [];

  function traverse(node: any) {
    if (!node) return;

    // Check if this node is a mention
    if (node.type === 'mention' && node.attrs?.id) {
      mentions.push(node.attrs.id);
    }

    // Recursively traverse content array
    if (Array.isArray(node?.content)) {
      for (const child of node.content) {
        traverse(child);
      }
    }
  }

  // Start traversal from root content
  if (Array.isArray(json?.content)) {
    for (const item of json.content) {
      traverse(item);
    }
  }

  return mentions;
}

/**
 * Find and read files matching the mentioned paths
 */
async function findMentionedFiles(
  mentionPaths: string[],
  baseDirectory: string
): Promise<Array<{ type: 'resource'; resource: EmbeddedResourceResource }>> {
  const resourceParts: Array<{ type: 'resource'; resource: EmbeddedResourceResource }> = [];

  for (const filePath of mentionPaths) {
    const fullPath = `${baseDirectory}/${filePath}`;

    // Get MIME type based on file extension
    const mimeType = getMimeType(filePath);
    resourceParts.push({
      type: 'resource',
      resource: {
        uri: `file://${fullPath}`,
        mimeType,
        [isTextFile(mimeType) ? 'text' : 'blob']: '', // Data omitted to save token
      } as any,
    });
  }

  return resourceParts;
}

/**
 * Build content blocks from text, mentions, and attachments
 */
async function buildContentBlocks({
  text, attachments, editorJson, baseDirectory
}: {
  text: string,
  attachments?: FileAttachment[],
  editorJson: any,
  baseDirectory: string
}): Promise<ContentBlock[]> {
  const parts: ContentBlock[] = [
    {
      type: 'text',
      text,
    },
  ];

  // Step 1: Append attachments as content blocks if provided
  if (attachments?.length) {
    for (const attachment of attachments) {
      if (attachment.contentType && attachment.url) {
        // Handle URL-based attachments (data URLs, blob URLs, etc.)
        if (attachment.url.startsWith('data:')) {
          // Parse data URL: data:mimeType;base64,data
          const [meta, base64Data] = attachment.url.split(',', 2);
          const mimeType = meta.split(':')[1].split(';')[0];

          if (mimeType.startsWith('image/')) {
            // For images, use ImageContent
            parts.push({
              type: 'image',
              data: base64Data,
              mimeType,
            } satisfies ContentBlock);
          } else if (mimeType.startsWith('audio/')) {
            // For audio, use AudioContent
            parts.push({
              type: 'audio',
              data: base64Data,
              mimeType,
            } satisfies ContentBlock);
          } else if (isTextFile(mimeType)) {
            // For text files, use EmbeddedResource with 'text' field
            const textContent = atob(base64Data);
            parts.push({
              type: 'resource',
              resource: {
                uri: attachment.name || attachment.url,
                mimeType,
                text: textContent,
              },
            } satisfies ContentBlock);
          } else {
            // For other binary files, use EmbeddedResource with 'blob' field
            parts.push({
              type: 'resource',
              resource: {
                uri: attachment.name || attachment.url,
                mimeType,
                blob: base64Data,
              },
            } satisfies ContentBlock);
          }
        } else {
          // For regular URL attachments, we'd need to fetch the content
          // For now, skip as we need the actual content
          console.log(`Skipping URL attachment without content: ${attachment.url}`);
        }
      }
    }
  }

  // Step 2: Find mentions in the editor JSON
  const mentionPaths = findMentions(editorJson);
  // Step 3: Find and read files matching those mentions
  const resourceParts = await findMentionedFiles(mentionPaths, baseDirectory);
  // Add resource parts to the parts array
  parts.push(...resourceParts);

  return parts;
}

const handleSubmit = async (data: { attachments?: FileAttachment[] }) => {
  if (!input.value.trim() || !isInitialized.value || !client) return;

  try {
    const text = input.value.trim();

    // Get editor JSON to find mentions
    const editor = (chatBoxRef.value as any)?.editor;
    const json = editor?.getJSON();
    const baseDirectory = props.agent.directory;

    // Build content blocks with text, mentions, and attachments
    const parts = await buildContentBlocks({
      text,
      attachments: data.attachments,
      editorJson: json,
      baseDirectory,
    });

    // Add selection context if visible
    const contextParts = await selectionContextRef.value?.buildContextParts(baseDirectory);
    if (contextParts) {
      parts.push(...contextParts);
    }

    messages.value.push({
      id: String(messages.value.length),
      role: "user",
      content: text,
      parts: parts as any,
    });

    error.value = null;
    input.value = "";
    status.value = "submitted";

    if (!props.chat && messages.value.length === 1) {
      // Async update chat meta data
      (async () => {
        const { text: topic } = await generateTopic(messages.value[0].content);
        const agentChat = {
          id: props.chatId,
          topic: topic,
          createdAt: new Date(),
          updatedAt: new Date(),
          agentId: props.agent.id,
          ext: {
            sessionId: sessionId.value ?? undefined,
          },
        };
        await writeChat(agentChat);
        eventBus.emit("chat_created", { id: props.chatId, topic });
        useTabsStore().setTitle(`/chat/${props.chatId}`, topic);
      })();
    } else {
      // Async update chat meta data
      (async () => {
        await updateChat(props.chatId, {
          updatedAt: new Date(),
        });
        eventBus.emit("chat_updated", { id: props.chatId });
      })();
    }

    const ret = await client.connection.prompt({
      sessionId: sessionId.value!,
      prompt: parts,
    });
    console.log('sessionPrompt result', ret);
    status.value = "ready";
  } catch (err) {
    console.error("Failed to send message:", err);
    error.value = `Failed to send message: ${JSON.stringify(err, null, 2)}`;
    status.value = "error";
  }
};

// Helper function to get MIME type based on file extension
function getMimeType(filePath: string): string {
  return mime.getType(filePath) || 'text/plain';
}

function cancel() {
  client.connection.cancel({
    sessionId: sessionId.value!,
  });
  status.value = 'ready';
}

onMounted(() => {
  start();
});

onUnmounted(() => {
  stop();
});

watch(status, (newStatus, oldStatus) => {
  if ((oldStatus === 'streaming' || oldStatus === 'submitted') && newStatus === 'ready') {
    eventBus.emit('tab_notify', { path: `/chat/${props.chatId}` });
  }
});

interface MentionItem {
  id: string;
  label: string;
  path: string;
  is_dir: boolean;
}

let editorInstance: Editor | null = null;
function selectMention(index: number) {
  const item = mentionItems.value[index];
  if (item && mentionRange.value && editorInstance) {
    editorInstance.chain()
      .focus()
      .insertContentAt(mentionRange.value, [{
        type: 'mention',
        attrs: { id: item.id, label: item.path }
      }])
      .run();
    mentionMenuOpen.value = false;
  }
}

function selectSlashCommand(index: number) {
  const item = slashCommandItems.value[index];
  if (item && slashRange.value && editorInstance) {
    editorInstance.chain()
      .focus()
      .insertContentAt(slashRange.value, `/${item.name}`)
      .run();
    slashMenuOpen.value = false;
  }
}

interface FileSuggestion {
  name: string;
  path: string;
  is_dir: boolean;
}

async function globFiles(directory: string, pattern?: string): Promise<FileSuggestion[]> {
  try {
    return invoke<FileSuggestion[]>('glob_files', {
      directory,
      pattern,
    });
  } catch (error) {
    toast.add({
      title: 'Failed to glob files',
      description: error instanceof Error ? error.message : String(error),
      icon: 'i-lucide-alert-circle',
      color: 'error'
    });
    return [];
  }
}

const mentionMenuOpen = ref(false);
const mentionItems = ref<MentionItem[]>([]);
const mentionQuery = ref("");
const selectedIndex = ref(0);
const mentionRange = ref<{ from: number; to: number } | null>(null);

const slashMenuOpen = ref(false);
const slashCommandItems = ref<{ name: string; description: string }[]>([]);
const slashSelectedIndex = ref(0);
const slashRange = ref<{ from: number; to: number } | null>(null);
const slashQuery = ref("");

// Click outside handler
const handleClickOutside = (event: MouseEvent) => {
  // mentionMenuRef is a component ref, so we need $el to get the DOM element
  const menuEl = mentionMenuRef.value?.$el;
  const slashMenuEl = slashMenuRef.value?.$el;
  const chatBoxEl = chatBoxRef.value?.$el;

  // Check if click is outside mention menu and chatbox
  if (
    menuEl &&
    !menuEl.contains(event.target as Node) &&
    chatBoxEl &&
    !chatBoxEl.contains(event.target as Node)
  ) {
    mentionMenuOpen.value = false;
  }

  // Check if click is outside slash menu and chatbox
  if (
    slashMenuEl &&
    !slashMenuEl.contains(event.target as Node) &&
    chatBoxEl &&
    !chatBoxEl.contains(event.target as Node)
  ) {
    slashMenuOpen.value = false;
  }
};

// Add click listener with { once: true } when menu opens
watch([mentionMenuOpen, slashMenuOpen], ([isMentionOpen, isSlashOpen]) => {
  if (isMentionOpen || isSlashOpen) {
    document.addEventListener('click', handleClickOutside, { once: true });
  }
});

// Virtual element for cursor position
interface VirtualElement {
  getBoundingClientRect: () => DOMRect;
}

const virtualReference = ref<VirtualElement | null>(null);
const mentionMenuRef = ref<any>(null);
const virtualReferenceSlash = ref<VirtualElement | null>(null);
const slashMenuRef = ref<any>(null);

const { floatingStyles } = useFloating(virtualReference, mentionMenuRef, {
  placement: 'bottom-start',
  middleware: [
    offset(25),
    flip({
      fallbackPlacements: ['top-start'],
    }),
    shift({
      padding: 10,
    }),
  ],
  whileElementsMounted: autoUpdate,
});

const { floatingStyles: floatingStylesSlash } = useFloating(virtualReferenceSlash, slashMenuRef, {
  placement: 'bottom-start',
  middleware: [
    offset(25),
    flip({
      fallbackPlacements: ['top-start'],
    }),
    shift({
      padding: 10,
    }),
  ],
  whileElementsMounted: autoUpdate,
});

const mentionExtension = Mention.configure({
  HTMLAttributes: {
    class: 'mention',
  },
  suggestion: {
    char: '@',
    items: async ({ query }) => {
      mentionQuery.value = query;
      const files = await globFiles(props.agent.directory!, query);
      mentionItems.value = files.map((file): MentionItem => ({
        id: file.path,
        label: file.name,
        path: file.path,
        is_dir: file.is_dir,
      }));
      selectedIndex.value = 0;
      return mentionItems.value;
    },
    render: () => {
      return {
        onStart: (props: any) => {
          if (!props.clientRect) {
            return
          }
          mentionRange.value = { from: props.range.from, to: props.range.to };
          const { from } = props.range;
          const { view } = props.editor;
          const coords = view.coordsAtPos(from);

          // Create virtual element for cursor position
          virtualReference.value = {
            getBoundingClientRect() {
              return {
                width: 0,
                height: 0,
                x: coords.left,
                y: coords.top,
                top: coords.top,
                left: coords.left,
                right: coords.right,
                bottom: coords.bottom,
                toJSON() {
                  return this;
                },
              };
            },
          };
          mentionMenuOpen.value = true;
          editorInstance = props.editor;
        },

        onUpdate(props: any) {
          mentionRange.value = { from: props.range.from, to: props.range.to };
          const { from } = props.range;
          const { view } = props.editor;
          const coords = view.coordsAtPos(from);

          // Update virtual element position
          virtualReference.value = {
            getBoundingClientRect() {
              return {
                width: 0,
                height: 0,
                x: coords.left,
                y: coords.top,
                top: coords.top,
                left: coords.left,
                right: coords.right,
                bottom: coords.bottom,
                toJSON() {
                  return this;
                },
              };
            },
          };
        },

        onKeyDown(props: any) {
          if (props.event.key === 'Escape') {
            mentionMenuOpen.value = false;
            return true;
          }

          if (props.event.key === 'ArrowUp') {
            selectedIndex.value = (selectedIndex.value - 1 + mentionItems.value.length) % mentionItems.value.length;
            return true;
          }

          if (props.event.key === 'ArrowDown') {
            selectedIndex.value = (selectedIndex.value + 1) % mentionItems.value.length;
            return true;
          }

          if (props.event.key === 'ArrowLeft' || props.event.key === 'ArrowRight') {
            // Allow arrow keys to pass through to the editor for navigation
            return false;
          }

          const key = props.event.key;
          // Allow tab and enter to pass through to the editor for navigation
          if (key === 'Tab' || key === 'Enter') {
            props.event.stopPropagation();
            const item = mentionItems.value[selectedIndex.value];
            if (item) {
              selectMention(selectedIndex.value);
            }
            return true;
          }

          return false;
        },

        onExit() {
          mentionMenuOpen.value = false;
          mentionItems.value = [];
          mentionRange.value = null;
        },
      };
    },
  },
});

const slashExtension = Mention.extend({ name: 'slash' }).configure({
  HTMLAttributes: {
    class: 'slash',
  },
  suggestion: {
    char: '/',
    items: async ({ query }) => {
      slashQuery.value = query;
      const filteredCommands = availableCommands.value
        .filter(cmd => cmd.name.toLowerCase().includes(query.toLowerCase()) ||
                     cmd.description.toLowerCase().includes(query.toLowerCase()));
      slashCommandItems.value = filteredCommands.map(cmd => ({
        name: cmd.name,
        description: cmd.description,
      }));
      slashSelectedIndex.value = 0;
      return slashCommandItems.value;
    },
    render: () => {
      return {
        onStart: (props: any) => {
          if (!props.clientRect) {
            return
          }
          slashRange.value = { from: props.range.from, to: props.range.to };
          const { from } = props.range;
          const { view } = props.editor;
          const coords = view.coordsAtPos(from);

          // Create virtual element for cursor position
          virtualReferenceSlash.value = {
            getBoundingClientRect() {
              return {
                width: 0,
                height: 0,
                x: coords.left,
                y: coords.top,
                top: coords.top,
                left: coords.left,
                right: coords.right,
                bottom: coords.bottom,
                toJSON() {
                  return this;
                },
              };
            },
          };
          slashMenuOpen.value = true;
          editorInstance = props.editor;
        },

        onUpdate(props: any) {
          slashRange.value = { from: props.range.from, to: props.range.to };
          const { from } = props.range;
          const { view } = props.editor;
          const coords = view.coordsAtPos(from);

          // Update virtual element position
          virtualReferenceSlash.value = {
            getBoundingClientRect() {
              return {
                width: 0,
                height: 0,
                x: coords.left,
                y: coords.top,
                top: coords.top,
                left: coords.left,
                right: coords.right,
                bottom: coords.bottom,
                toJSON() {
                  return this;
                },
              };
            },
          };
        },

        onKeyDown(props: any) {
          if (props.event.key === 'Escape') {
            slashMenuOpen.value = false;
            return true;
          }

          if (props.event.key === 'ArrowUp') {
            slashSelectedIndex.value = (slashSelectedIndex.value - 1 + slashCommandItems.value.length) % slashCommandItems.value.length;
            return true;
          }

          if (props.event.key === 'ArrowDown') {
            slashSelectedIndex.value = (slashSelectedIndex.value + 1) % slashCommandItems.value.length;
            return true;
          }

          if (props.event.key === 'ArrowLeft' || props.event.key === 'ArrowRight') {
            // Allow arrow keys to pass through to the editor for navigation
            return false;
          }

          const key = props.event.key;
          // Allow tab and enter to pass through to the editor for navigation
          if (key === 'Tab' || key === 'Enter') {
            props.event.stopPropagation();
            const item = slashCommandItems.value[slashSelectedIndex.value];
            if (item) {
              selectSlashCommand(slashSelectedIndex.value);
            }
            return true;
          }

          return false;
        },

        onExit() {
          slashMenuOpen.value = false;
          slashCommandItems.value = [];
          slashRange.value = null;
        },
      };
    },
  },
});
</script>

<template>
  <div class="flex-1 flex flex-col min-h-0 justify-center relative">
    <UAlert
      v-if="error"
      title="Error!"
      :description="error"
      color="error"
      icon="i-lucide-alert-octagon"
      :ui="{
        icon: 'size-10'
      }"
      class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-20 max-w-2xl w-full"
    />
    <section v-else-if="!isInitialized" class="flex-1 flex flex-col justify-center items-center gap-4">
      <Spinner />
      <p class="text-gray-500 text-sm">Initializing agent...</p>
    </section>
    <template v-else>
      <header class="absolute top-2 right-2 z-10">
        <UButton
          icon="i-mdi-arrow-expand-horizontal"
          color="neutral"
          variant="subtle"
          @click="expanded = !expanded"
        />
      </header>
      <AnimatePresence>
        <div v-if="showMessageList" class="flex-1 overflow-y-auto min-h-0 flex justify-center">
          <MessageList
            key="message-list"
            animate="visible"
            :messages="messages"
            :status="status"
            :width="viewWidth"
            :variants="{
              visible: { maxHeight: '100%' },
              hidden: { maxHeight: '0' },
            }"
          />
        </div>
        <header v-else class="mx-auto text-3xl font-semibold mb-3">
          Let's work on something!
        </header>
      </AnimatePresence>
      <div class="px-8 my-4">
        <ChatBox
          ref="chatBoxRef"
          class="chat-box mx-auto"
          v-model="input"
          :style="{ maxWidth: viewWidth ? `${viewWidth}px` : '100%' }"
          :status="status"
          :messages="messages"
          @submit="handleSubmit"
          @stop="cancel"
          :extensions="[
            mentionExtension,
            slashExtension,
          ]"
        >
          <template #left-addons>
            <UTooltip text="Add files as context">
              <UButton
                icon="i-lucide-at-sign"
                color="primary"
                variant="soft"
                size="sm"
                :disabled="nonInteractive"
                @click.stop="handleMentionInsert"
              />
            </UTooltip>
            <UTooltip text="Run command">
              <UButton
                icon="i-lucide-command"
                color="primary"
                variant="soft"
                size="sm"
                :disabled="nonInteractive"
                @click.stop="handleSlashInsert"
              />
            </UTooltip>
            <ModeSelector
              v-if="hasModes"
              v-model="currentModeId"
              :available-modes="availableModes"
              :disabled="nonInteractive"
              @update:modelValue="handleModeChange"
            />
            <ContextDisplay
              ref="selectionContextRef"
              v-if="artifactKey"
              :artifact-key="artifactKey"
            />
            <MentionMenu
              v-if="mentionMenuOpen"
              ref="mentionMenuRef"
              :mention-items="mentionItems"
              :selected-index="selectedIndex"
              :floating-styles="floatingStyles"
              @select="selectMention($event)"
            />
            <SlashCommandMenu
              v-if="slashMenuOpen"
              ref="slashMenuRef"
              :command-items="slashCommandItems"
              :selected-index="slashSelectedIndex"
              :floating-styles="floatingStylesSlash"
              @select="selectSlashCommand($event)"
            />
          </template>
        </ChatBox>
      </div>
    </template>
  </div>
</template>

<style lang="scss" scoped>
.chat-box {
  :deep(.mention) {
    background: rgba(59, 130, 246, 0.1);
    color: #3b82f6;
    border-radius: 0.25rem;
    padding: 0 0.25rem;
    font-weight: 500;
  }
  :deep(.slash) {
    background: rgba(139, 92, 246, 0.1);
    color: #8b5cf6;
    border-radius: 0.25rem;
    padding: 0 0.25rem;
    font-weight: 500;
  }
}
</style>

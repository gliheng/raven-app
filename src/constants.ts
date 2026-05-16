import { InjectionKey, Ref } from "vue";
import { FileUIPart, UIMessage } from "ai";
import { Chat } from "@ai-sdk/vue";
import { FileEntry } from "./components/WorkspaceEditor/types";

export const CHAT_ACTIONS = Symbol("chat-actions") as InjectionKey<{
  chat: Chat<UIMessage>;
  regenerate: () => Promise<void>;
  sendMessage: (message: {
    text: string;
    files?: FileUIPart[];
  }) => Promise<void>;
}>;
export const MESSAGE_GRAPH = Symbol("message-graph") as InjectionKey<{
  graph: Ref<Record<string, any>>;
  select: (id: string, i: number) => void;
}>;
export const ROOT_NODE_ID = "__root";

export const SUPABASE_URL = "https://xgazvyjwnjwablelrrsc.supabase.co";
export const SUPABASE_ANON_KEY = "sb_publishable_wbwXXEx1TFLEz7zKTFHkOQ_HQaHIwAF";

export const EDITOR_ACTIONS = Symbol("editor-actions") as InjectionKey<{
  listFiles(relativePath: string): Promise<FileEntry[]>;
  loadFileContent(relativePath: string): Promise<string>;
  saveFileContent(relativePath: string, content: string): Promise<void>;
}>;

export interface AgentProgramOption {
  value: string;
  label: string;
}

export const AGENT_PROGRAMS: AgentProgramOption[] = [
  { value: 'codex', label: 'Codex' },
  { value: 'gemini', label: 'Gemini CLI' },
  { value: 'claude', label: 'Claude Code' },
  { value: 'qwen', label: 'Qwen Code' },
  { value: 'opencode', label: 'OpenCode' },
];

export const MODELS_BY_PROVIDER = {
  deepseek: [
    {
      label: "Deepseek",
      value: "deepseek-chat",
    },
    {
      label: "Deepseek R1",
      value: "deepseek-reasoner",
    },
  ],
  minimax: [
    {
      label: "Minimax M2.5",
      value: "MiniMax-M2.5",
    },
    {
      label: "Minimax M2.1",
      value: "MiniMax-M2.1",
    },
    {
      label: "Minimax M2",
      value: "MiniMax-M2",
    },
  ],
  zai: [
    {
      label: "GLM 5",
      value: "glm-5",
    },
    {
      label: "GLM 4.7",
      value: "glm-4.7",
    },
    {
      label: "GLM 4.6v",
      value: "glm-4.6v",
    },
    {
      label: "GLM 4.7 Flash",
      value: "glm-4.7-flash",
    },
    {
      label: "GLM 4.5 Flash",
      value: "glm-4.5-flash",
    },
  ],
  openrouter: [
    {
      label: "Claude haiku 4.5",
      value: "anthropic/claude-haiku-4.5",
    },
    {
      label: "Claude opus 4.5",
      value: "anthropic/claude-opus-4.5",
    },
    {
      label: "Claude sonnet 4.5",
      value: "anthropic/claude-sonnet-4.5",
    },
    {
      label: "Claude haiku 4",
      value: "anthropic/claude-haiku-4",
    },
    {
      label: "Claude opus 4",
      value: "anthropic/claude-opus-4",
    },
    {
      label: "Claude sonnet 4",
      value: "anthropic/claude-sonnet-4",
    },
    {
      label: "Gemini 2.5 flash",
      value: "google/gemini-2.5-flash",
    },
    {
      label: "Gemini 2.5 pro",
      value: "google/gemini-2.5-pro-preview",
    },
  ],
  siliconflow: [
    {
      label: "Qwen3 32B",
      value: "Qwen/Qwen3-32B",
    },
    {
      label: "QwQ 32B",
      value: "Qwen/QwQ-32B",
    },
    {
      label: "MiniMax M2",
      value: "MiniMaxAI/MiniMax-M2",
    },
    {
      label: "MiniMax M2.1",
      value: "MiniMaxAI/MiniMax-M2.1",
    },
    {
      label: "MiniMax M2.5",
      value: "Pro/MiniMaxAI/MiniMax-M2.5",
    },
    {
      label: "GLM 5",
      value: "Pro/zai-org/GLM-5",
    },
    {
      label: "GLM 4.7",
      value: "Pro/zai-org/GLM-4.7",
    },
    {
      label: "Kimi K2.5",
      value: "Pro/moonshotai/Kimi-K2.5",
    },
  ],
  moonshot: [
    {
      label: "Kimi K2.5",
      value: "kimi-k2.5",
    },
  ],
};

export const IMAGE_MODELS_BY_PROVIDER: Record<
  string,
  { label: string; value: string }[]
> = {
  openai: [
    { label: "DALL-E 3", value: "dall-e-3" },
    { label: "DALL-E 2", value: "dall-e-2" },
  ],
  stability: [
    { label: "Stable Image Core", value: "core" },
    { label: "Stable Image Ultra", value: "ultra" },
    { label: "Stable Diffusion 3", value: "sd3" },
  ],
  siliconflow: [
    { label: "Kolors", value: "kolors" },
    { label: "Qwen Image", value: "qwen-image" },
    { label: "Qwen Image Edit", value: "qwen-image-edit" },
    { label: "Qwen Image Edit 2509", value: "qwen-image-edit-2509" },
  ],
};

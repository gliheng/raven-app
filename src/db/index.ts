import Database from '@tauri-apps/plugin-sql';
import { UIMessage } from 'ai';
import { ROOT_NODE_ID } from '@/constants';

const DB_PATH = 'sqlite:data.db';

let db: Database | null = null;

export { db };

export async function init() {
  db = await Database.load(DB_PATH);
}

export interface Chat {
  id: string;
  topic: string;
  createdAt: Date;
  updatedAt: Date;
  agentId?: string;
  ext?: {
    // AgentChat ext fields
    sessionId?: string;
    // SimpleChat ext fields
    model?: string;
    mcpServers?: string[];
    webSearch?: boolean;
    temperature?: number;
    contextSize?: number;
  };
}

export interface ChatMessage {
  chatId: string;
  id: string;
  data: UIMessage;
  parent?: string;
  children?: string[];
  siblingCount?: number;
  siblingIndex?: number;
}

export type AgentProgram = 'codex' | 'gemini' | 'claude' | 'qwen' | 'opencode';

export interface Agent {
  id: string;
  name: string;
  icon: string;
  createdAt: Date;
  updatedAt: Date;
  program: AgentProgram;
  directory: string;
}

export interface FileStore {
  file: File;
  createdAt: Date;
}

export interface Document {
  id: string;
  type: 'note' | 'chart';
  name: string;
  icon: string;
  content?: string;
  data?: string;
  createdAt: Date;
  updatedAt: Date;
}

export interface Note {
  id: string;
  name: string;
  icon: string;
  content?: string;
  createdAt: Date;
  updatedAt: Date;
}

export interface Chart {
  id: string;
  name: string;
  icon: string;
  data: string;
  createdAt: Date;
  updatedAt: Date;
}

export interface Image {
  id: string;
  fileId: number;
  provider: string;
  model: string;
  params: Record<string, any>;
  createdAt: Date;
  updatedAt: Date;
}

export interface ImageWithFile extends Image {
  fileUrl: string;
}

// Helper function to serialize/deserialize Date objects
function dateToString(date: Date): string {
  return date.toISOString();
}

function stringToDate(dateString: string): Date {
  return new Date(dateString);
}

// Chat operations
export async function writeChat(data: Chat): Promise<void> {
  if (!db) throw new Error('Database not initialized');
  await db.execute(
    `INSERT OR REPLACE INTO chat (id, topic, createdAt, updatedAt, agentId, ext)
     VALUES ($1, $2, $3, $4, $5, $6)`,
    [
      data.id,
      data.topic,
      dateToString(data.createdAt),
      dateToString(data.updatedAt),
      data.agentId ?? null,
      data.ext ? JSON.stringify(data.ext) : null,
    ]
  );
}

export async function getAllChats(): Promise<Chat[]> {
  if (!db) throw new Error('Database not initialized');
  const result = await db.select<Chat[]>(
    `SELECT * FROM chat ORDER BY updatedAt DESC`
  );
  return result.map((row: any) => ({
    ...row,
    createdAt: stringToDate(row.createdAt),
    updatedAt: stringToDate(row.updatedAt),
    ext: row.ext ? JSON.parse(row.ext) : undefined,
  }));
}

export async function getChat(chatId: string): Promise<Chat | undefined> {
  if (!db) throw new Error('Database not initialized');
  const result = await db.select<Chat[]>(
    `SELECT * FROM chat WHERE id = $1`,
    [chatId]
  );
  if (result.length === 0) return undefined;
  const row = result[0] as any;
  return {
    ...row,
    createdAt: stringToDate(row.createdAt),
    updatedAt: stringToDate(row.updatedAt),
    ext: row.ext ? JSON.parse(row.ext) : undefined,
  };
}

export async function deleteChat(chatId: string): Promise<void> {
  if (!db) throw new Error('Database not initialized');
  await db.execute('DELETE FROM message WHERE chatId = $1', [chatId]);
  await db.execute('DELETE FROM chat WHERE id = $1', [chatId]);
}

export async function updateChat(
  id: string,
  data: Partial<Chat>
): Promise<void> {
  if (!db) throw new Error('Database not initialized');

  const existing = await getChat(id);
  if (!existing) throw new Error(`Chat with ID ${id} not found`);

  const updated = { ...existing, ...data, updatedAt: new Date() };
  await writeChat(updated);
}

export async function updateChatExt(
  id: string,
  extFields: Partial<NonNullable<Chat['ext']>>
): Promise<void> {
  if (!db) throw new Error('Database not initialized');

  const existing = await getChat(id);
  if (!existing) throw new Error(`Chat with ID ${id} not found`);

  const updated = {
    ...existing,
    ext: { ...existing.ext, ...extFields },
    updatedAt: new Date(),
  };
  await writeChat(updated);
}

export async function searchChats(query: string): Promise<Chat[]> {
  if (!db) throw new Error('Database not initialized');
  const result = await db.select<Chat[]>(
    `SELECT * FROM chat WHERE LOWER(topic) LIKE LOWER($1) ORDER BY updatedAt DESC`,
    [`%${query}%`]
  );
  return result.map((row: any) => ({
    ...row,
    createdAt: stringToDate(row.createdAt),
    updatedAt: stringToDate(row.updatedAt),
    ext: row.ext ? JSON.parse(row.ext) : undefined,
  }));
}

export async function getChatsByAgentId(agentId: string): Promise<Chat[]> {
  if (!db) throw new Error('Database not initialized');
  const result = await db.select<Chat[]>(
    `SELECT * FROM chat WHERE agentId = $1 ORDER BY updatedAt DESC`,
    [agentId]
  );
  return result.map((row: any) => ({
    ...row,
    createdAt: stringToDate(row.createdAt),
    updatedAt: stringToDate(row.updatedAt),
    ext: row.ext ? JSON.parse(row.ext) : undefined,
  }));
}

// Message operations
export async function writeMessages(
  chatId: string,
  messages: UIMessage[],
  parent: string | undefined
): Promise<void> {
  if (!db) throw new Error('Database not initialized');
  
  let prev: UIMessage | undefined;
  for (const msg of messages) {
    const parentId = prev?.id ?? parent;
    const dataStr = JSON.stringify(msg);

    await db.execute(
      `INSERT OR IGNORE INTO message (chatId, messageId, parent, data)
       VALUES ($1, $2, $3, $4)`,
      [chatId, msg.id, parentId ?? null, dataStr]
    );

    prev = msg;
  }
}

export async function getChatMessages(chatId: string): Promise<ChatMessage[]> {
  if (!db) throw new Error('Database not initialized');
  const result = await db.select<
    { chatId: string; messageId: string; parent: string | null; data: string }[]
  >(`SELECT * FROM message WHERE chatId = $1`, [chatId]);

  const messages: ChatMessage[] = result.map((row) => ({
    chatId: row.chatId,
    id: row.messageId,
    parent: row.parent ?? undefined,
    data: JSON.parse(row.data) as UIMessage,
  }));

  return selectMessagesFromTree(messages);
}

export async function getMessages(
  chatId: string,
  pathSelection?: Record<string, number>
): Promise<ChatMessage[]> {
  if (!db) throw new Error('Database not initialized');
  const result = await db.select<
    { chatId: string; messageId: string; parent: string | null; data: string }[]
  >(`SELECT * FROM message WHERE chatId = $1`, [chatId]);

  const messages: ChatMessage[] = result.map((row) => ({
    chatId: row.chatId,
    id: row.messageId,
    parent: row.parent ?? undefined,
    data: JSON.parse(row.data) as UIMessage,
  }));

  return selectMessagesFromTree(messages, pathSelection);
}

export interface ChatData {
  chat: Chat;
  messages: ChatMessage[];
}

// Agent operations
export async function writeAgent(data: Agent): Promise<void> {
  if (!db) throw new Error('Database not initialized');
  await db.execute(
    `INSERT OR REPLACE INTO agent (id, name, icon, createdAt, updatedAt, program, directory)
     VALUES ($1, $2, $3, $4, $5, $6, $7)`,
    [
      data.id,
      data.name,
      data.icon,
      dateToString(data.createdAt),
      dateToString(data.updatedAt),
      data.program,
      data.directory,
    ]
  );
}

export async function updateAgent(
  id: string,
  data: Partial<Agent>
): Promise<void> {
  if (!db) throw new Error('Database not initialized');

  const existing = await getAgent(id);
  if (!existing) throw new Error(`Agent with ID ${id} not found`);

  const updated = { ...existing, ...data, updatedAt: new Date() };
  await writeAgent(updated);
}

export async function deleteAgent(id: string): Promise<void> {
  if (!db) throw new Error('Database not initialized');

  // First delete all chats for this agent
  const chats = await getChatsByAgentId(id);
  for (const chat of chats) {
    await deleteChat(chat.id);
  }

  // Then delete the agent
  await db.execute('DELETE FROM agent WHERE id = $1', [id]);
}

export async function getAgents(): Promise<Agent[]> {
  if (!db) throw new Error('Database not initialized');
  const result = await db.select<Agent[]>(`SELECT * FROM agent`);
  return result.map((row: any) => ({
    ...row,
    createdAt: stringToDate(row.createdAt),
    updatedAt: stringToDate(row.updatedAt),
  }));
}

export async function getAgent(id: string): Promise<Agent | null> {
  if (!db) throw new Error('Database not initialized');
  const result = await db.select<Agent[]>(
    `SELECT * FROM agent WHERE id = $1`,
    [id]
  );
  if (result.length === 0) return null;
  const row = result[0] as any;
  return {
    ...row,
    createdAt: stringToDate(row.createdAt),
    updatedAt: stringToDate(row.updatedAt),
  };
}

export async function writeDocument(data: Document): Promise<void> {
  if (!db) throw new Error('Database not initialized');
  await db.execute(
    `INSERT OR REPLACE INTO document (id, type, name, icon, content, data, createdAt, updatedAt)
     VALUES ($1, $2, $3, $4, $5, $6, $7, $8)`,
    [
      data.id,
      data.type,
      data.name,
      data.icon,
      data.content ?? null,
      data.data ?? null,
      dateToString(data.createdAt),
      dateToString(data.updatedAt),
    ]
  );
}

export async function updateDocument(id: string, data: Partial<Document>): Promise<void> {
  if (!db) throw new Error('Database not initialized');

  const existing = await getDocument(id);
  if (!existing) throw new Error(`Document with ID ${id} not found`);

  const updated = { ...existing, ...data };
  await writeDocument(updated);
}

export async function getDocuments(type?: 'note' | 'chart'): Promise<Document[]> {
  if (!db) throw new Error('Database not initialized');
  
  let query = `SELECT * FROM document`;
  const params: any[] = [];
  
  if (type) {
    query += ` WHERE type = $1`;
    params.push(type);
  }
  
  query += ` ORDER BY updatedAt DESC`;
  
  const result = await db.select<Document[]>(query, params);
  return result.map((row: any) => ({
    ...row,
    createdAt: stringToDate(row.createdAt),
    updatedAt: stringToDate(row.updatedAt),
  }));
}

export async function getDocument(id: string): Promise<Document | undefined> {
  if (!db) throw new Error('Database not initialized');
  const result = await db.select<Document[]>(
    `SELECT * FROM document WHERE id = $1`,
    [id]
  );
  if (result.length === 0) return undefined;
  const row = result[0] as any;
  return {
    ...row,
    createdAt: stringToDate(row.createdAt),
    updatedAt: stringToDate(row.updatedAt),
  };
}

export async function deleteDocument(id: string): Promise<void> {
  if (!db) throw new Error('Database not initialized');
  await db.execute('DELETE FROM document WHERE id = $1', [id]);
}

// Image operations
export async function writeImage(data: Image): Promise<void> {
  if (!db) throw new Error('Database not initialized');
  await db.execute(
    `INSERT OR REPLACE INTO image (id, fileId, provider, model, params, createdAt, updatedAt)
     VALUES ($1, $2, $3, $4, $5, $6, $7)`,
    [
      data.id,
      data.fileId,
      data.provider,
      data.model,
      JSON.stringify(data.params),
      dateToString(data.createdAt),
      dateToString(data.updatedAt),
    ]
  );
}

export async function getImages(): Promise<ImageWithFile[]> {
  if (!db) throw new Error('Database not initialized');

  const result = await db.select<
    {
      id: string;
      fileId: number;
      provider: string;
      model: string;
      params: string;
      createdAt: string;
      updatedAt: string;
      fileName: string;
      fileType: string;
      fileData: string | number[];
    }[]
  >(
    `SELECT i.*, f.fileName, f.fileType, f.fileData
     FROM image i
     JOIN file_store f ON i.fileId = f.id
     ORDER BY i.updatedAt DESC`
  );

  const images: ImageWithFile[] = [];

  for (const row of result) {
    const fileDataArray = typeof row.fileData === 'string' 
      ? JSON.parse(row.fileData)
      : row.fileData;

    const uint8Array = new Uint8Array(fileDataArray);
    const blob = new Blob([uint8Array], { type: row.fileType });
    const fileUrl = URL.createObjectURL(blob);

    const params = row.params ? JSON.parse(row.params) : {};

    images.push({
      id: row.id,
      fileId: row.fileId,
      provider: row.provider,
      model: row.model,
      params,
      createdAt: stringToDate(row.createdAt),
      updatedAt: stringToDate(row.updatedAt),
      fileUrl,
    });
  }

  return images;
}

export async function getImage(id: string): Promise<ImageWithFile | undefined> {
  if (!db) throw new Error('Database not initialized');

  const result = await db.select<
    {
      id: string;
      fileId: number;
      provider: string;
      model: string;
      params: string;
      createdAt: string;
      updatedAt: string;
      fileName: string;
      fileType: string;
      fileData: string | number[];
    }[]
  >(
    `SELECT i.*, f.fileName, f.fileType, f.fileData
     FROM image i
     JOIN file_store f ON i.fileId = f.id
     WHERE i.id = $1`,
    [id]
  );

  if (result.length === 0) return undefined;

  const row = result[0];
  const fileDataArray = typeof row.fileData === 'string' 
    ? JSON.parse(row.fileData)
    : row.fileData;

  const uint8Array = new Uint8Array(fileDataArray);
  const blob = new Blob([uint8Array], { type: row.fileType });
  const fileUrl = URL.createObjectURL(blob);

  const params = row.params ? JSON.parse(row.params) : {};

  return {
    id: row.id,
    fileId: row.fileId,
    provider: row.provider,
    model: row.model,
    params,
    createdAt: stringToDate(row.createdAt),
    updatedAt: stringToDate(row.updatedAt),
    fileUrl,
  };
}

export async function deleteImage(id: string): Promise<void> {
  if (!db) throw new Error('Database not initialized');

  // First get the image to get the fileId for file cleanup
  const result = await db.select<
    { fileId: number }[]
  >(`SELECT fileId FROM image WHERE id = $1`, [id]);

  if (result.length === 0) return;

  // Delete from file_store (CASCADE will handle this via foreign key)
  await db.execute('DELETE FROM image WHERE id = $1', [id]);
}

// File operations
export async function writeFile(file: File): Promise<number> {
  if (!db) throw new Error('Database not initialized');

  // Convert File to Uint8Array for BLOB storage
  const arrayBuffer = await file.arrayBuffer();
  const uint8Array = new Uint8Array(arrayBuffer);

  const result = await db.execute(
    `INSERT INTO file_store (fileName, fileType, fileData, createdAt) VALUES ($1, $2, $3, $4)`,
    [file.name, file.type, uint8Array, dateToString(new Date())]
  );

  return result.lastInsertId as number;
}

export async function readFile(id: number): Promise<FileStore | undefined> {
  if (!db) throw new Error('Database not initialized');
  const result = await db.select<
    { fileName: string; fileType: string; fileData: string | number[]; createdAt: string }[]
  >(`SELECT * FROM file_store WHERE id = $1`, [id]);

  if (result.length === 0) return undefined;

  const row = result[0];

  // Handle both stringified array and actual array formats from Tauri's serialization
  const fileDataArray = typeof row.fileData === 'string'
    ? JSON.parse(row.fileData)
    : row.fileData;

  const uint8Array = new Uint8Array(fileDataArray);
  const file = new File([uint8Array], row.fileName, { type: row.fileType });

  return {
    file,
    createdAt: stringToDate(row.createdAt),
  };
}

// Export/Import functionality
interface ExportData {
  chats: Chat[];
  messages: ChatMessage[];
  version: string;
  exportDate: string;
}

export async function exportData(): Promise<ExportData> {
  if (!db) throw new Error('Database not initialized');
  const chats = await getAllChats();
  const result = await db.select<
    { chatId: string; messageId: string; parent: string | null; data: string }[]
  >(`SELECT * FROM message`);

  const messages: ChatMessage[] = result.map((row) => ({
    chatId: row.chatId,
    id: row.messageId,
    parent: row.parent ?? undefined,
    data: JSON.parse(row.data) as UIMessage,
  }));

  return {
    chats,
    messages,
    version: '1.0',
    exportDate: new Date().toISOString(),
  };
}

export async function importData(data: ExportData): Promise<void> {
  if (!db) throw new Error('Database not initialized');
  if (!data.chats || !data.messages || !data.version) {
    throw new Error('Invalid import data format');
  }

  await clearDatabase();

  for (const chat of data.chats) {
    await writeChat(chat);
  }

  for (const message of data.messages) {
    const msgData = JSON.stringify(message.data);
    await db.execute(
      `INSERT INTO message (chatId, messageId, parent, data) VALUES ($1, $2, $3, $4)`,
      [message.chatId, message.id, message.parent ?? null, msgData]
    );
  }
}

export async function clearDatabase(): Promise<void> {
  if (!db) throw new Error('Database not initialized');
  await db.execute('DELETE FROM message');
  await db.execute('DELETE FROM chat');
}

// Helper function to build message tree
function selectMessagesFromTree(
  messages: ChatMessage[],
  pathSelection?: Record<string, number>
): ChatMessage[] {
  const msgMap: Record<string, ChatMessage> = {};
  const tree: Record<string, string[]> = {};

  for (const msg of messages) {
    msgMap[msg.id] = msg;
    const parent = msg.parent ?? ROOT_NODE_ID;
    if (!tree[parent]) {
      tree[parent] = [];
    }
    tree[parent].push(msg.id);
  }

  for (const msg of messages) {
    const children = tree[msg.id];
    if (children) {
      msg.children = children;
    }
  }

  const list: ChatMessage[] = [];
  let nodeId = ROOT_NODE_ID;
  let children;

  while ((children = tree[nodeId])) {
    const i = pathSelection?.[nodeId] ?? children.length - 1;
    nodeId = children[i];
    const node = msgMap[nodeId];
    node.siblingIndex = i;
    node.siblingCount = children.length;
    list.push(node);
  }

  return list;
}

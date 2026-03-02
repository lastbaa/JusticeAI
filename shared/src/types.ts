// ─── Document Types ────────────────────────────────────────────────────────────

export interface DocumentPage {
  pageNumber: number;
  text: string;
}

export interface ParsedDocument {
  id: string;
  filePath: string;
  fileName: string;
  extension: 'pdf' | 'docx';
  pages: DocumentPage[];
  totalPages: number;
  wordCount: number;
  loadedAt: number;
}

// ─── RAG / Chunk Types ─────────────────────────────────────────────────────────

export interface DocumentChunk {
  id: string;
  documentId: string;
  fileName: string;
  filePath: string;
  pageNumber: number;
  chunkIndex: number;
  text: string;
  tokenCount: number;
}

export interface EmbeddedChunk extends DocumentChunk {
  embedding: number[];
}

export interface RetrievedChunk extends DocumentChunk {
  score: number;
  excerpt: string;
}

// ─── Chat / Message Types ──────────────────────────────────────────────────────

export interface Citation {
  fileName: string;
  filePath: string;
  pageNumber: number;
  excerpt: string;
  score: number;
}

export type MessageRole = 'user' | 'assistant' | 'system';

export interface ChatMessage {
  id: string;
  role: MessageRole;
  content: string;
  citations?: Citation[];
  isStreaming?: boolean;
  timestamp: number;
  notFound?: boolean;
}

// ─── Ollama / Model Types ──────────────────────────────────────────────────────

export interface OllamaModel {
  name: string;
  size: number;
  digest: string;
}

export interface OllamaStatus {
  running: boolean;
  models: OllamaModel[];
  hasLlmModel: boolean;
  hasEmbedModel: boolean;
  llmModelName: string;
  embedModelName: string;
}

// ─── Settings Types ────────────────────────────────────────────────────────────

export interface AppSettings {
  hfToken: string;
  chunkSize: number;
  chunkOverlap: number;
  topK: number;
}

export const DEFAULT_SETTINGS: AppSettings = {
  hfToken: '',
  chunkSize: 1000,
  chunkOverlap: 150,
  topK: 6,
};

// ─── IPC Channel Names ─────────────────────────────────────────────────────────

export const IPC = {
  CHECK_OLLAMA: 'check-ollama',
  LOAD_FILES: 'load-files',
  LOAD_FOLDER: 'load-folder',
  REMOVE_FILE: 'remove-file',
  GET_FILES: 'get-files',
  QUERY: 'query',
  QUERY_STREAM: 'query-stream',
  GET_SETTINGS: 'get-settings',
  SAVE_SETTINGS: 'save-settings',
  OPEN_FILE_DIALOG: 'open-file-dialog',
  OPEN_FOLDER_DIALOG: 'open-folder-dialog',
  // Chat history (encrypted)
  SAVE_SESSION: 'save-session',
  GET_SESSIONS: 'get-sessions',
  DELETE_SESSION: 'delete-session',
  // Document viewer
  GET_FILE_DATA: 'get-file-data',
  GET_PAGE_TEXT: 'get-page-text',
} as const

// ─── Chat Session Types ────────────────────────────────────────────────────────

export interface ChatSession {
  id: string
  name: string
  messages: ChatMessage[]
  createdAt: number
  updatedAt: number
};

// ─── IPC Payload Types ─────────────────────────────────────────────────────────

export interface LoadFilesPayload {
  filePaths: string[];
}

export interface QueryPayload {
  question: string;
  settings: AppSettings;
}

export interface QueryResult {
  answer: string;
  citations: Citation[];
  notFound: boolean;
}

export interface FileInfo {
  id: string;
  fileName: string;
  filePath: string;
  totalPages: number;
  wordCount: number;
  loadedAt: number;
  chunkCount: number;
}

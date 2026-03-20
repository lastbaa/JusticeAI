// ─── Case Types ───────────────────────────────────────────────────────────────

export interface Case {
  id: string
  name: string
  description?: string
  createdAt: number
  updatedAt: number
}

// ─── Document Types ────────────────────────────────────────────────────────────

export interface DocumentPage {
  pageNumber: number;
  text: string;
}

export interface ParsedDocument {
  id: string;
  filePath: string;
  fileName: string;
  extension: 'pdf' | 'docx' | 'txt' | 'md' | 'csv' | 'eml' | 'html' | 'htm' | 'mhtml' | 'xml' | 'xlsx' | 'png' | 'jpg' | 'jpeg' | 'tif' | 'tiff';
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

export type Theme = 'dark' | 'light';

export interface AppSettings {
  chunkSize: number;
  chunkOverlap: number;
  topK: number;
  theme: Theme;
}

export const DEFAULT_SETTINGS: AppSettings = {
  chunkSize: 1000,
  chunkOverlap: 150,
  topK: 6,
  theme: 'dark',
};

// ─── Model Status Types ────────────────────────────────────────────────────────

export interface ModelStatus {
  llmReady: boolean;
  llmSizeGb: number;
  downloadRequiredGb: number;
  ocrReady: boolean;
  ocrMessage?: string;
}

export interface OcrRuntimeStatus {
  ready: boolean;
  installAttempted: boolean;
  message: string;
}

export interface DownloadProgress {
  percent: number;
  downloadedBytes: number;
  totalBytes: number;
  done: boolean;
}

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
  // Case management
  GET_CASES: 'get-cases',
  SAVE_CASE: 'save-case',
  DELETE_CASE: 'delete-case',
  ASSIGN_SESSION_TO_CASE: 'assign-session-to-case',
  ASSIGN_FILE_TO_CASE: 'assign-file-to-case',
  GET_CASE_SUMMARIES: 'get-case-summaries',
} as const

// ─── Chat Session Types ────────────────────────────────────────────────────────

export interface ChatSession {
  id: string
  name: string
  messages: ChatMessage[]
  createdAt: number
  updatedAt: number
  caseId?: string
  summary?: string
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
  caseId?: string;
}

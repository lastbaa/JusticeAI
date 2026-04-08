// Shared types — mirror of app/src-tauri/src/state.rs
// Keep in sync with the Rust structs.

export type InferenceMode = 'quick' | 'balanced' | 'extended'

export type JurisdictionLevel = 'federal' | 'state' | 'county'

export interface Jurisdiction {
  level: JurisdictionLevel
  state?: string
  county?: string
}

export interface Case {
  id: string
  name: string
  description?: string
  createdAt: number
  updatedAt: number
  jurisdiction?: Jurisdiction
}

export interface DocumentPage {
  pageNumber: number
  text: string
}

export interface FileInfo {
  id: string
  fileName: string
  filePath: string
  totalPages: number
  wordCount: number
  loadedAt: number
  chunkCount: number
  caseId?: string
  detectedJurisdiction?: Jurisdiction
}

export interface Citation {
  fileName: string
  filePath: string
  pageNumber: number
  excerpt: string
  summary: string
  score: number
}

export type AssertionType =
  | 'citation_format'
  | 'citation_filename'
  | 'number_exactness'
  | 'blocklist'
  | 'hallucination'
  | 'fabricated_entity'

export interface AssertionResult {
  passed: boolean
  assertionType: AssertionType
  message: string
}

export interface QueryResult {
  answer: string
  citations: Citation[]
  notFound: boolean
  assertions?: AssertionResult[]
  confidence?: number
}

export interface QueryPayload {
  question: string
  settings: AppSettings
}

export interface AppSettings {
  chunkSize: number
  chunkOverlap: number
  topK: number
  theme: string
  jurisdiction?: Jurisdiction
  inferenceMode: InferenceMode
}

export interface ModelStatus {
  llmReady: boolean
  llmSizeGb: number
  downloadRequiredGb: number
  ocrReady: boolean
  ocrMessage?: string
}

export interface ChatMessage {
  id: string
  role: string
  content: string
  citations?: Citation[]
  isStreaming?: boolean
  timestamp: number
  notFound?: boolean
  qualityAssertions?: unknown
  inferenceMode?: string
  isGreeting?: boolean
  confidence?: number
}

export interface ChatSession {
  id: string
  name: string
  messages: ChatMessage[]
  createdAt: number
  updatedAt: number
  caseId?: string
  summary?: string
}

export interface OllamaModel {
  name: string
  size: number
  digest: string
}

export interface OllamaStatus {
  running: boolean
  models: OllamaModel[]
  hasLlmModel: boolean
  hasEmbedModel: boolean
  llmModelName: string
  embedModelName: string
}

export interface DownloadProgress {
  percent: number
  downloadedBytes: number
  totalBytes: number
  done: boolean
  retrying?: boolean
  attempt?: number
}

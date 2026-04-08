/**
 * Tauri API shim — wraps invoke() to expose the same window.api interface
 * that the React components already use. No component changes needed.
 */
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import type {
  AppSettings,
  Case,
  ChatSession,
  DocumentRole,
  DownloadProgress,
  EntityEntry,
  FileInfo,
  Jurisdiction,
  ModelStatus,
  OllamaStatus,
  QueryResult,
} from '../../../../shared/src/types'

export const api = {
  checkOllama: (): Promise<OllamaStatus> => invoke('check_ollama'),

  checkModels: (): Promise<ModelStatus> => invoke('check_models'),
  downloadModels: (): Promise<void> => invoke('download_models'),

  onDownloadProgress: (cb: (p: DownloadProgress) => void): Promise<() => void> =>
    listen('download-progress', (e) => cb(e.payload as DownloadProgress)),

  openFileDialog: async (): Promise<string[]> => {
    const result = await open({
      multiple: true,
      filters: [{
        name: 'Supported Files',
        extensions: ['pdf', 'docx', 'txt', 'md', 'csv', 'eml', 'html', 'htm', 'mhtml', 'xml', 'xlsx', 'png', 'jpg', 'jpeg', 'tif', 'tiff'],
      }],
    })
    if (!result) return []
    return Array.isArray(result) ? result : [result]
  },

  openFolderDialog: async (): Promise<string | null> => {
    const result = await open({ directory: true })
    if (!result) return null
    return Array.isArray(result) ? result[0] : result
  },

  loadFiles: (filePaths: string[], caseId?: string): Promise<FileInfo[]> =>
    invoke('load_files', { filePaths, caseId: caseId ?? null }),

  getFiles: (): Promise<FileInfo[]> => invoke('get_files'),

  removeFile: (fileId: string): Promise<void> => invoke('remove_file', { fileId }),

  query: (question: string, history: [string, string][], caseId?: string, caseContext?: string): Promise<QueryResult> =>
    invoke('query', { question, history, caseId: caseId ?? null, caseContext: caseContext ?? null }),

  onQueryToken: (cb: (token: string) => void): Promise<() => void> =>
    listen('query-token', (e) => cb(e.payload as string)),

  onQueryStatus: (cb: (status: { phase: string; chunks?: number }) => void): Promise<() => void> =>
    listen('query-status', (e) => cb(e.payload as { phase: string; chunks?: number })),

  getSettings: (): Promise<AppSettings> => invoke('get_settings'),

  saveSettings: (settings: AppSettings): Promise<void> =>
    invoke('save_settings', { settings }),

  saveSession: (session: ChatSession): Promise<boolean> =>
    invoke('save_session', { session }),

  getSessions: (): Promise<ChatSession[]> => invoke('get_sessions'),

  deleteSession: (sessionId: string): Promise<boolean> =>
    invoke('delete_session', { sessionId }),

  getFileData: (filePath: string): Promise<string> =>
    invoke('get_file_data', { filePath }),

  getPageText: (filePath: string, pageNumber: number): Promise<string> =>
    invoke('get_page_text', { filePath, pageNumber }),

  getFileServerPort: (): Promise<number> => invoke('get_file_server_port'),

  saveFile: (filePath: string, content: string): Promise<void> =>
    invoke('save_file', { filePath, content }),

  getBuildInfo: (): Promise<string> => invoke('get_build_info'),

  // Case management
  getCases: (): Promise<Case[]> => invoke('get_cases'),

  saveCase: (c: Case): Promise<void> => invoke('save_case', { case: c }),

  deleteCase: (caseId: string, deleteContents: boolean): Promise<void> =>
    invoke('delete_case', { caseId, deleteContents }),

  assignFileToCase: (fileId: string, caseId: string | null): Promise<void> =>
    invoke('assign_file_to_case', { fileId, caseId }),

  assignSessionToCase: (sessionId: string, caseId: string | null): Promise<void> =>
    invoke('assign_session_to_case', { sessionId, caseId }),

  setCaseJurisdiction: (caseId: string, jurisdiction: Jurisdiction | null): Promise<void> =>
    invoke('set_case_jurisdiction', { caseId, jurisdiction }),

  getCaseSummaries: (caseId: string, excludeSessionId?: string): Promise<{ sessionId: string; summary: string }[]> =>
    invoke('get_case_summaries', { caseId, excludeSessionId: excludeSessionId ?? null }),

  // Document roles
  setDocumentRole: (fileId: string, role: DocumentRole): Promise<void> =>
    invoke('set_document_role', { fileId, role }),

  // Entity registry
  getEntities: (caseId?: string): Promise<EntityEntry[]> =>
    invoke('get_entities', { caseId: caseId ?? null }),

  // Case context
  saveCaseContext: (caseId: string, context: string): Promise<void> =>
    invoke('save_case_context', { caseId, context }),
}

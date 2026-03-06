/**
 * Tauri API shim — wraps invoke() to expose the same window.api interface
 * that the React components already use. No component changes needed.
 */
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import type {
  AppSettings,
  ChatSession,
  DownloadProgress,
  FileInfo,
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
      filters: [{ name: 'Documents', extensions: ['pdf', 'docx'] }],
    })
    if (!result) return []
    return Array.isArray(result) ? result : [result]
  },

  openFolderDialog: async (): Promise<string | null> => {
    const result = await open({ directory: true })
    if (!result) return null
    return Array.isArray(result) ? result[0] : result
  },

  loadFiles: (filePaths: string[]): Promise<FileInfo[]> =>
    invoke('load_files', { filePaths }),

  getFiles: (): Promise<FileInfo[]> => invoke('get_files'),

  removeFile: (fileId: string): Promise<void> => invoke('remove_file', { fileId }),

  query: (question: string): Promise<QueryResult> => invoke('query', { question }),

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
}

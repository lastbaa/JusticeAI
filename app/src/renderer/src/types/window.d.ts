import {
  AppSettings,
  DownloadProgress,
  FileInfo,
  ModelStatus,
  OcrRuntimeStatus,
  OllamaStatus,
  QueryResult,
  ChatSession,
} from '../../../../../shared/src/types'

export {}

declare global {
  interface Window {
    api: {
      checkOllama: () => Promise<OllamaStatus>
      checkModels: () => Promise<ModelStatus>
      ensureOcrRuntime: () => Promise<OcrRuntimeStatus>
      downloadModels: () => Promise<void>
      onDownloadProgress: (cb: (p: DownloadProgress) => void) => Promise<() => void>
      openFileDialog: () => Promise<string[]>
      openFolderDialog: () => Promise<string | null>
      loadFiles: (filePaths: string[]) => Promise<FileInfo[]>
      getFiles: () => Promise<FileInfo[]>
      removeFile: (fileId: string) => Promise<void>
      query: (question: string) => Promise<QueryResult>
      getSettings: () => Promise<AppSettings>
      saveSettings: (settings: AppSettings) => Promise<void>
      saveSession: (session: ChatSession) => Promise<boolean>
      getSessions: () => Promise<ChatSession[]>
      deleteSession: (sessionId: string) => Promise<boolean>
      getFileData: (filePath: string) => Promise<string>
      getPageText: (filePath: string, pageNumber: number) => Promise<string>
    }
  }
}

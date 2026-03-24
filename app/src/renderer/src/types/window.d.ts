import {
  AppSettings,
  Case,
  ChatSession,
  DownloadProgress,
  FileInfo,
  Jurisdiction,
  ModelStatus,
  OllamaStatus,
  QueryResult,
} from '../../../../../shared/src/types'

export {}

declare global {
  interface Window {
    api: {
      checkOllama: () => Promise<OllamaStatus>
      checkModels: () => Promise<ModelStatus>
      downloadModels: () => Promise<void>
      onDownloadProgress: (cb: (p: DownloadProgress) => void) => Promise<() => void>
      openFileDialog: () => Promise<string[]>
      openFolderDialog: () => Promise<string | null>
      loadFiles: (filePaths: string[], caseId?: string) => Promise<FileInfo[]>
      getFiles: () => Promise<FileInfo[]>
      removeFile: (fileId: string) => Promise<void>
      query: (question: string, history: [string, string][], caseId?: string, caseContext?: string) => Promise<QueryResult>
      onQueryToken: (cb: (token: string) => void) => Promise<() => void>
      onQueryStatus: (cb: (status: { phase: string; chunks?: number }) => void) => Promise<() => void>
      getSettings: () => Promise<AppSettings>
      saveSettings: (settings: AppSettings) => Promise<void>
      saveSession: (session: ChatSession) => Promise<boolean>
      getSessions: () => Promise<ChatSession[]>
      deleteSession: (sessionId: string) => Promise<boolean>
      getFileData: (filePath: string) => Promise<string>
      getPageText: (filePath: string, pageNumber: number) => Promise<string>
      getFileServerPort: () => Promise<number>
      saveFile: (filePath: string, content: string) => Promise<void>
      getBuildInfo: () => Promise<string>
      getCases: () => Promise<Case[]>
      saveCase: (c: Case) => Promise<void>
      deleteCase: (caseId: string, deleteContents: boolean) => Promise<void>
      assignFileToCase: (fileId: string, caseId: string | null) => Promise<void>
      assignSessionToCase: (sessionId: string, caseId: string | null) => Promise<void>
      setCaseJurisdiction: (caseId: string, jurisdiction: Jurisdiction | null) => Promise<void>
      getCaseSummaries: (caseId: string, excludeSessionId?: string) => Promise<{ sessionId: string; summary: string }[]>
    }
  }
}

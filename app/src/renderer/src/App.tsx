import { useEffect, useRef, useState } from 'react'
import {
  AppSettings,
  ChatMessage,
  ChatSession,
  DEFAULT_SETTINGS,
  FileInfo,
} from '../../../../shared/src/types'
import { v4 as uuidv4 } from 'uuid'
import Sidebar from './components/Sidebar'
import ContextPanel from './components/ContextPanel'
import ChatInterface from './components/ChatInterface'
import Settings from './components/Settings'
import DocumentViewer from './components/DocumentViewer'
import ModelSetup from './components/ModelSetup'

type View = 'main' | 'settings'

function makeSessionName(messages: ChatMessage[]): string {
  const first = messages.find((m) => m.role === 'user')
  if (!first) return 'New Chat'
  const text = first.content.trim()
  return text.length > 52 ? text.slice(0, 52) + '…' : text
}

export default function App(): JSX.Element {
  const [view, setView] = useState<View>('main')
  const [files, setFiles] = useState<FileInfo[]>([])
  const [messages, setMessages] = useState<ChatMessage[]>([])
  const [sessions, setSessions] = useState<ChatSession[]>([])
  const [currentSessionId, setCurrentSessionId] = useState<string>(() => uuidv4())
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS)
  const [showModelSetup, setShowModelSetup] = useState(false)
  const [chatMode, setChatMode] = useState(false)
  const [sessionCreatedAt, setSessionCreatedAt] = useState<number>(() => Date.now())
  const [isLoading, setIsLoading] = useState(false)
  const [isQuerying, setIsQuerying] = useState(false)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [lastCitations, setLastCitations] = useState<import('../../../../shared/src/types').Citation[]>([])
  const [viewerCitation, setViewerCitation] = useState<import('../../../../shared/src/types').Citation | null>(null)

  const messagesRef = useRef(messages)
  const sessionIdRef = useRef(currentSessionId)
  const sessionCreatedAtRef = useRef(sessionCreatedAt)
  messagesRef.current = messages
  sessionIdRef.current = currentSessionId
  sessionCreatedAtRef.current = sessionCreatedAt

  useEffect(() => {
    async function init(): Promise<void> {
      try {
        const savedSettings = await window.api.getSettings()
        setSettings(savedSettings)
      } catch { }
      try {
        const existingFiles = await window.api.getFiles()
        setFiles(existingFiles)
      } catch { }
      try {
        const saved = await window.api.getSessions()
        setSessions(saved)
      } catch { }
      try {
        const modelStatus = await window.api.checkModels()
        if (!modelStatus.llmReady) setShowModelSetup(true)
      } catch { }
    }
    init()
  }, [])

  // Auto-save current session (debounced 1s)
  useEffect(() => {
    if (messages.length === 0) return
    const timer = setTimeout(async () => {
      const session: ChatSession = {
        id: sessionIdRef.current,
        name: makeSessionName(messagesRef.current),
        messages: messagesRef.current,
        createdAt: sessionCreatedAtRef.current,
        updatedAt: Date.now(),
      }
      try {
        await window.api.saveSession(session)
        const updated = await window.api.getSessions()
        setSessions(updated)
      } catch { }
    }, 1000)
    return () => clearTimeout(timer)
  }, [messages])

  // ── File management ───────────────────────────────────────────
  async function handleLoadPaths(paths: string[]): Promise<void> {
    setLoadError(null)
    setIsLoading(true)
    try {
      const loaded = await window.api.loadFiles(paths)
      if (loaded.length === 0) {
        setLoadError('No supported files found. Try PDF or DOCX files.')
        return
      }
      setFiles((prev) => {
        const existingIds = new Set(prev.map((f) => f.id))
        return [...prev, ...loaded.filter((f) => !existingIds.has(f.id))]
      })
    } catch (err) {
      setLoadError(err instanceof Error ? err.message : 'Failed to load files.')
    } finally {
      setIsLoading(false)
    }
  }

  async function handleAddFiles(): Promise<void> {
    try {
      const paths = await window.api.openFileDialog()
      if (!paths || paths.length === 0) return
      await handleLoadPaths(paths)
    } catch { }
  }

  async function handleAddFolder(): Promise<void> {
    try {
      const folderPath = await window.api.openFolderDialog()
      if (!folderPath) return
      await handleLoadPaths([folderPath])
    } catch { }
  }

  async function handleRemoveFile(id: string): Promise<void> {
    try {
      await window.api.removeFile(id)
      setFiles((prev) => prev.filter((f) => f.id !== id))
    } catch (err) {
      console.error('Failed to remove file:', err)
    }
  }

  // ── Chat ──────────────────────────────────────────────────────
  async function handleQuery(question: string): Promise<void> {
    const userMessage: ChatMessage = {
      id: uuidv4(),
      role: 'user',
      content: question,
      timestamp: Date.now(),
    }
    setMessages((prev) => [...prev, userMessage])
    setIsQuerying(true)

    try {
      const result = await window.api.query(question)
      const assistantMessage: ChatMessage = {
        id: uuidv4(),
        role: 'assistant',
        content: result.answer,
        citations: result.citations,
        notFound: result.notFound,
        timestamp: Date.now(),
      }
      setMessages((prev) => [...prev, assistantMessage])
      setLastCitations(result.citations)
    } catch (err) {
      const errorMessage: ChatMessage = {
        id: uuidv4(),
        role: 'assistant',
        content: `Unable to get a response. ${err instanceof Error ? err.message : 'Please try again.'}`,
        citations: [],
        notFound: true,
        timestamp: Date.now(),
      }
      setMessages((prev) => [...prev, errorMessage])
    } finally {
      setIsQuerying(false)
    }
  }

  // ── Navigation ────────────────────────────────────────────────
  function handleGoHome(): void {
    setChatMode(false)
    setMessages([])
    setCurrentSessionId(uuidv4())
    setSessionCreatedAt(Date.now())
    setLastCitations([])
    setViewerCitation(null)
    setView('main')
  }

  // ── Sessions ──────────────────────────────────────────────────
  function handleNewChat(): void {
    const newId = uuidv4()
    setMessages([])
    setCurrentSessionId(newId)
    setSessionCreatedAt(Date.now())
    setChatMode(true)
    setLastCitations([])
    setViewerCitation(null)
    setView('main')
  }

  async function handleLoadSession(session: ChatSession): Promise<void> {
    setMessages(session.messages)
    setCurrentSessionId(session.id)
    setSessionCreatedAt(session.createdAt)
    setChatMode(true)
    setLastCitations([])
    setViewerCitation(null)
    setView('main')
  }

  async function handleDeleteSession(sessionId: string): Promise<void> {
    try {
      await window.api.deleteSession(sessionId)
      setSessions((prev) => prev.filter((s) => s.id !== sessionId))
      if (sessionId === currentSessionId) handleNewChat()
    } catch (err) {
      console.error('Failed to delete session:', err)
    }
  }

  async function handleSaveSettings(newSettings: AppSettings): Promise<void> {
    try {
      await window.api.saveSettings(newSettings)
      setSettings(newSettings)
      setView('main')
    } catch (err) {
      console.error('Failed to save settings:', err)
    }
  }

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-[#080808]">
      <Sidebar
        sessions={sessions}
        currentSessionId={currentSessionId}
        isLoading={isLoading}
        onGoHome={handleGoHome}
        onNewChat={handleNewChat}
        onLoadSession={handleLoadSession}
        onDeleteSession={handleDeleteSession}
        onAddFiles={handleAddFiles}
        onOpenSettings={() => setView('settings')}
      />

      <ContextPanel
        files={files}
        citations={lastCitations}
        isQuerying={isQuerying}
        isLoading={isLoading}
        collapsed={viewerCitation !== null}
        onAddFiles={handleAddFiles}
        onRemoveFile={handleRemoveFile}
        onViewCitation={setViewerCitation}
      />

      <div className="flex flex-1 flex-col overflow-hidden">
        <ChatInterface
          messages={messages}
          files={files}
          isQuerying={isQuerying}
          isLoading={isLoading}
          loadError={loadError}
          chatMode={chatMode}
          onQuery={handleQuery}
          onNewChat={handleNewChat}
          onAddFiles={handleAddFiles}
          onAddFolder={handleAddFolder}
          onRemoveFile={handleRemoveFile}
          onLoadPaths={handleLoadPaths}
          onViewCitation={setViewerCitation}
        />
      </div>

      <DocumentViewer
        citation={viewerCitation}
        onClose={() => setViewerCitation(null)}
      />

      {showModelSetup && (
        <ModelSetup onComplete={() => setShowModelSetup(false)} />
      )}

      {view === 'settings' && (
        <Settings
          settings={settings}
          onSave={handleSaveSettings}
          onClose={() => setView('main')}
        />
      )}
    </div>
  )
}

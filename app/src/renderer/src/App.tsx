import { useEffect, useRef, useState } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { confirm } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'
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

const STOP_WORDS = new Set([
  'a','an','the','is','are','was','were','be','been','being',
  'have','has','had','do','does','did','will','would','could',
  'should','may','might','shall','can','need','ought',
  'i','me','my','we','our','you','your','he','she','it','they',
  'what','which','who','whom','this','that','these','those',
  'of','in','on','at','by','for','with','about','as','into',
  'through','before','after','to','from','up','and','but','or',
  'nor','so','yet','not','only','same','than','too','very','just',
  'how','when','where','why','there','here','out','any','all',
  'more','most','some','such','no','each','few','once','under',
  'between','tell','explain','describe','give','provide','find',
  'show','please','them','their','its','also','am','if','than',
])

function makeSessionName(messages: ChatMessage[]): string {
  const first = messages.find((m) => m.role === 'user')
  if (!first) return 'New Chat'

  const words = first.content
    .trim()
    .replace(/[^a-zA-Z0-9\s'-]/g, ' ')
    .split(/\s+/)
    .filter((w) => w.length > 2 && !STOP_WORDS.has(w.toLowerCase()))

  if (words.length === 0) {
    const text = first.content.trim()
    return text.length > 40 ? text.slice(0, 40) + '…' : text
  }

  const name = words.slice(0, 4).join(' ')
  return name.charAt(0).toUpperCase() + name.slice(1)
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

  // Track busy state in a ref so the close listener always reads the current value
  const isBusyRef = useRef(false)
  useEffect(() => {
    isBusyRef.current = isQuerying || showModelSetup
  }, [isQuerying, showModelSetup])

  // Close protection — intercepted from Rust; confirm if a query is in progress.
  // Uses a two-phase handshake: JS calls `set_can_close` (flips a Rust AtomicBool)
  // then appWindow.close(), so the second CloseRequested event passes through.
  useEffect(() => {
    const appWindow = getCurrentWindow()
    let unlisten: (() => void) | undefined

    async function doClose(): Promise<void> {
      await invoke('set_can_close')
      await appWindow.close()
    }

    appWindow
      .listen('app-close-requested', async () => {
        try {
          if (isBusyRef.current) {
            const ok = await confirm('Justice AI is busy. Quit anyway?', {
              title: 'Justice AI',
              kind: 'warning',
            })
            if (!ok) return
          }
          await doClose()
        } catch {
          // confirm() or invoke() failed — force close as fallback
          try {
            await doClose()
          } catch { /* nothing we can do */ }
        }
      })
      .then((fn) => { unlisten = fn })
      .catch(() => { /* listener registration failed — log only */ })

    return () => { unlisten?.() }
  }, [])

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

  async function handleClearFiles(): Promise<void> {
    if (files.length === 0) return
    try {
      await Promise.all(files.map((f) => window.api.removeFile(f.id)))
      setFiles([])
      setLastCitations([])
    } catch (err) {
      console.error('Failed to clear files:', err)
    }
  }

  async function handleClearSessions(): Promise<void> {
    try {
      await Promise.all(sessions.map((s) => window.api.deleteSession(s.id)))
      setSessions([])
      handleNewChat()
    } catch (err) {
      console.error('Failed to clear sessions:', err)
    }
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
        onClearSessions={handleClearSessions}
        onAddFiles={handleAddFiles}
        onOpenSettings={() => setView('settings')}
      />

      <div className="flex flex-1 flex-col overflow-hidden">
        <ChatInterface
          messages={messages}
          files={files}
          isQuerying={isQuerying}
          isLoading={isLoading}
          loadError={loadError}
          chatMode={chatMode}
          sessionName={makeSessionName(messages)}
          onQuery={handleQuery}
          onNewChat={handleNewChat}
          onAddFiles={handleAddFiles}
          onAddFolder={handleAddFolder}
          onRemoveFile={handleRemoveFile}
          onLoadPaths={handleLoadPaths}
          onViewCitation={setViewerCitation}
        />
      </div>

      <ContextPanel
        files={files}
        citations={lastCitations}
        isQuerying={isQuerying}
        isLoading={isLoading}
        collapsed={viewerCitation !== null}
        onAddFiles={handleAddFiles}
        onRemoveFile={handleRemoveFile}
        onClearFiles={handleClearFiles}
        onViewCitation={setViewerCitation}
      />

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

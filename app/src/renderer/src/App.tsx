import { useEffect, useRef, useState } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { confirm, save } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'
import {
  AppSettings,
  ChatMessage,
  ChatSession,
  Citation,
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
import Toast, { ToastMessage } from './components/Toast'

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
  const [queryPhase, setQueryPhase] = useState('')
  const [loadError, setLoadError] = useState<string | null>(null)
  const [lastCitations, setLastCitations] = useState<Citation[]>([])
  const [viewerCitation, setViewerCitation] = useState<Citation | null>(null)
  const [toasts, setToasts] = useState<ToastMessage[]>([])
  // Tracks manually set session name — null means use auto-name from first message
  const [sessionCustomName, setSessionCustomName] = useState<string | null>(null)

  const messagesRef = useRef(messages)
  const sessionIdRef = useRef(currentSessionId)
  const sessionCreatedAtRef = useRef(sessionCreatedAt)
  const sessionCustomNameRef = useRef(sessionCustomName)
  messagesRef.current = messages
  sessionIdRef.current = currentSessionId
  sessionCreatedAtRef.current = sessionCreatedAt
  sessionCustomNameRef.current = sessionCustomName

  // Track busy state in a ref so the close listener always reads the current value
  const isBusyRef = useRef(false)
  useEffect(() => {
    isBusyRef.current = isQuerying || showModelSetup
  }, [isQuerying, showModelSetup])

  // ── Toast helpers ──────────────────────────────────────────────
  function addToast(type: ToastMessage['type'], message: string): void {
    const id = uuidv4()
    setToasts((prev) => [...prev, { id, type, message }])
  }

  function removeToast(id: string): void {
    setToasts((prev) => prev.filter((t) => t.id !== id))
  }

  // Close protection — intercepted from Rust; confirm if a query is in progress.
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
          try {
            await doClose()
          } catch { /* nothing we can do */ }
        }
      })
      .then((fn) => { unlisten = fn })
      .catch(() => { /* listener registration failed */ })

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
        name: sessionCustomNameRef.current ?? makeSessionName(messagesRef.current),
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
    const ok = await confirm(
      `Remove all ${files.length} document${files.length === 1 ? '' : 's'}? This will clear the vector index.`,
      { title: 'Clear Documents', kind: 'warning' }
    )
    if (!ok) return
    try {
      await Promise.all(files.map((f) => window.api.removeFile(f.id)))
      setFiles([])
      setLastCitations([])
      addToast('success', 'All documents removed')
    } catch (err) {
      console.error('Failed to clear files:', err)
      addToast('error', 'Failed to remove documents')
    }
  }

  async function handleClearSessions(): Promise<void> {
    if (sessions.length === 0) return
    const ok = await confirm(
      `Delete all ${sessions.length} conversation${sessions.length === 1 ? '' : 's'}? This cannot be undone.`,
      { title: 'Clear All Chats', kind: 'warning' }
    )
    if (!ok) return
    try {
      await Promise.all(sessions.map((s) => window.api.deleteSession(s.id)))
      setSessions([])
      handleNewChat()
      addToast('success', 'All conversations deleted')
    } catch (err) {
      console.error('Failed to clear sessions:', err)
      addToast('error', 'Failed to delete conversations')
    }
  }

  async function handleRemoveFile(id: string): Promise<void> {
    const file = files.find((f) => f.id === id)
    try {
      await window.api.removeFile(id)
      setFiles((prev) => prev.filter((f) => f.id !== id))
      if (file) {
        setLastCitations((prev) => prev.filter((c) => c.filePath !== file.filePath))
        setViewerCitation((prev) => (prev?.filePath === file.filePath ? null : prev))
      }
      addToast('success', `Removed ${file?.fileName ?? 'document'}`)
    } catch (err) {
      console.error('Failed to remove file:', err)
      addToast('error', 'Failed to remove document')
    }
  }

  // ── Chat ──────────────────────────────────────────────────────
  async function handleQuery(question: string): Promise<void> {
    // Collect last 3 completed user→assistant pairs for conversation context
    const historyPairs: [string, string][] = []
    for (let i = 0; i + 1 < messages.length; i++) {
      const m = messages[i]
      const next = messages[i + 1]
      if (
        m.role === 'user' &&
        next.role === 'assistant' &&
        !next.isStreaming &&
        next.content.trim()
      ) {
        historyPairs.push([m.content, next.content])
        i++ // skip assistant we just consumed
      }
    }
    const recentHistory = historyPairs.slice(-3)

    const userMessage: ChatMessage = {
      id: uuidv4(),
      role: 'user',
      content: question,
      timestamp: Date.now(),
    }
    setMessages((prev) => [...prev, userMessage])
    setIsQuerying(true)
    setQueryPhase('')

    const streamingId = uuidv4()
    let unlistenToken: (() => void) | undefined
    let unlistenStatus: (() => void) | undefined

    try {
      unlistenToken = await window.api.onQueryToken((token: string) => {
        setMessages((prev) => {
          const existing = prev.find((m) => m.id === streamingId)
          if (existing) {
            return prev.map((m) =>
              m.id === streamingId ? { ...m, content: m.content + token } : m
            )
          }
          const streamingMsg: ChatMessage = {
            id: streamingId,
            role: 'assistant',
            content: token,
            citations: [],
            isStreaming: true,
            timestamp: Date.now(),
          }
          return [...prev, streamingMsg]
        })
      })

      unlistenStatus = await window.api.onQueryStatus(
        (status: { phase: string; chunks?: number }) => {
          if (status.phase === 'embedding') {
            setQueryPhase('Embedding query…')
          } else if (status.phase === 'searching') {
            setQueryPhase(
              status.chunks != null
                ? `Searching ${status.chunks} chunks…`
                : 'Searching documents…'
            )
          } else if (status.phase === 'generating') {
            setQueryPhase('Generating answer…')
          }
        }
      )

      const result = await window.api.query(question, recentHistory)

      const finalMessage: ChatMessage = {
        id: streamingId,
        role: 'assistant',
        content: result.answer,
        citations: result.citations,
        notFound: result.notFound,
        isStreaming: false,
        timestamp: Date.now(),
      }
      setMessages((prev) => {
        const hasStreaming = prev.some((m) => m.id === streamingId)
        if (hasStreaming) {
          return prev.map((m) => (m.id === streamingId ? finalMessage : m))
        }
        return [...prev, finalMessage]
      })
      setLastCitations(result.citations)
    } catch (err) {
      const errorMessage: ChatMessage = {
        id: streamingId,
        role: 'assistant',
        content: `Unable to get a response. ${err instanceof Error ? err.message : 'Please try again.'}`,
        citations: [],
        notFound: true,
        isStreaming: false,
        timestamp: Date.now(),
      }
      setMessages((prev) => {
        const hasStreaming = prev.some((m) => m.id === streamingId)
        if (hasStreaming) {
          return prev.map((m) => (m.id === streamingId ? errorMessage : m))
        }
        return [...prev, errorMessage]
      })
    } finally {
      unlistenToken?.()
      unlistenStatus?.()
      setIsQuerying(false)
      setQueryPhase('')
    }
  }

  // ── Export ─────────────────────────────────────────────────────
  async function handleExportChat(): Promise<void> {
    if (messages.length === 0) return
    const sessionName = sessionCustomName ?? makeSessionName(messages)
    const dateStr = new Date().toLocaleString()

    const parts: string[] = [
      `# Justice AI — Conversation Export`,
      ``,
      `**Session:** ${sessionName}  `,
      `**Exported:** ${dateStr}`,
      ``,
      `---`,
      ``,
    ]

    for (const m of messages) {
      if (m.isStreaming) continue
      if (m.role === 'user') {
        parts.push(`## You`, ``, m.content, ``)
      } else {
        parts.push(`## Justice AI`, ``, m.content, ``)
        if (m.citations && m.citations.length > 0) {
          parts.push(``, `**Sources:**`)
          m.citations.forEach((c, i) => {
            const excerpt = c.excerpt.length > 160 ? c.excerpt.slice(0, 160) + '…' : c.excerpt
            parts.push(`${i + 1}. **${c.fileName}** · Page ${c.pageNumber}`, `   > "${excerpt}"`)
          })
          parts.push(``)
        }
      }
      parts.push(`---`, ``)
    }

    const content = parts.join('\n')
    try {
      const filePath = await save({
        defaultPath: `${sessionName.replace(/[/\\:*?"<>|]/g, '-')}.md`,
        filters: [{ name: 'Markdown', extensions: ['md'] }],
      })
      if (!filePath) return
      await window.api.saveFile(filePath, content)
      addToast('success', 'Conversation exported')
    } catch (err) {
      console.error('Export failed:', err)
      addToast('error', 'Export failed')
    }
  }

  async function handleExportCitations(): Promise<void> {
    if (lastCitations.length === 0) return
    const rows = [
      `"Source","File","Page","Score","Excerpt"`,
      ...lastCitations.map((c, i) =>
        `${i + 1},"${c.fileName.replace(/"/g, '""')}",${c.pageNumber},${c.score.toFixed(3)},"${c.excerpt.replace(/"/g, '""').slice(0, 200)}"`
      ),
    ]
    const content = rows.join('\n')
    try {
      const filePath = await save({
        defaultPath: 'Justice AI Citations.csv',
        filters: [{ name: 'CSV', extensions: ['csv'] }],
      })
      if (!filePath) return
      await window.api.saveFile(filePath, content)
      addToast('success', 'Citations exported')
    } catch (err) {
      console.error('Export failed:', err)
      addToast('error', 'Export failed')
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
    setSessionCustomName(null)
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
    setSessionCustomName(null)
    setView('main')
  }

  async function handleLoadSession(session: ChatSession): Promise<void> {
    setMessages(session.messages)
    setCurrentSessionId(session.id)
    setSessionCreatedAt(session.createdAt)
    setChatMode(true)
    setLastCitations([])
    setViewerCitation(null)
    // Preserve the session's existing name (prevents auto-rename on next message)
    setSessionCustomName(session.name)
    setView('main')
  }

  async function handleDeleteSession(sessionId: string): Promise<void> {
    const session = sessions.find((s) => s.id === sessionId)
    const ok = await confirm(
      `Delete "${session?.name ?? 'this conversation'}"?`,
      { title: 'Delete Chat', kind: 'warning' }
    )
    if (!ok) return
    try {
      await window.api.deleteSession(sessionId)
      setSessions((prev) => prev.filter((s) => s.id !== sessionId))
      if (sessionId === currentSessionId) handleNewChat()
      addToast('success', 'Conversation deleted')
    } catch (err) {
      console.error('Failed to delete session:', err)
      addToast('error', 'Failed to delete conversation')
    }
  }

  async function handleRenameSession(id: string, newName: string): Promise<void> {
    const trimmed = newName.trim()
    if (!trimmed) return

    // Update current session custom name
    if (id === currentSessionId) {
      setSessionCustomName(trimmed)
    }

    // Update sessions list
    setSessions((prev) => prev.map((s) => s.id === id ? { ...s, name: trimmed } : s))

    // Persist to disk
    const session = sessions.find((s) => s.id === id)
    if (session) {
      try {
        await window.api.saveSession({ ...session, name: trimmed, updatedAt: Date.now() })
      } catch { }
    }
    addToast('success', 'Conversation renamed')
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
        onRenameSession={handleRenameSession}
        onClearSessions={handleClearSessions}
        onAddFiles={handleAddFiles}
        onOpenSettings={() => setView('settings')}
      />

      <div className="flex flex-1 flex-col overflow-hidden">
        <ChatInterface
          messages={messages}
          files={files}
          isQuerying={isQuerying}
          queryPhase={queryPhase}
          isLoading={isLoading}
          loadError={loadError}
          chatMode={chatMode}
          sessionName={sessionCustomName ?? makeSessionName(messages)}
          onQuery={handleQuery}
          onNewChat={handleNewChat}
          onAddFiles={handleAddFiles}
          onAddFolder={handleAddFolder}
          onRemoveFile={handleRemoveFile}
          onLoadPaths={handleLoadPaths}
          onViewCitation={setViewerCitation}
          onExportChat={messages.length > 0 ? handleExportChat : undefined}
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
        onExportCitations={lastCitations.length > 0 ? handleExportCitations : undefined}
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

      <Toast toasts={toasts} onDismiss={removeToast} />
    </div>
  )
}

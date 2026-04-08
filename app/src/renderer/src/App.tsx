import { useEffect, useRef, useState } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { confirm, save } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'
import {
  AppSettings,
  Case,
  ChatMessage,
  ChatSession,
  Citation,
  DEFAULT_SETTINGS,
  FileInfo,
  InferenceMode,
} from '../../../../shared/src/types'
import { v4 as uuidv4 } from 'uuid'
import Sidebar from './components/Sidebar'
import ContextPanel from './components/ContextPanel'
import ChatInterface from './components/ChatInterface'
import Settings from './components/Settings'
import DocumentViewer from './components/DocumentViewer'
import ModelSetup from './components/ModelSetup'
import PanelErrorBoundary from './components/PanelErrorBoundary'
import Toast, { ToastMessage } from './components/Toast'
import CommandPalette, { PaletteAction } from './components/CommandPalette'
import { makeSessionName, makeSessionSummary } from './utils/sessionName'

type View = 'main' | 'settings'
const SUPPORTED_EXTENSIONS = ['.pdf', '.docx', '.txt', '.md', '.csv', '.eml', '.html', '.htm', '.mhtml', '.xml', '.xlsx', '.png', '.jpg', '.jpeg', '.tif', '.tiff']

// Practice area presets (mirrors Settings.tsx PRESETS for deriving active area)
const PRACTICE_AREA_PRESETS = [
  { name: 'General',               chunkSize: 1000, chunkOverlap: 150, topK: 6 },
  { name: 'Criminal Law',          chunkSize: 1200, chunkOverlap: 200, topK: 8 },
  { name: 'Family / Domestic',     chunkSize: 800,  chunkOverlap: 100, topK: 5 },
  { name: 'Corporate / Contract',  chunkSize: 1500, chunkOverlap: 200, topK: 8 },
  { name: 'Immigration',           chunkSize: 1000, chunkOverlap: 150, topK: 7 },
  { name: 'Personal Injury',       chunkSize: 1000, chunkOverlap: 120, topK: 7 },
  { name: 'Real Estate / Property',chunkSize: 1200, chunkOverlap: 180, topK: 7 },
  { name: 'Employment / Labor',    chunkSize: 1100, chunkOverlap: 160, topK: 7 },
  { name: 'Regulatory / Compliance', chunkSize: 1400, chunkOverlap: 200, topK: 8 },
]

function getActivePracticeArea(s: AppSettings): string | null {
  const m = PRACTICE_AREA_PRESETS.find(
    (p) => p.chunkSize === s.chunkSize && p.chunkOverlap === s.chunkOverlap && p.topK === s.topK
  )
  return m?.name ?? 'General'
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
  // Page texts for key facts extraction (frontend-only)
  const [pageTexts, setPageTexts] = useState<string[]>([])
  // Case folders
  const [cases, setCases] = useState<Case[]>([])
  const [currentCaseId, setCurrentCaseId] = useState<string | null>(null)
  // Delete-case confirmation modal
  const [deleteCaseTarget, setDeleteCaseTarget] = useState<Case | null>(null)
  // Context panel minimize state
  const [contextPanelMinimized, setContextPanelMinimized] = useState(false)
  // Command palette
  const [showCommandPalette, setShowCommandPalette] = useState(false)

  const queryAbortRef = useRef(false)
  const messagesRef = useRef(messages)
  const sessionIdRef = useRef(currentSessionId)
  const sessionCreatedAtRef = useRef(sessionCreatedAt)
  const sessionCustomNameRef = useRef(sessionCustomName)
  const currentCaseIdRef = useRef(currentCaseId)
  messagesRef.current = messages
  sessionIdRef.current = currentSessionId
  sessionCreatedAtRef.current = sessionCreatedAt
  sessionCustomNameRef.current = sessionCustomName
  currentCaseIdRef.current = currentCaseId

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
      } catch {
        addToast('error', 'Failed to load settings — using defaults')
      }
      try {
        const existingFiles = await window.api.getFiles()
        setFiles(existingFiles)
      } catch {
        addToast('error', 'Failed to load saved documents')
      }
      try {
        const saved = await window.api.getSessions()
        setSessions(saved)
      } catch {
        addToast('error', 'Failed to load chat history')
      }
      try {
        const savedCases = await window.api.getCases()
        setCases(savedCases)
      } catch {
        addToast('error', 'Failed to load cases')
      }
      try {
        const modelStatus = await window.api.checkModels()
        if (!modelStatus.llmReady) setShowModelSetup(true)
      } catch {
        addToast('error', 'Failed to check model status')
      }
    }
    init()
  }, [])

  // Auto-save current session (debounced 1s)
  useEffect(() => {
    // Don't save if only greeting messages exist
    const persistable = messages.filter((m) => !m.isGreeting)
    if (persistable.length === 0) return
    const timer = setTimeout(async () => {
      const saveMsgs = messagesRef.current.filter((m) => !m.isGreeting)
      const session: ChatSession = {
        id: sessionIdRef.current,
        name: sessionCustomNameRef.current ?? makeSessionName(saveMsgs),
        messages: saveMsgs,
        createdAt: sessionCreatedAtRef.current,
        updatedAt: Date.now(),
        caseId: currentCaseIdRef.current ?? undefined,
        summary: makeSessionSummary(saveMsgs) || undefined,
      }
      try {
        await window.api.saveSession(session)
        const updated = await window.api.getSessions()
        setSessions(updated)
      } catch { }
    }, 1000)
    return () => clearTimeout(timer)
  }, [messages])

  // Fetch page texts for key facts extraction when files change
  useEffect(() => {
    if (files.length === 0) { setPageTexts([]); return }
    let cancelled = false
    async function fetchTexts(): Promise<void> {
      const texts: string[] = []
      for (const f of files) {
        // Fetch first 5 pages max per file to keep it fast
        const maxPages = Math.min(f.totalPages, 5)
        for (let p = 1; p <= maxPages; p++) {
          try {
            const text = await window.api.getPageText(f.filePath, p)
            if (text) texts.push(text)
          } catch { /* skip */ }
        }
      }
      if (!cancelled) setPageTexts(texts)
    }
    fetchTexts()
    return () => { cancelled = true }
  }, [files])

  // ── File management ───────────────────────────────────────────
  async function handleLoadPaths(paths: string[]): Promise<void> {
    setLoadError(null)
    setIsLoading(true)
    try {
      const loaded = await window.api.loadFiles(paths, currentCaseId ?? undefined)
      if (loaded.length === 0) {
        const hasSupportedExtensions = paths.some((p) => {
          const lower = p.toLowerCase()
          return SUPPORTED_EXTENSIONS.some((ext) => lower.endsWith(ext))
        })
        if (!hasSupportedExtensions) {
          setLoadError('Unsupported format. Supported: PDF, DOCX, TXT, MD, CSV, EML, HTML/MHTML, XML, XLSX, and common image formats (OCR).')
        } else {
          setLoadError('Could not read the file. It may be scanned, password-protected, or corrupted.')
        }
        return
      }
      setFiles((prev) => {
        const existingIds = new Set(prev.map((f) => f.id))
        return [...prev, ...loaded.filter((f) => !existingIds.has(f.id))]
      })
    } catch (err) {
      const msg = err instanceof Error ? err.message.toLowerCase() : ''
      if (msg.includes('permission') || msg.includes('access denied')) {
        setLoadError('Permission denied. Check that the app has access to this file.')
      } else if (msg.includes('password') || msg.includes('encrypt')) {
        setLoadError('This file is password-protected and cannot be opened.')
      } else {
        setLoadError(err instanceof Error ? err.message : 'Failed to load files. Please try again.')
      }
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
      `Strike all ${files.length} document${files.length === 1 ? '' : 's'} from the record? This will clear the vector index.`,
      { title: 'Strike from the Record', kind: 'warning' }
    )
    if (!ok) return
    try {
      await Promise.all(files.map((f) => window.api.removeFile(f.id)))
      setFiles([])
      setLastCitations([])
      setPageTexts([])
      addToast('success', 'Documents struck from the record')
    } catch (err) {
      console.error('Failed to clear files:', err)
      addToast('error', 'Failed to remove documents')
    }
  }

  async function handleClearSessions(): Promise<void> {
    if (sessions.length === 0) return
    const ok = await confirm(
      `Strike all ${sessions.length} conversation${sessions.length === 1 ? '' : 's'} from the record? This cannot be undone.`,
      { title: 'Strike from the Record', kind: 'warning' }
    )
    if (!ok) return
    try {
      await Promise.all(sessions.map((s) => window.api.deleteSession(s.id)))
      setSessions([])
      handleNewChat()
      addToast('success', 'All conversations struck from the record')
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
  // NOTE: Do NOT call setLastCitations([]) at query start.
  // Preserve previous citations until new results arrive to avoid flashing the empty context panel.
  // Citations are replaced when result lands, or preserved on error/cancel.
  async function handleQuery(question: string): Promise<void> {
    queryAbortRef.current = false
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
        if (queryAbortRef.current) return
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
          if (queryAbortRef.current) return
          if (status.phase === 'embedding') {
            setQueryPhase('Embedding query')
          } else if (status.phase === 'searching') {
            setQueryPhase(
              status.chunks != null
                ? `Searching ${status.chunks} chunks`
                : 'Searching documents'
            )
          } else if (status.phase === 'generating') {
            setQueryPhase('Generating answer')
          }
        }
      )

      // Build cross-conversation context from sibling sessions in the same case
      let caseContext: string | undefined
      if (currentCaseId) {
        try {
          const summaries = await window.api.getCaseSummaries(currentCaseId, currentSessionId)
          if (summaries.length > 0) {
            caseContext = summaries.map((s) => s.summary).join('\n')
          }
        } catch { }
      }

      const result = await window.api.query(question, recentHistory, currentCaseId ?? undefined, caseContext)

      const finalMessage: ChatMessage = {
        id: streamingId,
        role: 'assistant',
        content: result.answer,
        citations: result.citations,
        notFound: result.notFound,
        isStreaming: false,
        timestamp: Date.now(),
        qualityAssertions: result.assertions,
        inferenceMode: settings.inferenceMode,
        confidence: result.confidence,
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
    const exportMsgs = messages.filter((m) => !m.isGreeting && !m.isStreaming)
    if (exportMsgs.length === 0) return
    const sessionName = sessionCustomName ?? makeSessionName(exportMsgs)
    const dateStr = new Date().toLocaleDateString('en-US', { year: 'numeric', month: 'long', day: 'numeric' })

    const parts: string[] = [
      `# MEMORANDUM`,
      ``,
      `| | |`,
      `|---|---|`,
      `| **TO:** | [Client / File] |`,
      `| **FROM:** | Justice AI Legal Research Assistant |`,
      `| **DATE:** | ${dateStr} |`,
      `| **RE:** | ${sessionName} |`,
      ``,
      `---`,
      ``,
    ]

    // Collect all citations for footnotes
    const allCitations: Citation[] = []
    const citationKey = (c: Citation): string => `${c.fileName}::${c.pageNumber}`
    const seenCitations = new Set<string>()

    let questionNum = 0
    for (const m of exportMsgs) {
      if (m.role === 'user') {
        questionNum++
        parts.push(`## ${questionNum}. Question`, ``, `> ${m.content}`, ``)
      } else {
        parts.push(`### Analysis`, ``, m.content, ``)
        if (m.citations && m.citations.length > 0) {
          for (const c of m.citations) {
            const key = citationKey(c)
            if (!seenCitations.has(key)) {
              seenCitations.add(key)
              allCitations.push(c)
            }
          }
        }
        parts.push(`---`, ``)
      }
    }

    // Footnotes section
    if (allCitations.length > 0) {
      parts.push(`## Sources Cited`, ``)
      allCitations.forEach((c, i) => {
        const excerpt = c.excerpt.length > 200 ? c.excerpt.slice(0, 200) + '\u2026' : c.excerpt
        parts.push(`${i + 1}. **${c.fileName}**, p. ${c.pageNumber}`)
        parts.push(`   > "${excerpt}"`)
        parts.push(``)
      })
    }

    parts.push(`---`, ``, `*Generated by Justice AI — all processing performed locally on-device.*`)

    const content = parts.join('\n')
    try {
      const filePath = await save({
        defaultPath: `${sessionName.replace(/[/\\:*?"<>|]/g, '-')} — Memo.md`,
        filters: [{ name: 'Markdown', extensions: ['md'] }],
      })
      if (!filePath) return
      await window.api.saveFile(filePath, content)
      addToast('success', 'Legal memo exported')
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
    queryAbortRef.current = true
    setChatMode(false)
    setMessages([])
    setCurrentSessionId(uuidv4())
    setSessionCreatedAt(Date.now())
    setLastCitations([])
    setViewerCitation(null)
    setSessionCustomName(null)
    setCurrentCaseId(null)
    setView('main')
  }

  // ── Sessions ──────────────────────────────────────────────────
  function handleNewChat(): void {
    queryAbortRef.current = true
    const newId = uuidv4()
    setMessages([])
    setCurrentSessionId(newId)
    setSessionCreatedAt(Date.now())
    setChatMode(true)
    setLastCitations([])
    setViewerCitation(null)
    setSessionCustomName(null)
    // Preserve currentCaseId so new chats auto-belong to the current project
    setView('main')
  }

  async function handleLoadSession(session: ChatSession): Promise<void> {
    queryAbortRef.current = true
    setMessages(session.messages)
    setCurrentSessionId(session.id)
    setSessionCreatedAt(session.createdAt)
    setChatMode(true)
    setLastCitations([])
    setViewerCitation(null)
    // Preserve the session's existing name (prevents auto-rename on next message)
    setSessionCustomName(session.name)
    setCurrentCaseId(session.caseId ?? null)
    setView('main')
  }

  async function handleDeleteSession(sessionId: string): Promise<void> {
    const session = sessions.find((s) => s.id === sessionId)
    const ok = await confirm(
      `Strike "${session?.name ?? 'this conversation'}" from the record?`,
      { title: 'Strike from the Record', kind: 'warning' }
    )
    if (!ok) return
    try {
      await window.api.deleteSession(sessionId)
      setSessions((prev) => prev.filter((s) => s.id !== sessionId))
      if (sessionId === currentSessionId) handleNewChat()
      addToast('success', 'Struck from the record')
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

    // Use functional update to read fresh state AND persist atomically
    let sessionToSave: ChatSession | undefined
    setSessions((prev) => {
      const updated = prev.map((s) => {
        if (s.id !== id) return s
        const renamed = { ...s, name: trimmed, updatedAt: Date.now() }
        sessionToSave = renamed
        return renamed
      })
      return updated
    })

    // Persist to disk using the fresh session object
    if (sessionToSave) {
      try {
        await window.api.saveSession(sessionToSave)
      } catch { }
    }
    addToast('success', 'Conversation renamed')
  }

  // ── Cases ───────────────────────────────────────────────────
  async function handleCreateCase(name: string): Promise<void> {
    const now = Date.now()
    const newCase: Case = {
      id: uuidv4(),
      name: name.trim(),
      createdAt: now,
      updatedAt: now,
    }
    try {
      await window.api.saveCase(newCase)
      setCases((prev) => [...prev, newCase])
      setCurrentCaseId(newCase.id)
      addToast('success', `Case "${newCase.name}" created`)
    } catch (err) {
      console.error('Failed to create case:', err)
      addToast('error', 'Failed to create case')
    }
  }

  async function handleRenameCase(id: string, name: string): Promise<void> {
    const trimmed = name.trim()
    if (!trimmed) return
    const existing = cases.find((c) => c.id === id)
    if (!existing) return
    const updated: Case = { ...existing, name: trimmed, updatedAt: Date.now() }
    try {
      await window.api.saveCase(updated)
      setCases((prev) => prev.map((c) => (c.id === id ? updated : c)))
      addToast('success', 'Case renamed')
    } catch (err) {
      console.error('Failed to rename case:', err)
      addToast('error', 'Failed to rename case')
    }
  }

  function handleDeleteCase(id: string): void {
    const c = cases.find((c) => c.id === id)
    if (c) setDeleteCaseTarget(c)
  }

  async function confirmDeleteCase(deleteContents: boolean): Promise<void> {
    const c = deleteCaseTarget
    if (!c) return
    setDeleteCaseTarget(null)
    try {
      await window.api.deleteCase(c.id, deleteContents)
      setCases((prev) => prev.filter((x) => x.id !== c.id))
      if (deleteContents) {
        setSessions((prev) => prev.filter((s) => s.caseId !== c.id))
        setFiles((prev) => prev.filter((f) => f.caseId !== c.id))
      } else {
        setSessions((prev) =>
          prev.map((s) => (s.caseId === c.id ? { ...s, caseId: undefined } : s))
        )
        setFiles((prev) =>
          prev.map((f) => (f.caseId === c.id ? { ...f, caseId: undefined } : f))
        )
      }
      if (currentCaseId === c.id) setCurrentCaseId(null)
      addToast('success', `Case "${c.name}" deleted`)
    } catch (err) {
      console.error('Failed to delete case:', err)
      addToast('error', 'Failed to delete case')
    }
  }

  function handleSelectCase(id: string | null): void {
    setCurrentCaseId(id)
  }

  async function handleMoveSessionToCase(sessionId: string, caseId: string | null): Promise<void> {
    try {
      await window.api.assignSessionToCase(sessionId, caseId)
      setSessions((prev) =>
        prev.map((s) => (s.id === sessionId ? { ...s, caseId: caseId ?? undefined } : s))
      )
    } catch (err) {
      console.error('Failed to move session:', err)
      addToast('error', 'Failed to move session')
    }
  }

  async function handleMoveFileToCase(fileId: string, caseId: string | null): Promise<void> {
    try {
      await window.api.assignFileToCase(fileId, caseId)
      setFiles((prev) =>
        prev.map((f) => (f.id === fileId ? { ...f, caseId: caseId ?? undefined } : f))
      )
    } catch (err) {
      console.error('Failed to move file:', err)
      addToast('error', 'Failed to move file')
    }
  }

  // Filtered files based on current case
  const caseFiles = currentCaseId
    ? files.filter((f) => f.caseId === currentCaseId)
    : files

  function handleToggleTheme(): void {
    const newTheme = settings.theme === 'dark' ? 'light' : 'dark'
    const newSettings = { ...settings, theme: newTheme as AppSettings['theme'] }
    handleSaveSettings(newSettings)
  }

  function handleInferenceModeChange(mode: InferenceMode): void {
    const newSettings = { ...settings, inferenceMode: mode }
    handleSaveSettings(newSettings)
  }

  function handleDeleteMessage(id: string): void {
    setMessages((prev) => {
      const idx = prev.findIndex((m) => m.id === id)
      if (idx === -1) return prev
      const msg = prev[idx]
      if (msg.role === 'user') {
        // Delete user message and the following assistant message (pair)
        const next = prev[idx + 1]
        if (next && next.role === 'assistant') {
          return prev.filter((_, i) => i !== idx && i !== idx + 1)
        }
        return prev.filter((_, i) => i !== idx)
      } else {
        // Delete assistant message and the preceding user message (pair)
        const prevMsg = prev[idx - 1]
        if (prevMsg && prevMsg.role === 'user') {
          return prev.filter((_, i) => i !== idx && i !== idx - 1)
        }
        return prev.filter((_, i) => i !== idx)
      }
    })
  }

  function handleRetryMessage(id: string): void {
    const idx = messages.findIndex((m) => m.id === id)
    if (idx === -1) return
    // Find the preceding user message
    let userMsg: ChatMessage | undefined
    for (let j = idx - 1; j >= 0; j--) {
      if (messages[j].role === 'user') {
        userMsg = messages[j]
        break
      }
    }
    if (!userMsg) return
    const question = userMsg.content
    // Remove the assistant message being retried, then re-query after state settles
    setMessages((prev) => prev.filter((m) => m.id !== id))
    setTimeout(() => handleQuery(question), 0)
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

  async function handleReindex(): Promise<void> {
    if (files.length === 0) return
    setIsLoading(true)
    try {
      // Snapshot current files with their case assignments
      const snapshot = files.map((f) => ({ id: f.id, filePath: f.filePath, caseId: f.caseId }))

      // Remove all files (clears old chunks)
      await Promise.all(snapshot.map((f) => window.api.removeFile(f.id)))

      // Collect unique file paths
      const paths = [...new Set(snapshot.map((f) => f.filePath))]

      // Re-load all files (re-chunks with new settings)
      const reloaded = await window.api.loadFiles(paths)

      // Re-assign case IDs based on original file paths
      const pathToCaseId = new Map<string, string>()
      for (const f of snapshot) {
        if (f.caseId) pathToCaseId.set(f.filePath, f.caseId)
      }

      for (const file of reloaded) {
        const caseId = pathToCaseId.get(file.filePath)
        if (caseId) {
          await window.api.assignFileToCase(file.id, caseId)
          file.caseId = caseId
        }
      }

      setFiles(reloaded)
      addToast('success', `Re-indexed ${reloaded.length} document${reloaded.length === 1 ? '' : 's'}`)
    } catch (err) {
      console.error('Re-index failed:', err)
      addToast('error', 'Re-indexing failed. Some documents may need to be re-added.')
      // Refresh file list from backend
      try {
        const remaining = await window.api.getFiles()
        setFiles(remaining)
      } catch { }
    } finally {
      setIsLoading(false)
    }
  }

  // Apply theme to root so CSS variables cascade
  useEffect(() => {
    document.documentElement.setAttribute('data-theme', settings.theme)
  }, [settings.theme])

  // Cmd+K / Ctrl+K command palette
  useEffect(() => {
    function handleKeyDown(e: globalThis.KeyboardEvent): void {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault()
        setShowCommandPalette((v) => !v)
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [])

  const paletteActions: PaletteAction[] = [
    {
      id: 'new-chat',
      label: 'New Chat',
      icon: <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M1.75 1h8.5c.966 0 1.75.784 1.75 1.75v5.5A1.75 1.75 0 0 1 10.25 10H7.06l-2.573 2.573A1.458 1.458 0 0 1 2 11.543V10h-.25A1.75 1.75 0 0 1 0 8.25v-5.5C0 1.784.784 1 1.75 1ZM1.5 2.75v5.5c0 .138.112.25.25.25H3v2.19l2.72-2.72h4.53a.25.25 0 0 0 .25-.25v-5.5a.25.25 0 0 0-.25-.25h-8.5a.25.25 0 0 0-.25.25Z"/></svg>,
      onAction: handleNewChat,
    },
    {
      id: 'upload-docs',
      label: 'Add Documents',
      icon: <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="17 8 12 3 7 8" /><line x1="12" y1="3" x2="12" y2="15" /></svg>,
      onAction: handleAddFiles,
    },
    {
      id: 'upload-folder',
      label: 'Add Folder',
      icon: <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M.54 3.87.5 3a2 2 0 0 1 2-2h3.672a2 2 0 0 1 1.414.586l.828.828A2 2 0 0 0 9.828 3H13.5a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-11a2 2 0 0 1-2-2V3.87Z"/></svg>,
      onAction: handleAddFolder,
    },
    {
      id: 'cycle-mode',
      label: 'Cycle Inference Mode',
      icon: <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round"><circle cx="8" cy="8" r="6"/><line x1="8" y1="5" x2="8" y2="8"/><line x1="8" y1="8" x2="10.5" y2="10"/></svg>,
      onAction: () => {
        const modes: InferenceMode[] = ['quick', 'balanced', 'extended']
        const cur = settings.inferenceMode ?? 'balanced'
        const next = modes[(modes.indexOf(cur) + 1) % modes.length]
        handleInferenceModeChange(next)
      },
    },
    {
      id: 'export',
      label: 'Export Chat',
      icon: <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M2.75 14A1.75 1.75 0 0 1 1 12.25v-2.5a.75.75 0 0 1 1.5 0v2.5c0 .138.112.25.25.25h10.5a.25.25 0 0 0 .25-.25v-2.5a.75.75 0 0 1 1.5 0v2.5A1.75 1.75 0 0 1 13.25 14ZM7.25 7.689V2a.75.75 0 0 1 1.5 0v5.689l1.97-1.97a.749.749 0 1 1 1.06 1.06l-3.25 3.25a.749.749 0 0 1-1.06 0L4.22 6.779a.749.749 0 1 1 1.06-1.06l1.97 1.97Z"/></svg>,
      onAction: handleExportChat,
    },
    {
      id: 'settings',
      label: 'Open Settings',
      icon: <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M8 0a8.2 8.2 0 0 1 .701.031C9.444.095 9.99.645 10.16 1.29l.288 1.107c.018.066.079.158.2.196a5.5 5.5 0 0 1 .874.455c.112.07.214.063.276.04l1.063-.39c.618-.228 1.306-.016 1.705.474a8.2 8.2 0 0 1 .782 1.236c.306.586.2 1.294-.24 1.724l-.775.717c-.048.044-.084.136-.076.251a5.5 5.5 0 0 1 0 .91c-.008.115.028.207.076.251l.775.717c.44.43.546 1.138.24 1.724a8.2 8.2 0 0 1-.782 1.236c-.399.49-1.087.702-1.705.474l-1.063-.39c-.062-.023-.164-.03-.276.04a5.5 5.5 0 0 1-.874.455c-.121.038-.182.13-.2.196l-.288 1.107c-.17.645-.716 1.195-1.459 1.259a8.3 8.3 0 0 1-1.402 0c-.743-.064-1.289-.614-1.459-1.259l-.288-1.107c-.018-.066-.079-.158-.2-.196a5.5 5.5 0 0 1-.874-.455c-.112-.07-.214-.063-.276-.04l-1.063.39c-.618.228-1.306.016-1.705-.474a8.2 8.2 0 0 1-.782-1.236c-.306-.586-.2-1.294.24-1.724l.775-.717c.048-.044.084-.136.076-.251a5.5 5.5 0 0 1 0-.91c.008-.115-.028-.207-.076-.251l-.775-.717c-.44-.43-.546-1.138-.24-1.724a8.2 8.2 0 0 1 .782-1.236c.399-.49 1.087-.702 1.705-.474l1.063.39c.062.023.164.03.276-.04a5.5 5.5 0 0 1 .874-.455c.121-.038.182-.13.2-.196l.288-1.107C6.01.645 6.556.095 7.299.03 7.53.01 7.764 0 8 0Zm-.571 1.525c-.036.003-.108.036-.137.146l-.289 1.105c-.147.561-.549.967-.998 1.189a4 4 0 0 0-.634.33c-.418.266-.862.395-1.378.235l-1.063-.39c-.104-.038-.175.006-.21.055a6.7 6.7 0 0 0-.635 1.002c-.046.09-.042.18.026.252l.775.717c.416.384.634.917.612 1.468a4 4 0 0 0 0 .654c.022.551-.196 1.084-.612 1.468l-.775.717c-.068.072-.072.162-.026.252.167.32.363.617.635 1.002.035.049.106.093.21.055l1.063-.39c.516-.16.96-.031 1.378.235.197.126.41.235.634.33.449.222.851.628.998 1.189l.289 1.105c.029.11.101.143.137.146a6.6 6.6 0 0 0 1.142 0c.036-.003.108-.036.137-.146l.289-1.105c.147-.561.549-.967.998-1.189.224-.095.437-.204.634-.33.418-.266.862-.395 1.378-.235l1.063.39c.104.038.175-.006.21-.055a6.7 6.7 0 0 0 .635-1.002c.046-.09.042-.18-.026-.252l-.775-.717c-.416-.384-.634-.917-.612-1.468a4 4 0 0 0 0-.654c-.022-.551.196-1.084.612-1.468l.775-.717c.068-.072.072-.162.026-.252a6.7 6.7 0 0 0-.635-1.002c-.035-.049-.106-.093-.21-.055l-1.063.39c-.516.16-.96.031-1.378-.235a4 4 0 0 0-.634-.33c-.449-.222-.851-.628-.998-1.189l-.289-1.105c-.029-.11-.101-.143-.137-.146a6.6 6.6 0 0 0-1.142 0ZM11 8a3 3 0 1 1-6 0 3 3 0 0 1 6 0ZM9.5 8a1.5 1.5 0 1 0-3.001.001A1.5 1.5 0 0 0 9.5 8Z"/></svg>,
      onAction: () => setView('settings'),
    },
  ]

  return (
    <div className="flex h-screen w-screen overflow-hidden" style={{ background: 'var(--bg)' }}>
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
        files={files}
        cases={cases}
        currentCaseId={currentCaseId}
        onCreateCase={handleCreateCase}
        onSelectCase={handleSelectCase}
        onDeleteCase={handleDeleteCase}
        onRenameCase={handleRenameCase}
        onMoveSession={handleMoveSessionToCase}
        onRemoveFile={handleRemoveFile}
        onMoveFileToCase={handleMoveFileToCase}
        caseFiles={caseFiles}
        onLoadPaths={handleLoadPaths}
      />

      <main className="flex flex-1 flex-col overflow-hidden">
        <ChatInterface
          messages={messages}
          files={files}
          isQuerying={isQuerying}
          queryPhase={queryPhase}
          isLoading={isLoading}
          loadError={loadError}
          chatMode={chatMode}
          sessionName={sessionCustomName ?? makeSessionName(messages.filter((m) => !m.isGreeting))}
          sessionId={currentSessionId}
          onQuery={handleQuery}
          onAddFiles={handleAddFiles}
          onAddFolder={handleAddFolder}
          onLoadPaths={handleLoadPaths}
          onViewCitation={setViewerCitation}
          onExportChat={messages.length > 0 ? handleExportChat : undefined}
          practiceArea={getActivePracticeArea(settings)}
          chunkTexts={pageTexts}
          theme={settings.theme}
          onToggleTheme={handleToggleTheme}
          onDeleteMessage={handleDeleteMessage}
          onRetryMessage={handleRetryMessage}
          inferenceMode={settings.inferenceMode ?? 'balanced'}
          onInferenceModeChange={handleInferenceModeChange}
        />
      </main>

      <PanelErrorBoundary name="Context Panel">
        <ContextPanel
          files={caseFiles}
          citations={lastCitations}
          isQuerying={isQuerying}
          isLoading={isLoading}
          collapsed={caseFiles.length === 0 && lastCitations.length === 0}
          minimized={contextPanelMinimized}
          onToggleMinimize={() => setContextPanelMinimized((v) => !v)}
          onAddFiles={handleAddFiles}
          onRemoveFile={handleRemoveFile}
          onClearFiles={handleClearFiles}
          onViewCitation={setViewerCitation}
          activeCitation={viewerCitation}
          onExportCitations={lastCitations.length > 0 ? handleExportCitations : undefined}
          caseName={currentCaseId ? cases.find((c) => c.id === currentCaseId)?.name : undefined}
        />
      </PanelErrorBoundary>

      <PanelErrorBoundary name="Document Viewer">
        <DocumentViewer
          citation={viewerCitation}
          onClose={() => setViewerCitation(null)}
        />
      </PanelErrorBoundary>

      {showModelSetup && (
        <ModelSetup onComplete={() => setShowModelSetup(false)} />
      )}

      {view === 'settings' && (
        <PanelErrorBoundary name="Settings">
          <Settings
            settings={settings}
            onSave={handleSaveSettings}
            onClose={() => setView('main')}
            onReindex={handleReindex}
          />
        </PanelErrorBoundary>
      )}

      <Toast toasts={toasts} onDismiss={removeToast} />

      {showCommandPalette && (
        <CommandPalette
          actions={paletteActions}
          onClose={() => setShowCommandPalette(false)}
        />
      )}

      {/* Delete case confirmation modal */}
      {deleteCaseTarget && (
        <div
          className="fixed inset-0 z-[9999] flex items-center justify-center"
          style={{ background: 'rgba(0,0,0,0.6)', backdropFilter: 'blur(4px)' }}
          role="dialog"
          aria-modal="true"
          aria-labelledby="delete-case-title"
          aria-describedby="delete-case-desc"
          tabIndex={-1}
          onClick={() => setDeleteCaseTarget(null)}
          onKeyDown={(e) => { if (e.key === 'Escape') setDeleteCaseTarget(null) }}
        >
          <div
            className="rounded-xl p-6 shadow-2xl"
            style={{ background: 'var(--modal-bg)', color: 'var(--fg)', maxWidth: 420, width: '90%' }}
            onClick={(e) => e.stopPropagation()}
          >
            <h3 id="delete-case-title" className="text-lg font-semibold mb-2">Strike from the Record</h3>
            <p id="delete-case-desc" className="text-sm mb-4" style={{ color: 'var(--fg-muted)' }}>
              Strike &ldquo;{deleteCaseTarget.name}&rdquo; from the record? Choose what happens to its files and sessions.
            </p>
            <div className="flex flex-col gap-2">
              <button
                className="w-full rounded-lg px-4 py-2.5 text-sm font-medium transition-colors"
                style={{ background: 'var(--accent)', color: '#000' }}
                onClick={() => confirmDeleteCase(false)}
              >
                Keep Files &amp; Sessions
              </button>
              <button
                className="w-full rounded-lg px-4 py-2.5 text-sm font-medium transition-colors"
                style={{ background: '#dc2626', color: '#fff' }}
                onClick={() => confirmDeleteCase(true)}
              >
                Delete Everything
              </button>
              <button
                autoFocus
                className="w-full rounded-lg px-4 py-2.5 text-sm font-medium transition-colors"
                style={{ background: 'var(--hover-bg)', color: 'var(--fg)' }}
                onClick={() => setDeleteCaseTarget(null)}
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

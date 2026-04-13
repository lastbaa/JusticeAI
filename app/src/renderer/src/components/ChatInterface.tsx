import React, { useEffect, useRef, useState, KeyboardEvent } from 'react'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { ChatMessage, Citation, FileInfo, FileLoadProgress, InferenceMode, Theme } from '../../../../../shared/src/types'
import MessageBubble from './MessageBubble'
import QueryTemplates from './QueryTemplates'
import FactsPanel from './FactsPanel'

interface Props {
  messages: ChatMessage[]
  isQuerying: boolean
  queryPhase?: string
  files: FileInfo[]
  isLoading: boolean
  fileLoadProgress: FileLoadProgress | null
  loadError: string | null
  chatMode: boolean
  sessionName: string
  onQuery: (question: string) => void
  onStopQuery?: () => void
  onAddFiles: () => void
  onAddFolder: () => void
  onLoadPaths: (paths: string[]) => void
  onViewCitation: (citation: Citation) => void
  onExportChat?: () => void
  sessionId?: string
  practiceArea?: string | null
  chunkTexts?: string[]
  theme?: Theme
  onToggleTheme?: () => void
  onDeleteMessage?: (id: string) => void
  onRetryMessage?: (id: string) => void
  inferenceMode?: InferenceMode
  onInferenceModeChange?: (mode: InferenceMode) => void
}

// ── Inference mode dropdown (ChatGPT-style) ──────────────────────────────────

const MODE_OPTIONS: { key: InferenceMode; label: string; description: string; icon: JSX.Element }[] = [
  {
    key: 'quick',
    label: 'Brief',
    description: 'Faster, shorter answers',
    icon: <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round"><path d="M9.5 1L5 8h3.5L7 15l7-9H9.5L13 1z"/></svg>,
  },
  {
    key: 'balanced',
    label: 'Standard',
    description: 'Standard depth and detail',
    icon: <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round"><circle cx="8" cy="8" r="6"/><line x1="8" y1="5" x2="8" y2="8"/><line x1="8" y1="8" x2="10.5" y2="10"/></svg>,
  },
  {
    key: 'extended',
    label: 'Discovery',
    description: 'Comprehensive analysis',
    icon: <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round"><circle cx="7" cy="7" r="4.5"/><line x1="10.2" y1="10.2" x2="14" y2="14"/><line x1="7" y1="5" x2="7" y2="9"/><line x1="5" y1="7" x2="9" y2="7"/></svg>,
  },
]

function InferenceModeDropdown({
  value,
  onChange,
  disabled,
}: {
  value: InferenceMode
  onChange: (mode: InferenceMode) => void
  disabled?: boolean
}): JSX.Element {
  const [open, setOpen] = useState(false)
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    function handleClick(e: MouseEvent): void {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false)
    }
    document.addEventListener('mousedown', handleClick)
    return () => document.removeEventListener('mousedown', handleClick)
  }, [open])

  const current = MODE_OPTIONS.find((o) => o.key === value) ?? MODE_OPTIONS[1]

  return (
    <div ref={ref} className="relative no-drag">
      <button
        type="button"
        onClick={() => { if (!disabled) setOpen((v) => !v) }}
        className="flex items-center gap-1.5 rounded-lg px-2 py-1 transition-all"
        style={{
          color: 'var(--text)',
          background: open ? 'rgb(var(--ov) / 0.06)' : 'transparent',
          opacity: disabled ? 0.5 : 1,
          cursor: disabled ? 'not-allowed' : 'pointer',
        }}
        onMouseEnter={(e) => { if (!disabled && !open) (e.currentTarget as HTMLButtonElement).style.background = 'rgb(var(--ov) / 0.04)' }}
        onMouseLeave={(e) => { if (!open) (e.currentTarget as HTMLButtonElement).style.background = 'transparent' }}
      >
        <span className="text-[13px] font-semibold tracking-[-0.01em]">Justice AI</span>
        <span className="text-[10.5px] font-medium" style={{ color: 'rgba(201,168,76,0.7)' }}>{current.label}</span>
        <svg
          width="8" height="8" viewBox="0 0 10 10" fill="none"
          stroke="rgb(var(--ov) / 0.3)" strokeWidth="1.6" strokeLinecap="round"
          style={{ transform: open ? 'rotate(180deg)' : 'rotate(0deg)', transition: 'transform 0.15s ease' }}
        >
          <path d="M2 3.5l3 3 3-3" />
        </svg>
      </button>

      {open && (
        <div
          className="absolute left-0 top-full z-50 mt-1 rounded-xl py-1.5 w-[220px]"
          style={{
            background: 'var(--bg-alt)',
            border: '1px solid rgb(var(--ov) / 0.1)',
            boxShadow: '0 8px 32px rgba(0,0,0,0.35)',
          }}
        >
          {MODE_OPTIONS.map((opt) => {
            const active = opt.key === value
            return (
              <button
                key={opt.key}
                type="button"
                onClick={() => { onChange(opt.key); setOpen(false) }}
                className="w-full flex items-center gap-3 px-3 py-2 transition-colors text-left"
                style={{
                  background: active ? 'rgba(201,168,76,0.08)' : 'transparent',
                  color: active ? 'var(--gold)' : 'var(--text)',
                }}
                onMouseEnter={(e) => {
                  if (!active) (e.currentTarget as HTMLButtonElement).style.background = 'rgb(var(--ov) / 0.05)'
                }}
                onMouseLeave={(e) => {
                  (e.currentTarget as HTMLButtonElement).style.background = active ? 'rgba(201,168,76,0.08)' : 'transparent'
                }}
              >
                <div style={{ color: active ? 'var(--gold)' : 'rgb(var(--ov) / 0.4)' }}>
                  {opt.icon}
                </div>
                <div className="flex-1 min-w-0">
                  <p className="text-[12px] font-medium">{opt.label}</p>
                  <p className="text-[10.5px]" style={{ color: 'rgb(var(--ov) / 0.4)' }}>{opt.description}</p>
                </div>
                {active && (
                  <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="var(--gold)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M2 8l4 4 8-8" />
                  </svg>
                )}
              </button>
            )
          })}
        </div>
      )}
    </div>
  )
}

// ── Thinking animation ────────────────────────────────────────────────────────
const THINKING_PHRASES = [
  'Reviewing the record',
  'Consulting precedent',
  'Examining the evidence',
  'Cross-referencing exhibits',
  'Weighing relevant provisions',
  'Preparing counsel',
  'Deliberating on findings',
  'Drafting the opinion',
  'Approaching the bench',
  'Sustained… thinking',
  'Invoking stare decisis',
  'Checking the docket',
  'Filing a mental brief',
  'Conferring with co-counsel',
  'Entering chambers',
  'Swearing in the facts',
  'Polling the jury of neurons',
  'Reading the fine print',
  'Sequestering the evidence',
  'Calling an expert witness',
  'Sidebar conference in progress',
  'Requesting a brief recess',
  'Examining the witness',
  'Searching for loopholes',
  'Subpoenaing relevant facts',
  'Applying the reasonable AI standard',
  'Citing sources furiously',
  'Marshalling the arguments',
  'Deliberations are underway',
  'Per curiam processing',
  'Establishing chain of custody',
  'Redacting irrelevant thoughts',
  'Building the case',
  'Your honor, one moment',
  'Gaveling through the data',
  'Reviewing amicus briefs',
  'Motioning for more time',
]

function shuffled<T>(arr: T[]): T[] {
  const a = [...arr]
  for (let i = a.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1))
    ;[a[i], a[j]] = [a[j], a[i]]
  }
  return a
}

function TypingIndicator({ phase }: { phase?: string }): JSX.Element {
  const [phrases] = useState(() => shuffled(THINKING_PHRASES))
  const [phraseIdx, setPhraseIdx] = useState(0)
  const [elapsed, setElapsed] = useState(0)
  const [phraseKey, setPhraseKey] = useState(0)
  const startRef = useRef(Date.now())

  useEffect(() => {
    let phraseTimer: ReturnType<typeof setInterval> | undefined
    if (!phase) {
      phraseTimer = setInterval(() => {
        setPhraseIdx((i) => (i + 1) % phrases.length)
        setPhraseKey((k) => k + 1)
      }, 2800)
    }
    const elapsedTimer = setInterval(
      () => setElapsed(Math.floor((Date.now() - startRef.current) / 1000)),
      1000
    )
    return () => {
      if (phraseTimer) clearInterval(phraseTimer)
      clearInterval(elapsedTimer)
    }
  }, [phase])

  const displayPhrase = phase || phrases[phraseIdx]
  const displayKey = phase ? phase : phraseKey

  return (
    <div className="flex gap-3 max-w-4xl mx-auto w-full" style={{ animation: 'fadeUp 0.3s ease both' }} role="status" aria-live="polite">
      {/* Avatar */}
      <div
        className="flex h-7 w-7 shrink-0 mt-1 items-center justify-center rounded-full"
        style={{
          background: 'rgba(201,168,76,0.1)',
          border: '1px solid rgba(201,168,76,0.22)',
          animation: 'pulseGlow 2.4s ease-in-out infinite',
        }}
      >
        <svg width="12" height="12" viewBox="0 0 28 28" fill="none">
          <circle cx="14" cy="5" r="1.5" fill="#c9a84c" />
          <rect x="13.25" y="5" width="1.5" height="16" fill="#c9a84c" />
          <rect x="9" y="21" width="10" height="1.5" rx="0.75" fill="#c9a84c" />
          <rect x="12" y="22.5" width="4" height="1.5" rx="0.75" fill="#c9a84c" />
          <rect x="5" y="8.25" width="18" height="1.5" rx="0.75" fill="#c9a84c" />
          <line x1="7" y1="9.75" x2="5.5" y2="17" stroke="#c9a84c" strokeWidth="1.2" strokeLinecap="round" />
          <line x1="21" y1="9.75" x2="22.5" y2="17" stroke="#c9a84c" strokeWidth="1.2" strokeLinecap="round" />
          <path d="M3 17 Q5.5 20 8 17" stroke="#c9a84c" strokeWidth="1.3" fill="none" strokeLinecap="round" />
          <path d="M20 17 Q22.5 20 25 17" stroke="#c9a84c" strokeWidth="1.3" fill="none" strokeLinecap="round" />
        </svg>
      </div>

      {/* Card */}
      <div
        className="rounded-2xl rounded-tl-sm px-5 py-4 flex flex-col gap-3 relative overflow-hidden"
        style={{
          background: 'var(--bg-alt)',
          border: '1px solid rgba(201,168,76,0.14)',
          animation: 'pulseGlow 2.4s ease-in-out infinite',
          minWidth: 240,
          maxWidth: 340,
        }}
      >
        <p className="text-[11px] font-semibold" style={{ color: 'rgba(201,168,76,0.65)' }}>
          Justice AI
        </p>

        {/* Phrase + spinner row */}
        <div className="flex items-center gap-2.5">
          <div
            className="animate-spin shrink-0 w-3.5 h-3.5 rounded-full"
            style={{ border: '2px solid rgba(201,168,76,0.18)', borderTopColor: '#c9a84c' }}
          />
          <span
            key={displayKey}
            className="text-[13px]"
            style={{
              color: 'rgb(var(--ov) / 0.65)',
              animation: 'phraseIn 0.35s ease both',
            }}
          >
            {displayPhrase}
            <span className="loading-ellipsis" style={{ color: 'rgba(201,168,76,0.55)', fontWeight: 600 }} />
          </span>
        </div>

        {/* Scanning bar */}
        <div
          className="h-px w-full overflow-hidden rounded-full"
          style={{ background: 'rgb(var(--ov) / 0.05)' }}
        >
          <div
            style={{
              height: '100%',
              width: '30%',
              background: 'linear-gradient(90deg, transparent, rgba(201,168,76,0.55), transparent)',
              animation: 'scan 1.6s ease-in-out infinite',
            }}
          />
        </div>

        {/* Elapsed hint */}
        {elapsed >= 5 && (
          <p
            className="text-[10px]"
            style={{ color: 'rgb(var(--ov) / 0.45)', animation: 'fadeUp 0.4s ease both' }}
          >
            {elapsed < 60
              ? `${elapsed}s`
              : `${Math.floor(elapsed / 60)}m ${String(elapsed % 60).padStart(2, '0')}s`}{' '}
            · running locally — first query may take a minute or two
          </p>
        )}
      </div>
    </div>
  )
}

// ── Main component ────────────────────────────────────────────────────────────
export default function ChatInterface({
  messages,
  isQuerying,
  queryPhase,
  files,
  isLoading,
  fileLoadProgress,
  loadError,
  chatMode,
  sessionName,
  onQuery,
  onStopQuery,
  onAddFiles,
  onAddFolder,
  onLoadPaths,
  onViewCitation,
  onExportChat,
  sessionId,
  practiceArea,
  chunkTexts,
  theme,
  onToggleTheme,
  onDeleteMessage,
  onRetryMessage,
  inferenceMode = 'balanced',
  onInferenceModeChange,
}: Props): JSX.Element {
  const [input, setInput] = useState('')
  const [isDragging, setIsDragging] = useState(false)
  const [justLoaded, setJustLoaded] = useState(false)
  const [showHelp, setShowHelp] = useState(false)
  const [showScrollBtn, setShowScrollBtn] = useState(false)
  const prevIsLoadingRef = useRef(false)
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const scrollContainerRef = useRef<HTMLDivElement>(null)
  const textareaRef = useRef<HTMLTextAreaElement>(null)

  const hasFiles = files.length > 0

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages, isQuerying])

  useEffect(() => {
    const el = textareaRef.current
    if (!el) return
    el.style.height = 'auto'
    el.style.height = Math.min(el.scrollHeight, 200) + 'px'
  }, [input])

  // Escape key stops generation
  useEffect(() => {
    if (!isQuerying || !onStopQuery) return
    function handleEsc(e: globalThis.KeyboardEvent): void {
      if (e.key === 'Escape') onStopQuery!()
    }
    window.addEventListener('keydown', handleEsc)
    return () => window.removeEventListener('keydown', handleEsc)
  }, [isQuerying, onStopQuery])

  useEffect(() => {
    if (prevIsLoadingRef.current && !isLoading && hasFiles) {
      setJustLoaded(true)
      const t = setTimeout(() => setJustLoaded(false), 3000)
      return () => clearTimeout(t)
    }
    prevIsLoadingRef.current = isLoading
  }, [isLoading, hasFiles])

  function handleSend(): void {
    const trimmed = input.trim()
    if (!trimmed || isQuerying) return
    setInput('')
    onQuery(trimmed)
  }

  function handleKeyDown(e: KeyboardEvent<HTMLTextAreaElement>): void {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
  }

  // Tauri drag-and-drop via native webview events
  useEffect(() => {
    let unlisten: (() => void) | undefined
    getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === 'enter' || event.payload.type === 'over') {
        setIsDragging(true)
      } else if (event.payload.type === 'leave') {
        setIsDragging(false)
      } else if (event.payload.type === 'drop') {
        setIsDragging(false)
        const paths = event.payload.paths
        if (paths.length > 0) onLoadPaths(paths)
      }
    }).then((fn) => { unlisten = fn })
    return () => { unlisten?.() }
  }, [onLoadPaths])

  // ── CHAT VIEW ───────────────────────────────────────────────────────────────
  const isEmpty = messages.filter((m) => !m.isGreeting).length === 0

  return (
    <div
      className="flex flex-1 flex-col h-screen overflow-hidden relative"
      style={{ background: 'var(--bg)' }}
    >
      {/* Full-screen drag overlay */}
      {isDragging && (
        <div
          className="absolute inset-0 z-50 flex flex-col items-center justify-center pointer-events-none"
          style={{
            background: 'var(--backdrop)',
            backdropFilter: 'blur(4px)',
            WebkitBackdropFilter: 'blur(4px)',
            animation: 'fadeUp 0.15s ease both',
          }}
        >
          {/* Dashed frame inset from edges */}
          <div
            className="absolute"
            style={{
              inset: 20,
              borderRadius: 20,
              border: '1.5px dashed rgba(201,168,76,0.45)',
              background: 'rgba(201,168,76,0.03)',
              pointerEvents: 'none',
            }}
          />
          {/* Centered content */}
          <div className="flex flex-col items-center gap-3 relative z-10">
            <div
              className="flex h-16 w-16 items-center justify-center rounded-2xl"
              style={{
                background: 'rgba(201,168,76,0.1)',
                border: '1px solid rgba(201,168,76,0.3)',
                boxShadow: '0 0 32px rgba(201,168,76,0.12)',
              }}
            >
              <svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="#c9a84c" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                <polyline points="17 8 12 3 7 8" />
                <line x1="12" y1="3" x2="12" y2="15" />
              </svg>
            </div>
            <p className="text-[15px] font-semibold tracking-[-0.01em]" style={{ color: '#c9a84c' }}>
              Release to add documents
            </p>
            <p className="text-[12px]" style={{ color: 'rgba(201,168,76,0.5)' }}>
              PDF and DOCX supported
            </p>
          </div>
        </div>
      )}

      {/* Header — matches sidebar's h-11 traffic zone + content row */}
      <div
        className="drag-region flex h-11 items-center justify-between shrink-0 px-5"
        style={{ borderBottom: '1px solid rgb(var(--ov) / 0.05)' }}
      >
        <div className="no-drag flex items-center gap-3 min-w-0">
          {onInferenceModeChange ? (
            <InferenceModeDropdown value={inferenceMode} onChange={onInferenceModeChange} disabled={isQuerying} />
          ) : (
            <span className="text-[13px] font-semibold tracking-[-0.01em]" style={{ color: 'var(--text)' }}>Justice AI</span>
          )}
          {!isEmpty && (
            <span
              className="text-[11px] truncate"
              style={{ color: 'rgb(var(--ov) / 0.55)', maxWidth: 250 }}
            >
              {sessionName}
            </span>
          )}
        </div>

        <div className="no-drag flex items-center gap-2 shrink-0">
          {/* Inference mode toggle moved to top-left dropdown */}
          {/* Doc count pill */}
          {files.length > 0 && (
            <div
              className="flex items-center gap-1.5 rounded-lg px-2.5 py-1 text-[11px]"
              style={{
                border: justLoaded ? '1px solid rgba(201,168,76,0.4)' : '1px solid rgb(var(--ov) / 0.06)',
                color: justLoaded ? 'rgba(201,168,76,0.9)' : 'rgb(var(--ov) / 0.5)',
                background: justLoaded ? 'rgba(201,168,76,0.08)' : 'rgb(var(--ov) / 0.02)',
                transition: 'all 0.4s ease',
              }}
            >
              <svg width="9" height="9" viewBox="0 0 16 16" fill="rgba(201,168,76,0.5)">
                <path d="M2 1.75C2 .784 2.784 0 3.75 0h6.586c.464 0 .909.184 1.237.513l2.914 2.914c.329.328.513.773.513 1.237v9.586A1.75 1.75 0 0 1 13.25 16h-9.5A1.75 1.75 0 0 1 2 14.25z" />
              </svg>
              {files.length} {files.length === 1 ? 'doc' : 'docs'}
            </div>
          )}
          {/* Help button */}
          <button
            onClick={() => setShowHelp(true)}
            aria-label="Quick reference"
            className="flex items-center justify-center h-6 w-6 rounded-full text-[11px] font-bold transition-all"
            style={{
              border: '1px solid rgb(var(--ov) / 0.08)',
              color: 'rgb(var(--ov) / 0.5)',
              background: 'rgb(var(--ov) / 0.02)',
            }}
            onMouseEnter={(e) => {
              const el = e.currentTarget as HTMLButtonElement
              el.style.color = 'rgba(201,168,76,0.8)'
              el.style.borderColor = 'rgba(201,168,76,0.25)'
              el.style.background = 'rgba(201,168,76,0.06)'
            }}
            onMouseLeave={(e) => {
              const el = e.currentTarget as HTMLButtonElement
              el.style.color = 'rgb(var(--ov) / 0.5)'
              el.style.borderColor = 'rgb(var(--ov) / 0.08)'
              el.style.background = 'rgb(var(--ov) / 0.02)'
            }}
          >
            ?
          </button>
          {/* Export button */}
          {onExportChat && !isEmpty && (
            <button
              onClick={onExportChat}
              title="Export conversation"
              aria-label="Export conversation"
              className="flex items-center gap-1.5 rounded-lg px-2.5 py-1 text-[11px] transition-all"
              style={{ border: '1px solid rgb(var(--ov) / 0.06)', color: 'rgb(var(--ov) / 0.5)', background: 'rgb(var(--ov) / 0.02)' }}
              onMouseEnter={(e) => {
                const el = e.currentTarget as HTMLButtonElement
                el.style.color = 'rgba(201,168,76,0.75)'
                el.style.borderColor = 'rgba(201,168,76,0.22)'
              }}
              onMouseLeave={(e) => {
                const el = e.currentTarget as HTMLButtonElement
                el.style.color = 'rgb(var(--ov) / 0.28)'
                el.style.borderColor = 'rgb(var(--ov) / 0.06)'
              }}
            >
              <svg width="9" height="9" viewBox="0 0 16 16" fill="currentColor">
                <path d="M2.75 14A1.75 1.75 0 0 1 1 12.25v-2.5a.75.75 0 0 1 1.5 0v2.5c0 .138.112.25.25.25h10.5a.25.25 0 0 0 .25-.25v-2.5a.75.75 0 0 1 1.5 0v2.5A1.75 1.75 0 0 1 13.25 14ZM7.25 7.689V2a.75.75 0 0 1 1.5 0v5.689l1.97-1.97a.749.749 0 1 1 1.06 1.06l-3.25 3.25a.749.749 0 0 1-1.06 0L4.22 6.779a.749.749 0 1 1 1.06-1.06l1.97 1.97Z" />
              </svg>
              Export
            </button>
          )}
        </div>
      </div>

      {/* Messages */}
      <div
        key={sessionId}
        ref={scrollContainerRef}
        className="flex-1 overflow-y-auto relative"
        style={{ animation: 'sessionFade 0.25s ease both' }}
        onScroll={(e) => {
          const el = e.currentTarget
          const distFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight
          setShowScrollBtn(distFromBottom > 200)
        }}
      >
        {isEmpty ? (
          <div className="flex h-full flex-col items-center justify-center px-8 py-16 text-center relative">
            {!hasFiles ? (
              /* No docs yet — welcome state with chat encouragement */
              <>
                {/* Court seal watermark */}
                <svg
                  width="260" height="260" viewBox="0 0 200 200" fill="none"
                  className="absolute pointer-events-none select-none"
                  style={{ opacity: 0.025, top: '50%', left: '50%', transform: 'translate(-50%, -50%)' }}
                >
                  <circle cx="100" cy="100" r="95" stroke="currentColor" strokeWidth="2.5" />
                  <circle cx="100" cy="100" r="82" stroke="currentColor" strokeWidth="1" />
                  <circle cx="100" cy="52" r="3" fill="currentColor" />
                  <rect x="98.5" y="52" width="3" height="48" fill="currentColor" />
                  <rect x="86" y="100" width="28" height="3" rx="1.5" fill="currentColor" />
                  <rect x="92" y="103" width="16" height="3" rx="1.5" fill="currentColor" />
                  <rect x="70" y="58" width="60" height="3" rx="1.5" fill="currentColor" />
                  <line x1="76" y1="61" x2="73" y2="82" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" />
                  <line x1="124" y1="61" x2="127" y2="82" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" />
                  <path d="M67 82 Q73 92 79 82" stroke="currentColor" strokeWidth="2.5" fill="none" strokeLinecap="round" />
                  <path d="M121 82 Q127 92 133 82" stroke="currentColor" strokeWidth="2.5" fill="none" strokeLinecap="round" />
                  <path d="M38 140 Q30 120 42 100" stroke="currentColor" strokeWidth="1.5" fill="none" />
                  <path d="M42 148 Q32 130 40 112" stroke="currentColor" strokeWidth="1.5" fill="none" />
                  <path d="M48 155 Q36 140 42 122" stroke="currentColor" strokeWidth="1.5" fill="none" />
                  <path d="M56 160 Q42 148 46 132" stroke="currentColor" strokeWidth="1.5" fill="none" />
                  <path d="M65 163 Q52 155 54 140" stroke="currentColor" strokeWidth="1.5" fill="none" />
                  <path d="M162 140 Q170 120 158 100" stroke="currentColor" strokeWidth="1.5" fill="none" />
                  <path d="M158 148 Q168 130 160 112" stroke="currentColor" strokeWidth="1.5" fill="none" />
                  <path d="M152 155 Q164 140 158 122" stroke="currentColor" strokeWidth="1.5" fill="none" />
                  <path d="M144 160 Q158 148 154 132" stroke="currentColor" strokeWidth="1.5" fill="none" />
                  <path d="M135 163 Q148 155 146 140" stroke="currentColor" strokeWidth="1.5" fill="none" />
                  <text textAnchor="middle" fontSize="10" fontWeight="600" letterSpacing="4" fill="currentColor">
                    <textPath href="#sealArcTop" startOffset="50%">JUSTICE AI</textPath>
                  </text>
                  <defs>
                    <path id="sealArcTop" d="M30 100 A70 70 0 0 1 170 100" />
                  </defs>
                </svg>

                <div
                  className="mb-5 flex h-14 w-14 items-center justify-center rounded-2xl relative z-10"
                  style={{
                    background: 'rgba(201,168,76,0.07)',
                    border: '1px solid rgba(201,168,76,0.2)',
                    boxShadow: '0 4px 20px rgba(201,168,76,0.06)',
                  }}
                >
                  <svg width="24" height="24" viewBox="0 0 28 28" fill="none">
                    <circle cx="14" cy="5" r="1.5" fill="#c9a84c" />
                    <rect x="13.25" y="5" width="1.5" height="16" fill="#c9a84c" />
                    <rect x="9" y="21" width="10" height="1.5" rx="0.75" fill="#c9a84c" />
                    <rect x="12" y="22.5" width="4" height="1.5" rx="0.75" fill="#c9a84c" />
                    <rect x="5" y="8.25" width="18" height="1.5" rx="0.75" fill="#c9a84c" />
                    <line x1="7" y1="9.75" x2="5.5" y2="17" stroke="#c9a84c" strokeWidth="1.2" strokeLinecap="round" />
                    <line x1="21" y1="9.75" x2="22.5" y2="17" stroke="#c9a84c" strokeWidth="1.2" strokeLinecap="round" />
                    <path d="M3 17 Q5.5 20 8 17" stroke="#c9a84c" strokeWidth="1.3" fill="none" strokeLinecap="round" />
                    <path d="M20 17 Q22.5 20 25 17" stroke="#c9a84c" strokeWidth="1.3" fill="none" strokeLinecap="round" />
                  </svg>
                </div>
                <h3 className="mb-2 text-[22px] font-bold tracking-[-0.03em] relative z-10" style={{ color: 'var(--text)' }}>
                  Justice <span style={{ color: 'var(--gold)' }}>AI</span>
                </h3>
                <p className="mb-6 text-[13px] leading-relaxed relative z-10" style={{ color: 'rgb(var(--ov) / 0.45)', maxWidth: 340 }}>
                  {new Date().getHours() < 12 ? 'Good morning, counselor.' : new Date().getHours() < 17 ? 'Good afternoon, counselor.' : 'Good evening, counselor.'}{' '}
                  Ask anything — or add documents for cited answers.
                </p>

                {/* Suggestion chips */}
                <div className="flex flex-wrap justify-center gap-2 mb-6 relative z-10" style={{ maxWidth: 440 }}>
                  {[
                    'What is Justice AI?',
                    'How do I add documents?',
                    'What file types are supported?',
                  ].map((q) => (
                    <button
                      key={q}
                      onClick={() => { setInput(q); textareaRef.current?.focus() }}
                      className="px-3.5 py-2 rounded-xl text-[12px] transition-all"
                      style={{
                        background: 'rgb(var(--ov) / 0.03)',
                        border: '1px solid rgb(var(--ov) / 0.08)',
                        color: 'rgb(var(--ov) / 0.5)',
                      }}
                      onMouseEnter={(e) => {
                        const el = e.currentTarget as HTMLButtonElement
                        el.style.borderColor = 'rgba(201,168,76,0.3)'
                        el.style.color = 'rgba(201,168,76,0.8)'
                        el.style.background = 'rgba(201,168,76,0.04)'
                      }}
                      onMouseLeave={(e) => {
                        const el = e.currentTarget as HTMLButtonElement
                        el.style.borderColor = 'rgb(var(--ov) / 0.08)'
                        el.style.color = 'rgb(var(--ov) / 0.5)'
                        el.style.background = 'rgb(var(--ov) / 0.03)'
                      }}
                    >
                      {q}
                    </button>
                  ))}
                </div>

                {/* Add documents prompt */}
                <button
                  onClick={onAddFiles}
                  className="flex items-center gap-2 px-5 py-2.5 rounded-xl text-[12.5px] font-semibold relative z-10 transition-all"
                  style={{
                    background: 'rgba(201,168,76,0.08)',
                    border: '1px solid rgba(201,168,76,0.2)',
                    color: 'rgba(201,168,76,0.8)',
                  }}
                  onMouseEnter={(e) => {
                    const el = e.currentTarget as HTMLButtonElement
                    el.style.background = 'rgba(201,168,76,0.14)'
                    el.style.borderColor = 'rgba(201,168,76,0.35)'
                  }}
                  onMouseLeave={(e) => {
                    const el = e.currentTarget as HTMLButtonElement
                    el.style.background = 'rgba(201,168,76,0.08)'
                    el.style.borderColor = 'rgba(201,168,76,0.2)'
                  }}
                >
                  <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                    <polyline points="17 8 12 3 7 8" />
                    <line x1="12" y1="3" x2="12" y2="15" />
                  </svg>
                  Add documents for cited answers
                </button>
                <p className="mt-2.5 text-[11px] relative z-10" style={{ color: 'rgb(var(--ov) / 0.5)' }}>
                  Supports PDFs, Word, Excel, images,{' '}
                  <span
                    className="inline-block cursor-help relative group/tip"
                    style={{
                      color: 'rgba(201,168,76,0.5)',
                      borderBottom: '1px dashed rgba(201,168,76,0.25)',
                    }}
                  >
                    and more
                    <span
                      className="pointer-events-none absolute left-1/2 -translate-x-1/2 bottom-full mb-1.5 opacity-0 group-hover/tip:opacity-100 transition-opacity duration-150 whitespace-nowrap rounded-md px-2.5 py-1.5 text-[10px] font-medium z-10"
                      style={{
                        background: 'var(--bg-alt)',
                        border: '1px solid rgb(var(--ov) / 0.12)',
                        color: 'rgb(var(--ov) / 0.6)',
                        boxShadow: '0 4px 12px rgba(0,0,0,0.3)',
                      }}
                    >
                      TXT, MD, CSV, HTML, XML, EML, PNG, JPG, TIFF
                    </span>
                  </span>
                </p>
              </>
            ) : (
              /* Has docs — show template prompts + key facts */
              <>
                <p className="mb-1.5 text-[10.5px] font-semibold uppercase tracking-[0.2em]" style={{ color: 'rgba(201,168,76,0.5)' }}>
                  {files.length} {files.length === 1 ? 'exhibit' : 'exhibits'} on file
                </p>
                <h3 className="mb-5 text-[22px] font-semibold tracking-[-0.02em]" style={{ color: 'var(--text)' }}>
                  Court is in session
                </h3>
                {chunkTexts && chunkTexts.length > 0 && (
                  <div className="w-full max-w-lg mb-5">
                    <FactsPanel
                      chunkTexts={chunkTexts}
                      onClickFact={(q) => { setInput(q); textareaRef.current?.focus() }}
                    />
                  </div>
                )}
                <QueryTemplates
                  practiceArea={practiceArea ?? null}
                  onSelect={(q) => { setInput(q); textareaRef.current?.focus() }}
                />
              </>
            )}
          </div>
        ) : (
          <div role="log" aria-live="polite" className="flex flex-col gap-7 max-w-4xl mx-auto w-full px-6 py-8 pb-10">
            {messages.map((msg, idx) => {
              const isLastAssistant = msg.role === 'assistant' && !msg.isStreaming &&
                !messages.slice(idx + 1).some((m) => m.role === 'assistant' && !m.isStreaming)

              // Date separator between days
              let dateSep: JSX.Element | null = null
              if (msg.timestamp) {
                const d = new Date(msg.timestamp)
                const prev = idx > 0 ? messages[idx - 1] : null
                const prevD = prev?.timestamp ? new Date(prev.timestamp) : null
                if (!prevD || d.toDateString() !== prevD.toDateString()) {
                  const now = new Date()
                  const yesterday = new Date(now); yesterday.setDate(yesterday.getDate() - 1)
                  const label = d.toDateString() === now.toDateString() ? 'Today'
                    : d.toDateString() === yesterday.toDateString() ? 'Yesterday'
                    : d.toLocaleDateString([], { weekday: 'long', month: 'short', day: 'numeric' })
                  dateSep = (
                    <div key={`sep-${msg.id}`} className="flex items-center gap-3 my-1">
                      <div className="flex-1 h-px" style={{ background: 'rgb(var(--ov) / 0.06)' }} />
                      <span className="text-[10px] font-medium tracking-wide" style={{ color: 'rgb(var(--ov) / 0.5)' }}>{label}</span>
                      <div className="flex-1 h-px" style={{ background: 'rgb(var(--ov) / 0.06)' }} />
                    </div>
                  )
                }
              }

              return (
                <React.Fragment key={msg.id}>
                  {dateSep}
                  <MessageBubble
                    message={msg}
                    files={files}
                    onViewCitation={onViewCitation}
                    onDeleteMessage={onDeleteMessage}
                    onRetryMessage={onRetryMessage}
                    isLastAssistant={isLastAssistant}
                  />
                </React.Fragment>
              )
            })}
            {isQuerying && !messages.some((m) => m.isStreaming && m.content.length > 0) && (
              <TypingIndicator phase={queryPhase} />
            )}
            <div ref={messagesEndRef} className="h-4" />
          </div>
        )}

        {/* Scroll to bottom button */}
        {showScrollBtn && !isEmpty && (
          <button
            onClick={() => messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })}
            aria-label="Scroll to bottom"
            className="absolute left-1/2 -translate-x-1/2 bottom-4 z-10 flex items-center justify-center h-8 w-8 rounded-full transition-all"
            style={{
              background: 'var(--bg-alt)',
              border: '1px solid rgb(var(--ov) / 0.12)',
              boxShadow: '0 4px 12px rgba(0,0,0,0.25)',
              color: '#c9a84c',
              animation: 'fadeUp 0.2s ease both',
            }}
          >
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M8 3v10M4 9l4 4 4-4" />
            </svg>
          </button>
        )}
      </div>

      {/* Input */}
      <div
        className="shrink-0 px-6 py-4"
        style={{ borderTop: '1px solid rgb(var(--ov) / 0.05)', background: 'var(--bg)' }}
      >
        <div className="max-w-4xl mx-auto">
          {fileLoadProgress && (
            <div
              className="flex items-center gap-3 rounded-xl px-4 py-2.5 mb-3"
              style={{
                background: 'rgba(201,168,76,0.06)',
                border: '1px solid rgba(201,168,76,0.18)',
                animation: 'fadeUp 0.25s ease both',
              }}
            >
              <div className="shrink-0">
                <svg width="14" height="14" viewBox="0 0 16 16" fill="none" className="animate-spin" style={{ animationDuration: '1.5s' }}>
                  <circle cx="8" cy="8" r="6" stroke="rgba(201,168,76,0.3)" strokeWidth="2" />
                  <path d="M14 8a6 6 0 0 0-6-6" stroke="rgba(201,168,76,0.8)" strokeWidth="2" strokeLinecap="round" />
                </svg>
              </div>
              <div className="flex-1 min-w-0">
                <div className="flex items-center justify-between mb-1">
                  <span className="text-[11px] font-medium truncate" style={{ color: 'rgba(201,168,76,0.9)' }}>
                    {fileLoadProgress.totalFiles > 1
                      ? `${fileLoadProgress.fileName} (${fileLoadProgress.fileIndex + 1}/${fileLoadProgress.totalFiles})`
                      : fileLoadProgress.fileName}
                  </span>
                  <span className="text-[10px] ml-2 shrink-0" style={{ color: 'rgba(201,168,76,0.6)' }}>
                    {{
                      parsing: 'Parsing…',
                      analyzing: 'Analyzing…',
                      chunking: 'Chunking…',
                      embedding: fileLoadProgress.chunkCount
                        ? `Embedding ${fileLoadProgress.chunkCount} chunks…`
                        : 'Embedding…',
                      saving: 'Saving…',
                      complete: 'Done',
                    }[fileLoadProgress.stage]}
                  </span>
                </div>
                <div className="h-1 rounded-full overflow-hidden" style={{ background: 'rgba(201,168,76,0.12)' }}>
                  <div
                    className="h-full rounded-full"
                    style={{
                      background: 'rgba(201,168,76,0.7)',
                      transition: 'width 0.3s ease',
                      width: (() => {
                        const stageWeights: Record<string, number> = { parsing: 15, analyzing: 30, chunking: 45, embedding: 80, saving: 95, complete: 100 }
                        const stagePercent = stageWeights[fileLoadProgress.stage] ?? 0
                        if (fileLoadProgress.totalFiles <= 1) return `${stagePercent}%`
                        const filePercent = (fileLoadProgress.fileIndex / fileLoadProgress.totalFiles) * 100
                        const perFile = 100 / fileLoadProgress.totalFiles
                        return `${filePercent + (stagePercent / 100) * perFile}%`
                      })(),
                    }}
                  />
                </div>
              </div>
            </div>
          )}
          {loadError && (
            <div
              role="alert"
              className="flex items-start gap-2.5 rounded-xl px-4 py-3 mb-3"
              style={{
                background: 'rgba(201,168,76,0.06)',
                border: '1px solid rgba(201,168,76,0.28)',
                borderLeft: '2px solid rgba(201,168,76,0.7)',
                animation: 'fadeUp 0.25s ease both',
              }}
            >
              <svg width="13" height="13" viewBox="0 0 16 16" fill="none" className="shrink-0 mt-0.5">
                <path d="M8.22 1.754a.25.25 0 0 0-.44 0L1.698 13.132a.25.25 0 0 0 .22.368h12.164a.25.25 0 0 0 .22-.368L8.22 1.754zm-1.358-.29a1.75 1.75 0 0 1 3.076 0l6.082 11.378A1.75 1.75 0 0 1 14.082 15.5H1.918a1.75 1.75 0 0 1-1.538-2.658L6.862 1.464z" fill="rgba(201,168,76,0.75)" />
                <path d="M9 11a1 1 0 1 1-2 0 1 1 0 0 1 2 0zm-.25-5.25a.75.75 0 0 0-1.5 0v2.5a.75.75 0 0 0 1.5 0v-2.5z" fill="rgba(201,168,76,0.75)" />
              </svg>
              <p className="text-[12px] leading-relaxed" style={{ color: 'rgba(201,168,76,0.9)' }}>
                {loadError}
              </p>
            </div>
          )}
          <div
            className="rounded-2xl input-focus-ring"
            style={{
              background: 'var(--bg-alt)',
              border: '1px solid rgb(var(--ov) / 0.08)',
              boxShadow: '0 -1px 12px var(--shadow)',
            }}
          >
            <div className="flex items-center gap-3 px-4 py-3.5">
              <textarea
                ref={textareaRef}
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder={isQuerying ? 'Generating… press Esc to stop' : 'Ask about your documents…    Shift + Enter for new line'}
                rows={1}
                className="flex-1 bg-transparent text-[13.5px] leading-6 outline-none placeholder:text-[var(--placeholder)] disabled:opacity-50"
                style={{ maxHeight: 200, overflowY: 'auto', resize: 'none', color: 'var(--text)', transition: 'height 0.1s ease' }}
              />
              {isQuerying ? (
                <button
                  onClick={onStopQuery}
                  title="Stop generating"
                  aria-label="Stop generating"
                  className="flex shrink-0 h-8 w-8 items-center justify-center rounded-xl transition-all"
                  style={{
                    background: 'rgba(248,81,73,0.12)',
                    border: '1px solid rgba(248,81,73,0.25)',
                    color: '#f85149',
                  }}
                >
                  <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
                    <rect x="3" y="3" width="10" height="10" rx="1.5" />
                  </svg>
                </button>
              ) : (
                <button
                  onClick={handleSend}
                  disabled={!input.trim()}
                  title="Send message"
                  aria-label="Send message"
                  className="flex shrink-0 h-8 w-8 items-center justify-center rounded-xl disabled:opacity-30 disabled:cursor-not-allowed active:scale-95"
                  style={{
                    background: input.trim() ? 'var(--gold)' : 'rgb(var(--ov) / 0.06)',
                    color: input.trim() ? 'var(--text-on-gold)' : 'rgb(var(--ov) / 0.5)',
                    transition: 'background 0.2s ease, color 0.2s ease, box-shadow 0.2s ease, transform 0.1s ease',
                    boxShadow: input.trim() ? 'var(--shadow)' : 'none',
                  }}
                >
                  <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
                    <path d="M.989 8 .064 2.68a1.342 1.342 0 0 1 1.85-1.462l13.402 5.744a1.13 1.13 0 0 1 0 2.076L1.913 14.782a1.342 1.342 0 0 1-1.85-1.463L.99 8zm.603-5.135.024.12L2.15 7.25h6.848a.75.75 0 0 1 0 1.5H2.15l-.534 4.265-.024.12 13.016-5.577z" />
                  </svg>
                </button>
              )}
            </div>
          </div>
          <div className="mt-2 flex items-center justify-center gap-1.5">
            <svg width="8" height="8" viewBox="0 0 28 28" fill="none">
              <circle cx="14" cy="5" r="1.5" fill="rgba(201,168,76,0.25)" />
              <rect x="13.25" y="5" width="1.5" height="16" fill="rgba(201,168,76,0.25)" />
              <rect x="9" y="21" width="10" height="1.5" rx="0.75" fill="rgba(201,168,76,0.25)" />
              <rect x="5" y="8.25" width="18" height="1.5" rx="0.75" fill="rgba(201,168,76,0.25)" />
              <line x1="7" y1="9.75" x2="5.5" y2="17" stroke="rgba(201,168,76,0.25)" strokeWidth="1.2" strokeLinecap="round" />
              <line x1="21" y1="9.75" x2="22.5" y2="17" stroke="rgba(201,168,76,0.25)" strokeWidth="1.2" strokeLinecap="round" />
              <path d="M3 17 Q5.5 20 8 17" stroke="rgba(201,168,76,0.25)" strokeWidth="1.3" fill="none" strokeLinecap="round" />
              <path d="M20 17 Q22.5 20 25 17" stroke="rgba(201,168,76,0.25)" strokeWidth="1.3" fill="none" strokeLinecap="round" />
            </svg>
            <p className="text-[10px] tracking-wide" style={{ color: 'rgb(var(--ov) / 0.45)' }}>
              Justice AI · Enter to send · Answers grounded in your documents
            </p>
          </div>
        </div>
      </div>

      {/* Help modal */}
      {showHelp && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center"
          style={{ background: 'var(--backdrop)', backdropFilter: 'blur(8px)' }}
          onClick={() => setShowHelp(false)}
        >
          <div
            role="dialog"
            aria-modal="true"
            aria-label="Quick Reference"
            className="w-full max-w-md rounded-2xl overflow-hidden"
            style={{
              background: 'var(--modal-bg)',
              border: '1px solid rgb(var(--ov) / 0.08)',
              boxShadow: '0 40px 100px var(--shadow-heavy), 0 0 0 1px rgb(var(--ov) / 0.03)',
              animation: 'scaleIn 0.2s ease both',
            }}
            onClick={(e) => e.stopPropagation()}
          >
            {/* Header */}
            <div
              className="flex items-center justify-between px-6 py-4"
              style={{ borderBottom: '1px solid rgb(var(--ov) / 0.06)' }}
            >
              <h2 className="text-[14px] font-semibold" style={{ color: 'var(--text)' }}>Quick Reference</h2>
              <button
                onClick={() => setShowHelp(false)}
                aria-label="Close help"
                className="flex h-7 w-7 items-center justify-center rounded-lg transition-colors"
                style={{ color: 'rgb(var(--ov) / 0.5)' }}
                onMouseEnter={(e) => { (e.currentTarget as HTMLButtonElement).style.color = 'rgb(var(--ov) / 0.7)' }}
                onMouseLeave={(e) => { (e.currentTarget as HTMLButtonElement).style.color = 'rgb(var(--ov) / 0.3)' }}
              >
                <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
                  <path d="M3.72 3.72a.75.75 0 0 1 1.06 0L8 6.94l3.22-3.22a.75.75 0 1 1 1.06 1.06L9.06 8l3.22 3.22a.75.75 0 1 1-1.06 1.06L8 9.06l-3.22 3.22a.75.75 0 0 1-1.06-1.06L6.94 8 3.72 4.78a.75.75 0 0 1 0-1.06z" />
                </svg>
              </button>
            </div>

            {/* Body */}
            <div className="px-6 py-5 flex flex-col gap-5">
              {/* Keyboard shortcuts */}
              <div>
                <h3
                  className="text-[10px] font-semibold uppercase tracking-[0.12em] mb-3 pb-2 border-b"
                  style={{ color: 'rgb(var(--ov) / 0.5)', borderColor: 'rgb(var(--ov) / 0.06)' }}
                >
                  Keyboard Shortcuts
                </h3>
                <div className="flex flex-col gap-2">
                  {[
                    { keys: 'Enter', desc: 'Send message' },
                    { keys: 'Shift + Enter', desc: 'New line' },
                    { keys: 'Esc', desc: 'Close settings / dialogs' },
                    { keys: navigator.platform?.includes('Mac') ? '\u2318K' : 'Ctrl+K', desc: 'Command palette' },
                    { keys: 'Drag & Drop', desc: 'Add files to your session' },
                  ].map((item) => (
                    <div key={item.keys} className="flex items-center justify-between">
                      <span
                        className="text-[11px] font-mono px-2 py-0.5 rounded"
                        style={{ background: 'rgb(var(--ov) / 0.05)', color: 'rgba(201,168,76,0.7)', border: '1px solid rgb(var(--ov) / 0.08)' }}
                      >
                        {item.keys}
                      </span>
                      <span className="text-[12px]" style={{ color: 'rgb(var(--ov) / 0.5)' }}>{item.desc}</span>
                    </div>
                  ))}
                </div>
              </div>

              {/* Usage tips */}
              <div>
                <h3
                  className="text-[10px] font-semibold uppercase tracking-[0.12em] mb-3 pb-2 border-b"
                  style={{ color: 'rgb(var(--ov) / 0.5)', borderColor: 'rgb(var(--ov) / 0.06)' }}
                >
                  Usage Tips
                </h3>
                <ul className="flex flex-col gap-2">
                  {[
                    'Load your documents first. Supports PDF, Word, Excel, images, TXT, CSV, HTML, EML, and more.',
                    'Ask questions in plain, natural language.',
                    'Citations link directly to the source page in your documents.',
                    'Use cases to organize research across different matters.',
                  ].map((tip) => (
                    <li key={tip} className="flex items-start gap-2">
                      <span className="mt-1.5 w-1 h-1 rounded-full shrink-0" style={{ background: 'rgba(201,168,76,0.5)' }} />
                      <span className="text-[12px] leading-relaxed" style={{ color: 'rgb(var(--ov) / 0.5)' }}>{tip}</span>
                    </li>
                  ))}
                </ul>
              </div>
            </div>

            {/* Footer */}
            <div
              className="flex justify-end px-6 py-4"
              style={{ borderTop: '1px solid rgb(var(--ov) / 0.06)' }}
            >
              <button
                onClick={() => setShowHelp(false)}
                className="rounded-lg px-5 py-2 text-[12px] font-semibold transition-colors"
                style={{ background: 'var(--gold)', color: 'var(--text-on-gold)' }}
              >
                Got it
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

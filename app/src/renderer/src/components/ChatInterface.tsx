import { useEffect, useRef, useState, KeyboardEvent } from 'react'
import { ChatMessage, Citation, FileInfo } from '../../../../../shared/src/types'
import MessageBubble from './MessageBubble'

interface Props {
  messages: ChatMessage[]
  isQuerying: boolean
  files: FileInfo[]
  isLoading: boolean
  loadError: string | null
  chatMode: boolean
  sessionName: string
  onQuery: (question: string) => void
  onNewChat: () => void
  onAddFiles: () => void
  onAddFolder: () => void
  onRemoveFile: (id: string) => void
  onLoadPaths: (paths: string[]) => void
  onViewCitation: (citation: Citation) => void
}

// ── Thinking animation ────────────────────────────────────────────────────────
const THINKING_PHRASES = [
  'Reading your documents',
  'Finding relevant sections',
  'Cross-referencing passages',
  'Analyzing legal context',
  'Weighing relevant statutes',
  'Synthesizing key findings',
  'Building your answer',
  'Reviewing source citations',
]

function TypingIndicator(): JSX.Element {
  const [phraseIdx, setPhraseIdx] = useState(0)
  const [dotTick, setDotTick] = useState(0)
  const [elapsed, setElapsed] = useState(0)
  const [phraseKey, setPhraseKey] = useState(0)
  const startRef = useRef(Date.now())

  useEffect(() => {
    const phraseTimer = setInterval(() => {
      setPhraseIdx((i) => (i + 1) % THINKING_PHRASES.length)
      setPhraseKey((k) => k + 1)
    }, 2800)
    const dotsTimer = setInterval(() => setDotTick((t) => t + 1), 500)
    const elapsedTimer = setInterval(() => {
      setElapsed(Math.floor((Date.now() - startRef.current) / 1000))
    }, 1000)
    return () => {
      clearInterval(phraseTimer)
      clearInterval(dotsTimer)
      clearInterval(elapsedTimer)
    }
  }, [])

  const dots = ['·', '··', '···'][dotTick % 3]

  return (
    <div className="flex gap-3 max-w-3xl mx-auto w-full" style={{ animation: 'fadeUp 0.3s ease both' }}>
      {/* Avatar */}
      <div
        className="flex h-7 w-7 shrink-0 mt-1 items-center justify-center rounded-full"
        style={{
          background: 'rgba(201,168,76,0.1)',
          border: '1px solid rgba(201,168,76,0.22)',
          animation: 'pulseGlow 2.4s ease-in-out infinite',
        }}
      >
        <svg width="12" height="12" viewBox="0 0 20 20" fill="none">
          <rect x="1" y="3" width="11" height="4" rx="1.25" fill="#c9a84c" transform="rotate(45 6.5 5)" />
          <line x1="10.5" y1="10.5" x2="18.5" y2="18.5" stroke="#c9a84c" strokeWidth="2.5" strokeLinecap="round" />
          <rect x="0.5" y="16.5" width="8.5" height="2.5" rx="0.75" fill="#c9a84c" opacity="0.38" />
        </svg>
      </div>

      {/* Card */}
      <div
        className="rounded-2xl rounded-tl-sm px-5 py-4 flex flex-col gap-3 relative overflow-hidden"
        style={{
          background: '#0f0f0f',
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
            key={phraseKey}
            className="text-[13px]"
            style={{
              color: 'rgba(255,255,255,0.65)',
              animation: 'phraseIn 0.35s ease both',
            }}
          >
            {THINKING_PHRASES[phraseIdx]}
            <span style={{ color: 'rgba(201,168,76,0.55)', fontWeight: 600, letterSpacing: '0.05em' }}>
              {dots}
            </span>
          </span>
        </div>

        {/* Scanning bar */}
        <div
          className="h-px w-full overflow-hidden rounded-full"
          style={{ background: 'rgba(255,255,255,0.05)' }}
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
            style={{ color: 'rgba(255,255,255,0.18)', animation: 'fadeUp 0.4s ease both' }}
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

// ── Example questions ────────────────────────────────────────────────────────
const EXAMPLES = [
  'What are the key terms and obligations in this contract?',
  'Summarize the liability limitations and indemnification clauses',
  'Find all deadlines and notice requirements in the agreement',
  'What damages or remedies does this document contemplate?',
  'Identify any confidentiality or non-compete provisions',
  'What conditions must be met for termination of this agreement?',
]

// ── Main component ────────────────────────────────────────────────────────────
export default function ChatInterface({
  messages,
  isQuerying,
  files,
  isLoading,
  loadError,
  chatMode,
  sessionName,
  onQuery,
  onAddFiles,
  onAddFolder,
  onLoadPaths,
  onViewCitation,
}: Props): JSX.Element {
  const [input, setInput] = useState('')
  const [isDragging, setIsDragging] = useState(false)
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const textareaRef = useRef<HTMLTextAreaElement>(null)

  const hasFiles = files.length > 0

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages, isQuerying])

  useEffect(() => {
    const el = textareaRef.current
    if (!el) return
    el.style.height = 'auto'
    el.style.height = Math.min(el.scrollHeight, 128) + 'px'
  }, [input])

  function handleSend(): void {
    const trimmed = input.trim()
    if (!trimmed || isQuerying || !hasFiles) return
    setInput('')
    onQuery(trimmed)
  }

  function handleKeyDown(e: KeyboardEvent<HTMLTextAreaElement>): void {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
  }

  function handleDragOver(e: React.DragEvent<HTMLDivElement>): void {
    e.preventDefault()
    setIsDragging(true)
  }

  function handleDragLeave(e: React.DragEvent<HTMLDivElement>): void {
    e.preventDefault()
    setIsDragging(false)
  }

  function handleDrop(e: React.DragEvent<HTMLDivElement>): void {
    e.preventDefault()
    setIsDragging(false)
    const paths: string[] = []
    for (let i = 0; i < e.dataTransfer.files.length; i++) {
      const f = e.dataTransfer.files[i] as File & { path?: string }
      if (f.path) paths.push(f.path)
    }
    if (paths.length > 0) onLoadPaths(paths)
  }

  // ── WELCOME SCREEN ──────────────────────────────────────────────────────────
  if (!hasFiles && !chatMode) {
    return (
      <div
        className="flex flex-1 flex-col h-screen overflow-hidden bg-[#080808]"
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
      >
        {/* Title bar drag zone */}
        <div className="drag-region h-11 shrink-0" />

        <div className="flex-1 flex flex-col items-center justify-center px-10 pb-16">

          {/* Ambient glow behind icon */}
          <div className="relative mb-7">
            <div
              className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-40 h-40 rounded-full pointer-events-none"
              style={{ background: 'radial-gradient(circle, rgba(201,168,76,0.12) 0%, transparent 70%)' }}
            />
            <div
              className="relative flex h-[68px] w-[68px] items-center justify-center rounded-[20px]"
              style={{
                background: 'rgba(201,168,76,0.07)',
                border: '1px solid rgba(201,168,76,0.2)',
                boxShadow: '0 8px 32px rgba(201,168,76,0.08)',
                animation: 'floatY 4s ease-in-out infinite',
              }}
            >
              <svg width="30" height="30" viewBox="0 0 20 20" fill="none">
                <rect x="1" y="3" width="11" height="4" rx="1.25" fill="#c9a84c" transform="rotate(45 6.5 5)" />
                <line x1="10.5" y1="10.5" x2="18.5" y2="18.5" stroke="#c9a84c" strokeWidth="2.5" strokeLinecap="round" />
                <rect x="0.5" y="16.5" width="8.5" height="2.5" rx="0.75" fill="#c9a84c" opacity="0.38" />
              </svg>
            </div>
          </div>

          {/* Heading */}
          <h1 className="mb-2 text-[28px] font-bold tracking-[-0.03em] text-white leading-tight text-center">
            Justice <span style={{ color: '#c9a84c' }}>AI</span>
          </h1>
          <p
            className="mb-10 text-[13.5px] text-center leading-relaxed"
            style={{ color: 'rgba(255,255,255,0.3)', maxWidth: 320 }}
          >
            Ask anything about your case files.
            Every query runs locally — nothing leaves your device.
          </p>

          {/* Drop zone */}
          <div
            onClick={onAddFiles}
            onDragOver={handleDragOver}
            onDragLeave={handleDragLeave}
            onDrop={handleDrop}
            className="w-full cursor-pointer"
            style={{
              maxWidth: 480,
              borderRadius: 20,
              border: `1.5px dashed ${isDragging ? 'rgba(201,168,76,0.6)' : 'rgba(255,255,255,0.09)'}`,
              background: isDragging
                ? 'rgba(201,168,76,0.05)'
                : 'rgba(255,255,255,0.01)',
              padding: '40px 36px',
              transition: 'border-color 0.2s ease, background 0.2s ease, box-shadow 0.2s ease',
              boxShadow: isDragging
                ? '0 0 0 4px rgba(201,168,76,0.06), inset 0 0 40px rgba(201,168,76,0.03)'
                : 'none',
            }}
            onMouseEnter={(e) => {
              if (!isDragging) {
                const el = e.currentTarget as HTMLDivElement
                el.style.borderColor = 'rgba(201,168,76,0.25)'
                el.style.background = 'rgba(201,168,76,0.02)'
              }
            }}
            onMouseLeave={(e) => {
              if (!isDragging) {
                const el = e.currentTarget as HTMLDivElement
                el.style.borderColor = 'rgba(255,255,255,0.09)'
                el.style.background = 'rgba(255,255,255,0.01)'
              }
            }}
          >
            <div className="flex flex-col items-center text-center gap-4">
              {/* Upload icon */}
              <div
                className="flex h-16 w-16 items-center justify-center rounded-2xl"
                style={{
                  background: isDragging ? 'rgba(201,168,76,0.12)' : 'rgba(255,255,255,0.03)',
                  border: `1px solid ${isDragging ? 'rgba(201,168,76,0.35)' : 'rgba(255,255,255,0.07)'}`,
                  transition: 'all 0.2s ease',
                  boxShadow: isDragging ? '0 0 24px rgba(201,168,76,0.1)' : 'none',
                }}
              >
                {isDragging ? (
                  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#c9a84c" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                    <polyline points="17 8 12 3 7 8" />
                    <line x1="12" y1="3" x2="12" y2="15" />
                  </svg>
                ) : (
                  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="rgba(255,255,255,0.28)" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                    <polyline points="17 8 12 3 7 8" />
                    <line x1="12" y1="3" x2="12" y2="15" />
                  </svg>
                )}
              </div>

              <div>
                <p className="text-[15.5px] font-semibold tracking-[-0.01em] leading-snug" style={{ color: isDragging ? '#c9a84c' : '#fff' }}>
                  {isDragging ? 'Release to load documents' : 'Drop your documents here'}
                </p>
                <p className="mt-1.5 text-[12.5px]" style={{ color: 'rgba(255,255,255,0.25)' }}>
                  PDF and DOCX supported · or click to browse
                </p>
              </div>

              {/* CTA buttons */}
              <div className="flex items-center gap-3 mt-1">
                <div
                  className="flex items-center gap-2 px-5 py-2.5 rounded-xl text-[12.5px] font-semibold"
                  style={{
                    background: 'rgba(201,168,76,0.12)',
                    border: '1px solid rgba(201,168,76,0.28)',
                    color: '#c9a84c',
                    boxShadow: '0 2px 8px rgba(201,168,76,0.08)',
                  }}
                >
                  <svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor">
                    <path d="M2 1.75C2 .784 2.784 0 3.75 0h6.586c.464 0 .909.184 1.237.513l2.914 2.914c.329.328.513.773.513 1.237v9.586A1.75 1.75 0 0 1 13.25 16h-9.5A1.75 1.75 0 0 1 2 14.25V1.75z" />
                  </svg>
                  Browse files
                </div>
                <button
                  onClick={(e) => { e.stopPropagation(); onAddFolder() }}
                  className="text-[12px] transition-colors no-drag px-3 py-2 rounded-lg"
                  style={{ color: 'rgba(255,255,255,0.3)', background: 'transparent' }}
                  onMouseEnter={(e) => {
                    const el = e.currentTarget as HTMLButtonElement
                    el.style.color = 'rgba(255,255,255,0.6)'
                    el.style.background = 'rgba(255,255,255,0.04)'
                  }}
                  onMouseLeave={(e) => {
                    const el = e.currentTarget as HTMLButtonElement
                    el.style.color = 'rgba(255,255,255,0.3)'
                    el.style.background = 'transparent'
                  }}
                >
                  Load folder
                </button>
              </div>

              {isLoading && (
                <div className="flex items-center gap-2 mt-1">
                  <div
                    className="h-3.5 w-3.5 rounded-full animate-spin"
                    style={{ border: '2px solid rgba(201,168,76,0.2)', borderTopColor: '#c9a84c' }}
                  />
                  <p className="text-[12px]" style={{ color: '#c9a84c' }}>
                    Processing documents…
                  </p>
                </div>
              )}
              {loadError && (
                <div
                  className="flex items-start gap-2.5 rounded-xl px-4 py-3 mt-1 w-full"
                  style={{
                    background: 'rgba(201,168,76,0.06)',
                    border: '1px solid rgba(201,168,76,0.28)',
                    borderLeft: '2px solid rgba(201,168,76,0.7)',
                    maxWidth: 360,
                  }}
                >
                  <svg width="13" height="13" viewBox="0 0 16 16" fill="none" className="shrink-0 mt-0.5">
                    <path d="M8.22 1.754a.25.25 0 0 0-.44 0L1.698 13.132a.25.25 0 0 0 .22.368h12.164a.25.25 0 0 0 .22-.368L8.22 1.754zm-1.358-.29a1.75 1.75 0 0 1 3.076 0l6.082 11.378A1.75 1.75 0 0 1 14.082 15.5H1.918a1.75 1.75 0 0 1-1.538-2.658L6.862 1.464z" fill="rgba(201,168,76,0.75)" />
                    <path d="M9 11a1 1 0 1 1-2 0 1 1 0 0 1 2 0zm-.25-5.25a.75.75 0 0 0-1.5 0v2.5a.75.75 0 0 0 1.5 0v-2.5z" fill="rgba(201,168,76,0.75)" />
                  </svg>
                  <p className="text-[12px] leading-relaxed text-left" style={{ color: 'rgba(201,168,76,0.9)' }}>
                    {loadError}
                  </p>
                </div>
              )}
            </div>
          </div>

          {/* Bento capability cards */}
          <div className="grid grid-cols-2 gap-2 mt-8" style={{ maxWidth: 440, width: '100%' }}>
            {([
              {
                icon: (
                  <svg width="14" height="14" viewBox="0 0 16 16" fill="#c9a84c">
                    <path d="M8.533.133a1.75 1.75 0 0 0-1.066 0l-5.25 1.68A1.75 1.75 0 0 0 1 3.48V7c0 1.566.832 3.125 2.561 4.608C5.163 13.101 6.97 14 8 14s2.837-.899 4.439-2.392C14.168 10.125 15 8.566 15 7V3.48a1.75 1.75 0 0 0-1.217-1.667zM5.5 9l2 2 3.5-3.5" stroke="#3fb950" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" fill="none" />
                  </svg>
                ),
                title: 'Privacy-First',
                desc: 'Nothing ever leaves your device',
                accent: 'rgba(63,185,80,0.55)',
                bg: 'rgba(63,185,80,0.04)',
                border: 'rgba(63,185,80,0.12)',
              },
              {
                icon: (
                  <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="#c9a84c" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M2 1.75C2 .784 2.784 0 3.75 0h6.586c.464 0 .909.184 1.237.513l2.914 2.914c.329.328.513.773.513 1.237v9.586A1.75 1.75 0 0 1 13.25 16h-9.5A1.75 1.75 0 0 1 2 14.25V1.75z" />
                    <line x1="5.5" y1="9" x2="10.5" y2="9" />
                    <line x1="5.5" y1="11.5" x2="8.5" y2="11.5" />
                  </svg>
                ),
                title: 'Cited Answers',
                desc: 'Filename + page for every claim',
                accent: 'rgba(201,168,76,0.6)',
                bg: 'rgba(201,168,76,0.04)',
                border: 'rgba(201,168,76,0.12)',
              },
              {
                icon: (
                  <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="#c9a84c" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round">
                    <circle cx="8" cy="8" r="6.5" />
                    <path d="M8 4v4l2.5 2" />
                  </svg>
                ),
                title: 'Seconds, Not Hours',
                desc: 'Semantic search across all pages',
                accent: 'rgba(201,168,76,0.6)',
                bg: 'rgba(201,168,76,0.04)',
                border: 'rgba(201,168,76,0.12)',
              },
              {
                icon: (
                  <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="#c9a84c" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round">
                    <rect x="2" y="2" width="12" height="12" rx="3" />
                    <rect x="5" y="5" width="6" height="6" rx="1.5" />
                    <line x1="2" y1="6" x2="0.5" y2="6" />
                    <line x1="14" y1="6" x2="15.5" y2="6" />
                    <line x1="2" y1="10" x2="0.5" y2="10" />
                    <line x1="14" y1="10" x2="15.5" y2="10" />
                  </svg>
                ),
                title: 'Runs On-Device',
                desc: 'Saul-7B · no cloud, no API keys',
                accent: 'rgba(201,168,76,0.6)',
                bg: 'rgba(201,168,76,0.04)',
                border: 'rgba(201,168,76,0.12)',
              },
            ] as { icon: JSX.Element; title: string; desc: string; accent: string; bg: string; border: string }[]).map((card) => (
              <div
                key={card.title}
                className="rounded-xl px-3.5 py-3 flex flex-col gap-2 group"
                style={{
                  background: card.bg,
                  border: `1px solid ${card.border}`,
                  transition: 'background 0.18s ease, border-color 0.18s ease, transform 0.18s ease',
                  cursor: 'default',
                }}
                onMouseEnter={(e) => {
                  const el = e.currentTarget as HTMLDivElement
                  el.style.background = card.bg.replace('0.04', '0.07')
                  el.style.borderColor = card.border.replace('0.12', '0.22')
                  el.style.transform = 'translateY(-2px)'
                }}
                onMouseLeave={(e) => {
                  const el = e.currentTarget as HTMLDivElement
                  el.style.background = card.bg
                  el.style.borderColor = card.border
                  el.style.transform = 'translateY(0)'
                }}
              >
                <div
                  className="w-6 h-6 rounded-lg flex items-center justify-center shrink-0"
                  style={{ background: 'rgba(255,255,255,0.04)', border: '1px solid rgba(255,255,255,0.07)' }}
                >
                  {card.icon}
                </div>
                <div>
                  <p className="text-[11.5px] font-semibold text-white leading-tight">{card.title}</p>
                  <p className="text-[10.5px] mt-0.5 leading-snug" style={{ color: 'rgba(255,255,255,0.3)' }}>
                    {card.desc}
                  </p>
                </div>
              </div>
            ))}
          </div>

        </div>
      </div>
    )
  }

  // ── CHAT VIEW ───────────────────────────────────────────────────────────────
  const isEmpty = messages.length === 0

  return (
    <div
      className="flex flex-1 flex-col h-screen overflow-hidden bg-[#080808] relative"
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      {/* Full-screen drag overlay */}
      {isDragging && (
        <div
          className="absolute inset-0 z-50 flex flex-col items-center justify-center pointer-events-none"
          style={{
            background: 'rgba(8,8,8,0.7)',
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
        style={{ borderBottom: '1px solid rgba(255,255,255,0.05)' }}
      >
        <div className="no-drag flex items-center gap-2 min-w-0">
          <svg width="11" height="11" viewBox="0 0 20 20" fill="none" style={{ flexShrink: 0 }}>
            <rect x="1" y="3" width="11" height="4" rx="1.25" fill="rgba(201,168,76,0.45)" transform="rotate(45 6.5 5)" />
            <line x1="10.5" y1="10.5" x2="18.5" y2="18.5" stroke="rgba(201,168,76,0.45)" strokeWidth="2.5" strokeLinecap="round" />
          </svg>
          <span
            className="text-[12.5px] font-medium truncate"
            style={{ color: isEmpty ? 'rgba(255,255,255,0.2)' : 'rgba(255,255,255,0.6)', maxWidth: 400, letterSpacing: '-0.01em' }}
          >
            {isEmpty ? 'New Chat' : sessionName}
          </span>
        </div>

        {/* Minimal doc count pill — documents managed in Context panel */}
        {files.length > 0 && (
          <div
            className="no-drag flex items-center gap-1.5 rounded-lg px-2.5 py-1 text-[11px] shrink-0"
            style={{ border: '1px solid rgba(255,255,255,0.06)', color: 'rgba(255,255,255,0.28)', background: 'rgba(255,255,255,0.02)' }}
          >
            <svg width="9" height="9" viewBox="0 0 16 16" fill="rgba(201,168,76,0.5)">
              <path d="M2 1.75C2 .784 2.784 0 3.75 0h6.586c.464 0 .909.184 1.237.513l2.914 2.914c.329.328.513.773.513 1.237v9.586A1.75 1.75 0 0 1 13.25 16h-9.5A1.75 1.75 0 0 1 2 14.25z" />
            </svg>
            {files.length} {files.length === 1 ? 'doc' : 'docs'}
          </div>
        )}
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto">
        {isEmpty ? (
          <div className="flex h-full flex-col items-center justify-center px-8 py-16 text-center">
            {!hasFiles ? (
              /* No docs yet — prompt to add */
              <>
                <div
                  className="mb-5 flex h-14 w-14 items-center justify-center rounded-2xl"
                  style={{
                    background: 'rgba(201,168,76,0.07)',
                    border: '1px solid rgba(201,168,76,0.2)',
                    boxShadow: '0 4px 20px rgba(201,168,76,0.06)',
                  }}
                >
                  <svg width="22" height="22" viewBox="0 0 16 16" fill="rgba(201,168,76,0.8)">
                    <path d="M2 1.75C2 .784 2.784 0 3.75 0h6.586c.464 0 .909.184 1.237.513l2.914 2.914c.329.328.513.773.513 1.237v9.586A1.75 1.75 0 0 1 13.25 16h-9.5A1.75 1.75 0 0 1 2 14.25V1.75zM8.75 9.25a.75.75 0 0 0-1.5 0v1.5H5.75a.75.75 0 0 0 0 1.5h1.5v1.5a.75.75 0 0 0 1.5 0v-1.5h1.5a.75.75 0 0 0 0-1.5H8.75v-1.5z" />
                  </svg>
                </div>
                <h3 className="mb-2 text-[18px] font-semibold tracking-[-0.02em] text-white">
                  Add documents to get started
                </h3>
                <p className="mb-7 text-[12.5px] leading-relaxed" style={{ color: 'rgba(255,255,255,0.28)', maxWidth: 280 }}>
                  Load PDFs or Word files, then ask any question about your case.
                </p>
                <button
                  onClick={onAddFiles}
                  className="flex items-center gap-2 rounded-xl px-6 py-3 text-[13px] font-semibold transition-all"
                  style={{
                    background: '#c9a84c',
                    color: '#080808',
                    boxShadow: '0 4px 16px rgba(201,168,76,0.22)',
                  }}
                  onMouseEnter={(e) => {
                    const el = e.currentTarget as HTMLButtonElement
                    el.style.background = '#e8c97e'
                    el.style.boxShadow = '0 6px 20px rgba(201,168,76,0.32)'
                  }}
                  onMouseLeave={(e) => {
                    const el = e.currentTarget as HTMLButtonElement
                    el.style.background = '#c9a84c'
                    el.style.boxShadow = '0 4px 16px rgba(201,168,76,0.22)'
                  }}
                >
                  <svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor">
                    <path d="M7.75 2a.75.75 0 0 1 .75.75V7h4.25a.75.75 0 0 1 0 1.5H8.5v4.25a.75.75 0 0 1-1.5 0V8.5H2.75a.75.75 0 0 1 0-1.5H7V2.75A.75.75 0 0 1 7.75 2z" />
                  </svg>
                  Add Documents
                </button>
              </>
            ) : (
              /* Has docs — show example prompts */
              <>
                <p className="mb-1.5 text-[10.5px] font-semibold uppercase tracking-[0.2em]" style={{ color: 'rgba(201,168,76,0.5)' }}>
                  {files.length} {files.length === 1 ? 'document' : 'documents'} ready
                </p>
                <h3 className="mb-7 text-[22px] font-semibold tracking-[-0.02em] text-white">
                  What would you like to know?
                </h3>
                <div className="grid grid-cols-2 gap-2 w-full max-w-lg">
                  {EXAMPLES.map((q) => (
                    <button
                      key={q}
                      onClick={() => { setInput(q); textareaRef.current?.focus() }}
                      className="rounded-xl px-4 py-3.5 text-left text-[12px] leading-relaxed"
                      style={{
                        background: '#0c0c0c',
                        border: '1px solid rgba(255,255,255,0.06)',
                        color: 'rgba(255,255,255,0.38)',
                        transition: 'background 0.12s ease, border-color 0.12s ease, color 0.12s ease',
                      }}
                      onMouseEnter={(e) => {
                        const el = e.currentTarget as HTMLButtonElement
                        el.style.background = '#121212'
                        el.style.borderColor = 'rgba(201,168,76,0.22)'
                        el.style.color = 'rgba(255,255,255,0.78)'
                      }}
                      onMouseLeave={(e) => {
                        const el = e.currentTarget as HTMLButtonElement
                        el.style.background = '#0c0c0c'
                        el.style.borderColor = 'rgba(255,255,255,0.06)'
                        el.style.color = 'rgba(255,255,255,0.38)'
                      }}
                    >
                      {q}
                    </button>
                  ))}
                </div>
              </>
            )}
          </div>
        ) : (
          <div className="flex flex-col gap-7 max-w-3xl mx-auto w-full px-6 py-8 pb-10">
            {messages.map((msg) => (
              <MessageBubble key={msg.id} message={msg} onViewCitation={onViewCitation} />
            ))}
            {isQuerying && <TypingIndicator />}
            <div ref={messagesEndRef} className="h-4" />
          </div>
        )}
      </div>

      {/* Input */}
      <div
        className="shrink-0 px-6 py-4"
        style={{ borderTop: '1px solid rgba(255,255,255,0.05)', background: '#080808' }}
      >
        <div className="max-w-3xl mx-auto">
          {loadError && (
            <div
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
            className="rounded-2xl"
            style={{
              background: '#0d0d0d',
              border: '1px solid rgba(255,255,255,0.08)',
              transition: 'border-color 0.18s ease, box-shadow 0.18s ease',
            }}
            onFocusCapture={(e) => {
              const el = e.currentTarget as HTMLDivElement
              el.style.borderColor = 'rgba(201,168,76,0.35)'
              el.style.boxShadow = '0 0 0 3px rgba(201,168,76,0.07)'
            }}
            onBlurCapture={(e) => {
              const el = e.currentTarget as HTMLDivElement
              el.style.borderColor = 'rgba(255,255,255,0.08)'
              el.style.boxShadow = 'none'
            }}
          >
            <div className="flex items-end gap-3 px-4 py-3.5">
              <textarea
                ref={textareaRef}
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={handleKeyDown}
                disabled={isQuerying}
                placeholder="Ask a question about your documents…"
                rows={1}
                className="flex-1 bg-transparent text-[13.5px] text-white leading-6 outline-none placeholder-white/20 disabled:opacity-50"
                style={{ maxHeight: 128, overflowY: 'auto' }}
              />
              <button
                onClick={handleSend}
                disabled={isQuerying || !input.trim() || !hasFiles}
                title={!hasFiles ? 'Add documents first' : undefined}
                className="flex shrink-0 h-8 w-8 items-center justify-center rounded-xl disabled:opacity-30 disabled:cursor-not-allowed"
                style={{
                  background: input.trim() && !isQuerying ? '#c9a84c' : 'rgba(255,255,255,0.06)',
                  color: input.trim() && !isQuerying ? '#080808' : 'rgba(255,255,255,0.3)',
                  transition: 'background 0.2s ease, color 0.2s ease, box-shadow 0.2s ease',
                  boxShadow: input.trim() && !isQuerying ? '0 2px 10px rgba(201,168,76,0.25)' : 'none',
                }}
              >
                {isQuerying ? (
                  <svg className="animate-spin h-4 w-4" viewBox="0 0 24 24" fill="none">
                    <circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="3" opacity="0.2" />
                    <path d="M12 2a10 10 0 0110 10" stroke="currentColor" strokeWidth="3" strokeLinecap="round" />
                  </svg>
                ) : (
                  <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
                    <path d="M.989 8 .064 2.68a1.342 1.342 0 0 1 1.85-1.462l13.402 5.744a1.13 1.13 0 0 1 0 2.076L1.913 14.782a1.342 1.342 0 0 1-1.85-1.463L.99 8zm.603-5.135.024.12L2.15 7.25h6.848a.75.75 0 0 1 0 1.5H2.15l-.534 4.265-.024.12 13.016-5.577z" />
                  </svg>
                )}
              </button>
            </div>
          </div>
          <div className="mt-2 flex items-center justify-center gap-1.5">
            <svg width="8" height="8" viewBox="0 0 20 20" fill="none">
              <rect x="1" y="3" width="11" height="4" rx="1.25" fill="rgba(201,168,76,0.25)" transform="rotate(45 6.5 5)" />
              <line x1="10.5" y1="10.5" x2="18.5" y2="18.5" stroke="rgba(201,168,76,0.25)" strokeWidth="2.5" strokeLinecap="round" />
            </svg>
            <p className="text-[10px] tracking-wide" style={{ color: 'rgba(255,255,255,0.15)' }}>
              Justice AI · Enter to send · Answers grounded in your documents
            </p>
          </div>
        </div>
      </div>
    </div>
  )
}

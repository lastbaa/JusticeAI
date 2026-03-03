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
          minWidth: 220,
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
            className="w-full cursor-pointer transition-all"
            style={{
              maxWidth: 480,
              borderRadius: 18,
              border: `1.5px dashed ${isDragging ? 'rgba(201,168,76,0.55)' : 'rgba(255,255,255,0.1)'}`,
              background: isDragging ? 'rgba(201,168,76,0.04)' : 'rgba(255,255,255,0.01)',
              padding: '36px 32px',
              transition: 'all 0.2s ease',
            }}
            onMouseEnter={(e) => {
              if (!isDragging) {
                const el = e.currentTarget as HTMLDivElement
                el.style.borderColor = 'rgba(255,255,255,0.18)'
                el.style.background = 'rgba(255,255,255,0.025)'
              }
            }}
            onMouseLeave={(e) => {
              if (!isDragging) {
                const el = e.currentTarget as HTMLDivElement
                el.style.borderColor = 'rgba(255,255,255,0.1)'
                el.style.background = 'rgba(255,255,255,0.01)'
              }
            }}
          >
            <div className="flex flex-col items-center text-center gap-4">
              {/* Upload icon */}
              <div
                className="flex h-14 w-14 items-center justify-center rounded-2xl"
                style={{
                  background: isDragging ? 'rgba(201,168,76,0.1)' : 'rgba(255,255,255,0.04)',
                  border: `1px solid ${isDragging ? 'rgba(201,168,76,0.28)' : 'rgba(255,255,255,0.07)'}`,
                  transition: 'all 0.2s ease',
                }}
              >
                {isDragging ? (
                  <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="#c9a84c" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                    <polyline points="17 8 12 3 7 8" />
                    <line x1="12" y1="3" x2="12" y2="15" />
                  </svg>
                ) : (
                  <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="rgba(255,255,255,0.35)" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                    <polyline points="17 8 12 3 7 8" />
                    <line x1="12" y1="3" x2="12" y2="15" />
                  </svg>
                )}
              </div>

              <div>
                <p className="text-[15px] font-semibold text-white leading-snug">
                  {isDragging ? 'Release to load' : 'Drop your documents here'}
                </p>
                <p className="mt-1 text-[12.5px]" style={{ color: 'rgba(255,255,255,0.28)' }}>
                  PDF and DOCX · or click to browse
                </p>
              </div>

              {/* CTA buttons */}
              <div className="flex items-center gap-3 mt-1">
                <div
                  className="flex items-center gap-2 px-5 py-2.5 rounded-xl text-[12.5px] font-semibold"
                  style={{
                    background: 'rgba(201,168,76,0.1)',
                    border: '1px solid rgba(201,168,76,0.22)',
                    color: '#c9a84c',
                  }}
                >
                  <svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor">
                    <path d="M2 1.75C2 .784 2.784 0 3.75 0h6.586c.464 0 .909.184 1.237.513l2.914 2.914c.329.328.513.773.513 1.237v9.586A1.75 1.75 0 0 1 13.25 16h-9.5A1.75 1.75 0 0 1 2 14.25V1.75z" />
                  </svg>
                  Browse files
                </div>
                <button
                  onClick={(e) => { e.stopPropagation(); onAddFolder() }}
                  className="text-[12px] transition-colors no-drag"
                  style={{ color: 'rgba(255,255,255,0.25)' }}
                  onMouseEnter={(e) => { (e.currentTarget as HTMLButtonElement).style.color = 'rgba(255,255,255,0.55)' }}
                  onMouseLeave={(e) => { (e.currentTarget as HTMLButtonElement).style.color = 'rgba(255,255,255,0.25)' }}
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
                <p className="mt-1 text-[12px]" style={{ color: '#f85149' }}>
                  {loadError}
                </p>
              )}
            </div>
          </div>

          {/* Feature pills */}
          <div className="flex items-center gap-2.5 mt-8 flex-wrap justify-center">
            {[
              { label: 'Cited answers' },
              { label: 'Documents local' },
              { label: 'Source-grounded' },
              { label: 'Privacy-first' },
            ].map((f) => (
              <div
                key={f.label}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-full text-[11px] font-medium"
                style={{
                  background: 'rgba(255,255,255,0.03)',
                  border: '1px solid rgba(255,255,255,0.07)',
                  color: 'rgba(255,255,255,0.3)',
                }}
              >
                <span
                  className="w-1 h-1 rounded-full shrink-0"
                  style={{ background: 'rgba(201,168,76,0.6)' }}
                />
                {f.label}
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
    <div className="flex flex-1 flex-col h-screen overflow-hidden bg-[#080808]">

      {/* Header — matches sidebar's h-11 traffic zone + content row */}
      <div
        className="drag-region flex h-11 items-center justify-between shrink-0 px-5"
        style={{ borderBottom: '1px solid rgba(255,255,255,0.05)' }}
      >
        <div className="no-drag flex items-center gap-2">
          <svg width="11" height="11" viewBox="0 0 20 20" fill="none">
            <rect x="1" y="3" width="11" height="4" rx="1.25" fill="rgba(201,168,76,0.5)" transform="rotate(45 6.5 5)" />
            <line x1="10.5" y1="10.5" x2="18.5" y2="18.5" stroke="rgba(201,168,76,0.5)" strokeWidth="2.5" strokeLinecap="round" />
          </svg>
          <span
            className="text-[12.5px] font-medium truncate"
            style={{ color: isEmpty ? 'rgba(255,255,255,0.2)' : 'rgba(255,255,255,0.55)', maxWidth: 400 }}
          >
            {isEmpty ? 'New Chat' : sessionName}
          </span>
        </div>

        {/* Minimal doc count pill — documents managed in Context panel */}
        {files.length > 0 && (
          <div
            className="no-drag flex items-center gap-1.5 rounded-lg px-2.5 py-1 text-[11px]"
            style={{ border: '1px solid rgba(255,255,255,0.07)', color: 'rgba(255,255,255,0.25)' }}
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
            <p className="mb-1 text-[11px] font-semibold uppercase tracking-[0.18em]" style={{ color: 'rgba(201,168,76,0.5)' }}>
              {hasFiles
                ? `${files.length} ${files.length === 1 ? 'document' : 'documents'} ready`
                : 'No documents loaded'}
            </p>
            <h3 className="mb-7 text-[22px] font-semibold tracking-[-0.02em] text-white">
              What would you like to know?
            </h3>
            <div className="grid grid-cols-2 gap-2 w-full max-w-lg">
              {EXAMPLES.map((q) => (
                <button
                  key={q}
                  onClick={() => { setInput(q); textareaRef.current?.focus() }}
                  className="rounded-xl px-4 py-3.5 text-left text-[12px] leading-relaxed transition-all"
                  style={{ background: '#0c0c0c', border: '1px solid rgba(255,255,255,0.06)', color: 'rgba(255,255,255,0.38)' }}
                  onMouseEnter={(e) => {
                    const el = e.currentTarget as HTMLButtonElement
                    el.style.background = '#111'
                    el.style.borderColor = 'rgba(201,168,76,0.2)'
                    el.style.color = 'rgba(255,255,255,0.72)'
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
          </div>
        ) : (
          <div className="flex flex-col gap-7 max-w-3xl mx-auto w-full px-6 py-8">
            {messages.map((msg) => (
              <MessageBubble key={msg.id} message={msg} onViewCitation={onViewCitation} />
            ))}
            {isQuerying && <TypingIndicator />}
            <div ref={messagesEndRef} />
          </div>
        )}
      </div>

      {/* No-docs warning */}
      {!hasFiles && (
        <div
          className="shrink-0 mx-6 mb-0 mt-0 rounded-lg px-3 py-2 flex items-center gap-2"
          style={{ background: 'rgba(201,168,76,0.06)', border: '1px solid rgba(201,168,76,0.14)', marginBottom: -4 }}
        >
          <svg width="11" height="11" viewBox="0 0 16 16" fill="#c9a84c" opacity="0.7">
            <path d="M8 0a8 8 0 1 0 0 16A8 8 0 0 0 8 0zm.25 4.75a.75.75 0 0 0-1.5 0v4.5a.75.75 0 0 0 1.5 0v-4.5zM8 11a1 1 0 1 1 0 2 1 1 0 0 1 0-2z" />
          </svg>
          <p className="text-[11px]" style={{ color: 'rgba(201,168,76,0.7)' }}>
            Add documents in the panel on the right before asking a question
          </p>
        </div>
      )}

      {/* Input */}
      <div
        className="shrink-0 px-6 py-4"
        style={{ borderTop: '1px solid rgba(255,255,255,0.05)', background: '#080808' }}
      >
        <div className="max-w-3xl mx-auto">
          <div
            className="rounded-2xl transition-colors"
            style={{ background: '#0d0d0d', border: '1px solid rgba(255,255,255,0.08)' }}
            onFocusCapture={(e) => {
              (e.currentTarget as HTMLDivElement).style.borderColor = 'rgba(201,168,76,0.3)'
            }}
            onBlurCapture={(e) => {
              (e.currentTarget as HTMLDivElement).style.borderColor = 'rgba(255,255,255,0.08)'
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
                className="flex-1 bg-transparent text-[13px] text-white leading-6 outline-none placeholder-white/20 disabled:opacity-50"
                style={{ maxHeight: 128, overflowY: 'auto' }}
              />
              <button
                onClick={handleSend}
                disabled={isQuerying || !input.trim() || !hasFiles}
                title={!hasFiles ? 'Add documents first' : undefined}
                className="flex shrink-0 h-8 w-8 items-center justify-center rounded-xl transition-all disabled:opacity-30 disabled:cursor-not-allowed"
                style={{
                  background: input.trim() ? '#c9a84c' : 'rgba(255,255,255,0.06)',
                  color: input.trim() ? '#080808' : 'rgba(255,255,255,0.3)',
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
            <svg width="9" height="9" viewBox="0 0 20 20" fill="none">
              <rect x="1" y="3" width="11" height="4" rx="1.25" fill="rgba(201,168,76,0.3)" transform="rotate(45 6.5 5)" />
              <line x1="10.5" y1="10.5" x2="18.5" y2="18.5" stroke="rgba(201,168,76,0.3)" strokeWidth="2.5" strokeLinecap="round" />
            </svg>
            <p className="text-[10px]" style={{ color: 'rgba(255,255,255,0.13)' }}>
              Justice AI · Enter to send · Answers grounded in your documents
            </p>
          </div>
        </div>
      </div>
    </div>
  )
}

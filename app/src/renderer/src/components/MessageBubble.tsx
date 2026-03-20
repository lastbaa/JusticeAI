import { useState } from 'react'
import { ChatMessage, Citation } from '../../../../../shared/src/types'
import SourceCard from './SourceCard'

interface Props {
  message: ChatMessage
  onViewCitation?: (citation: Citation) => void
}

// ── Primary/secondary partition ───────────────────────────────────────────────
// Top citation is always primary. Any subsequent citation within 0.05 score of
// the top is also primary (handles ties), capped at 2 primaries total.
function partitionCitations(items: Citation[]): { primary: Citation[]; secondary: Citation[] } {
  if (items.length === 0) return { primary: [], secondary: [] }
  if (items.length === 1) return { primary: items, secondary: [] }
  const topScore = items[0].score
  const primary: Citation[] = []
  const secondary: Citation[] = []
  for (const c of items) {
    if (primary.length < 2 && topScore - c.score <= 0.05) primary.push(c)
    else secondary.push(c)
  }
  return { primary, secondary }
}

// ── Deduplicated citation list with "show all" toggle ────────────────────────
function CitationSources({ citations, onViewCitation }: { citations: Citation[]; onViewCitation?: (c: Citation) => void }): JSX.Element {
  const [showAll, setShowAll] = useState(false)
  const [secondaryOpen, setSecondaryOpen] = useState(false)

  // Deduplicate by (fileName, pageNumber) — keep highest-scored citation per page
  const deduped: Citation[] = []
  const seen = new Set<string>()
  // citations are already sorted by score descending from the pipeline
  for (const c of citations) {
    const key = `${c.fileName}::${c.pageNumber}`
    if (!seen.has(key)) {
      seen.add(key)
      deduped.push(c)
    }
  }

  const hasExtras = deduped.length < citations.length

  const displayBase = showAll ? citations : deduped
  const { primary: primarySlice, secondary: secondaryAll } = partitionCitations(displayBase)
  const secondarySlice = secondaryAll.slice(0, 6 - primarySlice.length)

  return (
    <div className="mt-3">
      {/* Header row */}
      <div className="flex items-center justify-between mb-2">
        <p
          className="text-[10px] font-semibold uppercase tracking-[0.12em]"
          style={{ color: 'rgb(var(--ov) / 0.18)' }}
        >
          Sources ({showAll ? citations.length : deduped.length})
        </p>
        {hasExtras && (
          <button
            onClick={() => setShowAll((v) => !v)}
            className="text-[10px] font-medium transition-colors"
            style={{ color: 'rgba(201,168,76,0.5)' }}
            onMouseEnter={(e) => { (e.currentTarget as HTMLButtonElement).style.color = 'rgba(201,168,76,0.85)' }}
            onMouseLeave={(e) => { (e.currentTarget as HTMLButtonElement).style.color = 'rgba(201,168,76,0.5)' }}
          >
            {showAll ? 'Show unique pages' : 'Show all chunks'}
          </button>
        )}
      </div>

      {/* Key Sources section */}
      {primarySlice.length > 0 && (
        <div className="mb-2">
          <p
            className="text-[9px] font-bold uppercase tracking-[0.14em] mb-1.5"
            style={{ color: 'rgba(201,168,76,0.45)' }}
          >
            Key Sources
          </p>
          <div className="flex flex-col gap-1.5">
            {primarySlice.map((citation, idx) => (
              <SourceCard key={`primary-${idx}`} citation={citation} onView={onViewCitation} isPrimary={true} />
            ))}
          </div>
        </div>
      )}

      {/* Supporting Sources section — collapsed by default */}
      {secondarySlice.length > 0 && (
        <div>
          <button
            onClick={() => setSecondaryOpen((v) => !v)}
            className="flex items-center gap-1.5 mb-1.5"
            style={{ background: 'none', border: 'none', padding: 0, cursor: 'pointer' }}
          >
            <p
              className="text-[9px] font-bold uppercase tracking-[0.14em]"
              style={{ color: 'rgb(var(--ov) / 0.18)' }}
            >
              Supporting Sources ({secondarySlice.length})
            </p>
            <svg
              width="8"
              height="8"
              viewBox="0 0 10 10"
              fill="none"
              stroke="rgb(var(--ov) / 0.2)"
              strokeWidth="1.8"
              strokeLinecap="round"
              style={{
                transform: secondaryOpen ? 'rotate(180deg)' : 'rotate(0deg)',
                transition: 'transform 0.18s ease',
              }}
            >
              <path d="M2 3.5l3 3 3-3" />
            </svg>
          </button>
          {secondaryOpen && (
            <div className="flex flex-col gap-1.5">
              {secondarySlice.map((citation, idx) => (
                <SourceCard key={`secondary-${idx}`} citation={citation} onView={onViewCitation} isPrimary={false} />
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  )
}

function formatTime(ts: number): string {
  return new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

function ScalesAvatar(): JSX.Element {
  return (
    <div
      className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full mt-0.5"
      style={{ background: 'rgba(201,168,76,0.09)', border: '1px solid rgba(201,168,76,0.2)' }}
    >
      <svg width="13" height="13" viewBox="0 0 28 28" fill="none">
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
  )
}

// ── Copy button with checkmark flash ─────────────────────────────────────────
function CopyButton({ text, className, style }: { text: string; className?: string; style?: React.CSSProperties }): JSX.Element {
  const [copied, setCopied] = useState(false)

  function handleCopy(e: React.MouseEvent): void {
    e.stopPropagation()
    navigator.clipboard.writeText(text).then(() => {
      setCopied(true)
      setTimeout(() => setCopied(false), 1800)
    }).catch(() => {})
  }

  return (
    <button
      onClick={handleCopy}
      title="Copy"
      className={`flex items-center justify-center rounded transition-all ${className ?? ''}`}
      style={{
        color: copied ? '#3fb950' : 'rgb(var(--ov) / 0.22)',
        ...style,
      }}
      onMouseEnter={(e) => {
        if (!copied) (e.currentTarget as HTMLButtonElement).style.color = 'rgb(var(--ov) / 0.55)'
      }}
      onMouseLeave={(e) => {
        if (!copied) (e.currentTarget as HTMLButtonElement).style.color = 'rgb(var(--ov) / 0.22)'
      }}
    >
      {copied ? (
        <svg width="11" height="11" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
          <path d="M2 8l4 4 8-8" />
        </svg>
      ) : (
        <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor">
          <path d="M0 6.75C0 5.784.784 5 1.75 5h1.5a.75.75 0 0 1 0 1.5h-1.5a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-1.5a.75.75 0 0 1 1.5 0v1.5A1.75 1.75 0 0 1 9.25 16h-7.5A1.75 1.75 0 0 1 0 14.25Z" />
          <path d="M5 1.75C5 .784 5.784 0 6.75 0h7.5C15.216 0 16 .784 16 1.75v7.5A1.75 1.75 0 0 1 14.25 11h-7.5A1.75 1.75 0 0 1 5 9.25Zm1.75-.25a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-7.5a.25.25 0 0 0-.25-.25Z" />
        </svg>
      )}
    </button>
  )
}

// ── Lightweight markdown renderer ─────────────────────────────────────────────
function renderMarkdown(text: string): JSX.Element {
  const lines = text.split('\n')
  const elements: JSX.Element[] = []
  let i = 0

  while (i < lines.length) {
    const line = lines[i]

    // Fenced code block
    if (line.startsWith('```')) {
      const lang = line.slice(3).trim()
      const codeLines: string[] = []
      i++
      while (i < lines.length && !lines[i].startsWith('```')) {
        codeLines.push(lines[i])
        i++
      }
      const codeText = codeLines.join('\n')
      elements.push(
        <div key={i} style={{ position: 'relative', margin: '8px 0' }}>
          <pre style={{ background: 'var(--bg)', border: '1px solid rgb(var(--ov) / 0.1)', borderRadius: 10, padding: '12px 16px', overflowX: 'auto', margin: 0 }}>
            <code style={{ fontFamily: "'SF Mono','Fira Mono',monospace", fontSize: '0.82em', color: 'rgb(var(--ov) / 0.82)', lineHeight: 1.65 }} data-lang={lang}>
              {codeText}
            </code>
          </pre>
          <div style={{ position: 'absolute', top: 8, right: 8 }}>
            <CopyButton text={codeText} />
          </div>
        </div>
      )
      i++
      continue
    }

    // Bullet list item
    if (/^[-*•]\s/.test(line)) {
      const items: string[] = []
      while (i < lines.length && /^[-*•]\s/.test(lines[i])) {
        items.push(lines[i].replace(/^[-*•]\s+/, ''))
        i++
      }
      elements.push(
        <ul key={i} style={{ listStyle: 'disc', paddingLeft: '1.25em', margin: '4px 0 6px' }}>
          {items.map((item, j) => (
            <li key={j} style={{ marginBottom: '0.15em' }}>{inlineMarkdown(item)}</li>
          ))}
        </ul>
      )
      continue
    }

    // Numbered list item
    if (/^\d+\.\s/.test(line)) {
      const items: string[] = []
      while (i < lines.length && /^\d+\.\s/.test(lines[i])) {
        items.push(lines[i].replace(/^\d+\.\s+/, ''))
        i++
      }
      elements.push(
        <ol key={i} style={{ listStyle: 'decimal', paddingLeft: '1.25em', margin: '4px 0 6px' }}>
          {items.map((item, j) => (
            <li key={j} style={{ marginBottom: '0.15em' }}>{inlineMarkdown(item)}</li>
          ))}
        </ol>
      )
      continue
    }

    // Headings
    const headingMatch = line.match(/^(#{1,3})\s+(.+)/)
    if (headingMatch) {
      const level = headingMatch[1].length
      const content = headingMatch[2]
      const style: React.CSSProperties = { fontWeight: 600, color: 'var(--text)', margin: '8px 0 3px', lineHeight: 1.3, fontSize: level === 1 ? '1.05em' : level === 2 ? '0.98em' : '0.93em' }
      elements.push(<p key={i} style={style}>{inlineMarkdown(content)}</p>)
      i++
      continue
    }

    // Blank line
    if (line.trim() === '') {
      if (elements.length > 0) {
        elements.push(<div key={i} style={{ height: '0.4em' }} />)
      }
      i++
      continue
    }

    // Normal paragraph line
    elements.push(
      <p key={i} style={{ margin: 0 }}>{inlineMarkdown(line)}</p>
    )
    i++
  }

  return <>{elements}</>
}

// Inline markdown: **bold**, *italic*, `code`
function inlineMarkdown(text: string): (string | JSX.Element)[] {
  const parts: (string | JSX.Element)[] = []
  const re = /(\*\*(.+?)\*\*|\*(.+?)\*|`([^`]+)`)/g
  let last = 0
  let m: RegExpExecArray | null

  while ((m = re.exec(text)) !== null) {
    if (m.index > last) parts.push(text.slice(last, m.index))
    if (m[0].startsWith('**')) {
      parts.push(<strong key={m.index} style={{ color: 'var(--text)', fontWeight: 600 }}>{m[2]}</strong>)
    } else if (m[0].startsWith('*')) {
      parts.push(<em key={m.index}>{m[3]}</em>)
    } else {
      parts.push(
        <code key={m.index} style={{ fontFamily: "'SF Mono','Fira Mono',monospace", fontSize: '0.85em', background: 'rgb(var(--ov) / 0.07)', border: '1px solid rgb(var(--ov) / 0.08)', borderRadius: 4, padding: '0.1em 0.35em' }}>
          {m[4]}
        </code>
      )
    }
    last = m.index + m[0].length
  }

  if (last < text.length) parts.push(text.slice(last))
  return parts
}

export default function MessageBubble({ message, onViewCitation }: Props): JSX.Element {
  const [hovered, setHovered] = useState(false)
  const isUser = message.role === 'user'

  if (isUser) {
    return (
      <div
        className="flex justify-end"
        style={{ animation: 'fadeUp 0.25s ease both' }}
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
      >
        <div style={{ maxWidth: '72%' }}>
          <div
            className="relative rounded-2xl rounded-tr-sm px-4 py-3.5"
            style={{
              background: 'var(--surface-raised)',
              border: '1px solid rgb(var(--ov) / 0.09)',
              boxShadow: '0 1px 8px var(--shadow)',
            }}
          >
            <p className="text-[13.5px] leading-relaxed whitespace-pre-wrap" style={{ color: 'var(--text)' }}>
              {message.content}
            </p>
            {hovered && (
              <div style={{ position: 'absolute', top: 6, right: 6 }}>
                <CopyButton text={message.content} />
              </div>
            )}
          </div>
          <p className="mt-1.5 text-right text-[10px]" style={{ color: 'rgb(var(--ov) / 0.2)' }}>
            {formatTime(message.timestamp)}
          </p>
        </div>
      </div>
    )
  }

  const isNotFound = message.notFound

  return (
    <div
      className="flex gap-3"
      style={{ animation: 'fadeUp 0.25s ease both' }}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      <ScalesAvatar />

      <div className="flex-1 min-w-0">
        <div className="flex items-center justify-between mb-2.5">
          <p
            className="text-[10px] font-bold tracking-[0.14em] uppercase"
            style={{ color: 'rgba(201,168,76,0.6)' }}
          >
            Justice AI
          </p>
          {hovered && !isNotFound && !message.isStreaming && message.content.trim() && (
            <CopyButton text={message.content} />
          )}
        </div>

        {isNotFound ? (
          <div
            className="rounded-xl px-4 py-3.5"
            style={{
              background: 'rgb(var(--ov) / 0.02)',
              border: '1px solid rgb(var(--ov) / 0.07)',
              borderLeft: '2px solid rgb(var(--ov) / 0.15)',
            }}
          >
            <div className="flex items-start gap-2.5">
              <svg width="13" height="13" viewBox="0 0 16 16" fill="none" className="shrink-0 mt-0.5">
                <path d="M8 1a7 7 0 1 0 0 14A7 7 0 0 0 8 1zm0 3.5a.75.75 0 0 1 .75.75v3a.75.75 0 0 1-1.5 0v-3A.75.75 0 0 1 8 4.5zm0 6.5a.75.75 0 1 1 0-1.5.75.75 0 0 1 0 1.5z" fill="rgb(var(--ov) / 0.25)" />
              </svg>
              <p
                className="text-[13px] leading-relaxed whitespace-pre-wrap"
                style={{ color: 'rgb(var(--ov) / 0.85)' }}
              >
                {message.content}
              </p>
            </div>
          </div>
        ) : (
          <div
            className="rounded-2xl rounded-tl-sm px-4 py-3.5"
            style={{
              background: 'var(--bg-alt)',
              border: '1px solid rgb(var(--ov) / 0.06)',
              borderLeft: '2.5px solid rgba(201,168,76,0.28)',
            }}
          >
            <div className="text-[13.5px] leading-[1.75]" style={{ color: 'var(--text)' }}>
              {message.content.trim()
                ? <>
                    {renderMarkdown(message.content)}
                    {message.isStreaming && (
                      <span
                        style={{
                          display: 'inline-block',
                          width: '2px',
                          height: '1em',
                          background: 'rgba(201,168,76,0.8)',
                          marginLeft: '2px',
                          verticalAlign: 'text-bottom',
                          animation: 'cursorBlink 0.9s step-end infinite',
                        }}
                      />
                    )}
                  </>
                : <span style={{ color: 'rgb(var(--ov) / 0.2)', fontStyle: 'italic' }}>No response generated. Please try again.</span>
              }
            </div>
          </div>
        )}

        <p className="mt-1.5 text-[10px]" style={{ color: 'rgb(var(--ov) / 0.22)' }}>
          {formatTime(message.timestamp)}
        </p>

        {!isNotFound && message.citations && message.citations.length > 0 && (
          <CitationSources citations={message.citations} onViewCitation={onViewCitation} />
        )}
      </div>
    </div>
  )
}

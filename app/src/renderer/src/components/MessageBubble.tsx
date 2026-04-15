import { useState } from 'react'
import { AssertionResult, ChatMessage, Citation, FileInfo, InferenceMode } from '../../../../../shared/src/types'
import SourceCard from './SourceCard'
import KeyFiguresCard, { extractKeyFigures } from './KeyFiguresCard'

interface Props {
  message: ChatMessage
  files?: FileInfo[]
  onViewCitation?: (citation: Citation) => void
  onDeleteMessage?: (id: string) => void
  onRetryMessage?: (id: string) => void
  isLastAssistant?: boolean
}

// ── Markdown rendering context ───────────────────────────────────────────────
interface MarkdownCtx {
  files?: FileInfo[]
  onViewCitation?: (c: Citation) => void
  inferenceMode?: InferenceMode
  citations?: Citation[]
}

// ── Primary/secondary partition ───────────────────────────────────────────────
// Primary = top citation + any within 75% of the top score (max 2 primaries).
// Scores are normalized 0–1 (top is always 1.0), so this is ratio-based.
function partitionCitations(items: Citation[]): { primary: Citation[]; secondary: Citation[] } {
  if (items.length === 0) return { primary: [], secondary: [] }
  if (items.length === 1) return { primary: items, secondary: [] }
  const topScore = items[0].score
  const threshold = topScore * 0.75
  const primary: Citation[] = []
  const secondary: Citation[] = []
  for (const c of items) {
    if (primary.length < 2 && c.score >= threshold) primary.push(c)
    else secondary.push(c)
  }
  return { primary, secondary }
}

// ── Deduplicated citation list with "show all" toggle ────────────────────────
function CitationSources({ citations, onViewCitation }: { citations: Citation[]; onViewCitation?: (c: Citation) => void }): JSX.Element {
  const [showAll, setShowAll] = useState(false)
  const [secondaryOpen, setSecondaryOpen] = useState(() => citations.length <= 3)

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
  const secondarySlice = secondaryAll.slice(0, 4)

  return (
    <div className="mt-3">
      {/* Header row */}
      <div className="flex items-center justify-between mb-2">
        <p
          className="text-[10px] font-semibold uppercase tracking-[0.12em]"
          style={{ color: 'rgb(var(--ov) / 0.45)' }}
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
            style={{ color: 'rgba(201,168,76,0.65)' }}
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
              style={{ color: 'rgb(var(--ov) / 0.45)' }}
            >
              Supporting Sources ({secondarySlice.length})
            </p>
            <svg
              width="8"
              height="8"
              viewBox="0 0 10 10"
              fill="none"
              stroke="rgb(var(--ov) / 0.45)"
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
  const d = new Date(ts)
  const now = new Date()
  const time = d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
  const isToday = d.toDateString() === now.toDateString()
  const yesterday = new Date(now)
  yesterday.setDate(yesterday.getDate() - 1)
  const isYesterday = d.toDateString() === yesterday.toDateString()
  if (isToday) return time
  if (isYesterday) return `Yesterday ${time}`
  return `${d.toLocaleDateString([], { month: 'short', day: 'numeric' })} ${time}`
}

function fullTimestamp(ts: number): string {
  return new Date(ts).toLocaleString([], { weekday: 'long', year: 'numeric', month: 'long', day: 'numeric', hour: '2-digit', minute: '2-digit' })
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
      setTimeout(() => setCopied(false), 1000)
    }).catch(() => {})
  }

  return (
    <button
      onClick={handleCopy}
      title="Copy"
      aria-label="Copy to clipboard"
      className={`flex items-center justify-center rounded transition-all ${className ?? ''}`}
      style={{
        color: copied ? '#3fb950' : 'rgb(var(--ov) / 0.4)',
        ...style,
      }}
      onMouseEnter={(e) => {
        if (!copied) (e.currentTarget as HTMLButtonElement).style.color = 'rgb(var(--ov) / 0.55)'
      }}
      onMouseLeave={(e) => {
        if (!copied) (e.currentTarget as HTMLButtonElement).style.color = 'rgb(var(--ov) / 0.4)'
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

// ── Extended mode section icon helper ────────────────────────────────────────
function SectionIcon({ heading }: { heading: string }): JSX.Element | null {
  const lower = heading.toLowerCase()
  if (lower.includes('direct answer')) {
    return (
      <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="var(--gold)" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" className="shrink-0">
        <circle cx="8" cy="8" r="6" />
        <path d="M5.5 8l2 2 3.5-3.5" />
      </svg>
    )
  }
  if (lower.includes('key findings')) {
    return (
      <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="var(--gold)" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" className="shrink-0">
        <path d="M8 1C5.24 1 3 3.24 3 6c0 1.77.93 3.32 2.33 4.2V12a1 1 0 001 1h3.34a1 1 0 001-1v-1.8A5 5 0 0013 6c0-2.76-2.24-5-5-5z" />
        <line x1="6" y1="14.5" x2="10" y2="14.5" />
      </svg>
    )
  }
  if (lower.includes('relevant provisions')) {
    return (
      <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="var(--gold)" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" className="shrink-0">
        <path d="M2 1.5h4a2 2 0 012 2v11l-3-2-3 2V1.5z" />
        <path d="M14 1.5h-4a2 2 0 00-2 2v11l3-2 3 2V1.5z" />
      </svg>
    )
  }
  if (lower.includes('caveat')) {
    return (
      <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="var(--gold)" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" className="shrink-0">
        <path d="M7.13 1.65a1 1 0 011.74 0l5.87 10.3A1 1 0 0113.87 13.5H2.13a1 1 0 01-.87-1.55z" />
        <line x1="8" y1="6" x2="8" y2="9" />
        <circle cx="8" cy="11" r="0.5" fill="var(--gold)" />
      </svg>
    )
  }
  return null
}

// ── HTML entity escaping for code content ─────────────────────────────────────
function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

// ── Strip inline citations from display text ─────────────────────────────────
// Citations are shown in the SOURCES section below; remove them from the body.
// Preserves trailing periods (sentence-ending punctuation) when stripping.
function stripInlineCitations(text: string): string {
  let result = text
    // [filename.pdf, p. 1], [doc, pp. 1-3] — preserve period if present after ]
    .replace(/\s*\[[^\[\]]{1,150},\s*pp?\.\s*[\d,\s–-]+\](\.)?/g, (_, dot) => dot || '')
    // [Source 3, p. 12], [Source 3] — preserve period if present after ]
    .replace(/\s*\[Source\s+\d+[^\]]*\](\.)?/g, (_, dot) => dot || '')
    .replace(/\s+([.,;])/g, '$1') // fix orphaned spaces before punctuation

  // Rejoin orphan short lines (e.g. "U." / "S." / "00" broken across lines).
  // If a line is ≤4 chars and non-empty, join it to the previous line.
  const lines = result.split('\n')
  const joined: string[] = []
  for (const line of lines) {
    const trimmed = line.trim()
    if (trimmed.length > 0 && trimmed.length <= 4 && joined.length > 0 && joined[joined.length - 1].trim().length > 0) {
      joined[joined.length - 1] = joined[joined.length - 1].trimEnd() + trimmed
    } else {
      joined.push(line)
    }
  }
  return joined.join('\n')
}

// ── Frontend dedup: last-line-of-defense against repeated bullets ─────────────
// Strips citations before comparing so "[doc, p. 1]" vs "[doc, p.1]" match.
function deduplicateLines(text: string): string {
  const lines = text.split('\n')
  const seen: string[] = []
  const result: string[] = []
  for (const line of lines) {
    const trimmed = line.trim()
    if (!trimmed) { result.push(line); continue }
    // Strip citations and normalize for comparison
    const noCites = trimmed.replace(/\s*\[[^\]]{0,200}\]/g, '')
    const normalized = noCites.toLowerCase().replace(/[^a-z0-9\s]/g, '').replace(/\s+/g, ' ').trim()
    if (normalized.length > 25) {
      // Exact match check
      if (seen.includes(normalized)) continue
      // Jaccard fuzzy match (>0.65 word overlap = duplicate)
      const curWords = new Set(normalized.split(' '))
      const isDup = seen.some(prev => {
        const prevWords = new Set(prev.split(' '))
        let inter = 0
        for (const w of curWords) if (prevWords.has(w)) inter++
        const union = new Set([...curWords, ...prevWords]).size
        return union > 0 && inter / union > 0.65
      })
      if (isDup) continue
      seen.push(normalized)
    }
    result.push(line)
  }
  return result.join('\n')
}

// ── Lightweight markdown renderer ─────────────────────────────────────────────
function renderMarkdown(text: string, ctx?: MarkdownCtx): JSX.Element {
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
      const escapedCodeText = escapeHtml(codeText)
      elements.push(
        <div key={i} style={{ position: 'relative', margin: '8px 0' }}>
          <pre style={{ background: 'var(--bg)', border: '1px solid rgb(var(--ov) / 0.1)', borderRadius: 10, padding: '12px 16px', overflowX: 'auto', margin: 0 }}>
            <code
              style={{ fontFamily: "'SF Mono','Fira Mono',monospace", fontSize: '0.82em', color: 'rgb(var(--ov) / 0.82)', lineHeight: 1.65 }}
              data-lang={lang}
              dangerouslySetInnerHTML={{ __html: escapedCodeText }}
            />
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
            <li key={j} style={{ marginBottom: '0.35em' }}>{inlineMarkdown(item, ctx)}</li>
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
            <li key={j} style={{ marginBottom: '0.35em' }}>{inlineMarkdown(item, ctx)}</li>
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

      // Extended mode: h3 gets styled section container with icon
      if (ctx?.inferenceMode === 'extended' && level === 3) {
        elements.push(
          <div
            key={i}
            style={{
              borderLeft: '3px solid var(--gold)',
              background: 'rgba(201,168,76,0.04)',
              borderRadius: '0 8px 8px 0',
              padding: '8px 12px',
              margin: '16px 0 8px',
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
            }}
          >
            <SectionIcon heading={content} />
            <p style={{ fontWeight: 600, color: 'var(--text)', margin: 0, lineHeight: 1.3, fontSize: '0.93em' }}>
              {inlineMarkdown(content, ctx)}
            </p>
          </div>
        )
      } else {
        const style: React.CSSProperties = { fontWeight: 600, color: 'var(--text)', margin: '14px 0 3px', lineHeight: 1.3, fontSize: level === 1 ? '1.05em' : level === 2 ? '0.98em' : '0.93em' }
        elements.push(<p key={i} style={style}>{inlineMarkdown(content, ctx)}</p>)
      }
      i++
      continue
    }

    // Table
    if (line.includes('|') && line.trim().startsWith('|') && line.trim().endsWith('|')) {
      const tableLines: string[] = []
      while (i < lines.length && lines[i].includes('|') && lines[i].trim().startsWith('|')) {
        tableLines.push(lines[i])
        i++
      }
      if (tableLines.length >= 2) {
        const parseRow = (row: string): string[] =>
          row.split('|').slice(1, -1).map((c) => c.trim())
        const isSeparator = (row: string): boolean =>
          /^\|[\s:|-]+\|$/.test(row.trim())
        const sepIdx = tableLines.findIndex(isSeparator)
        const headerRow = sepIdx > 0 ? parseRow(tableLines[0]) : null
        const bodyStart = sepIdx >= 0 ? sepIdx + 1 : headerRow ? 1 : 0
        const bodyRows = tableLines.slice(bodyStart).filter((r) => !isSeparator(r)).map(parseRow)
        elements.push(
          <div key={i} style={{ overflowX: 'auto', margin: '8px 0' }}>
            <table style={{ borderCollapse: 'collapse', width: '100%', fontSize: '0.88em' }}>
              {headerRow && (
                <thead>
                  <tr>
                    {headerRow.map((cell, ci) => (
                      <th key={ci} style={{ border: '1px solid rgb(var(--ov) / 0.1)', padding: '6px 10px', background: 'rgb(var(--ov) / 0.04)', fontWeight: 600, textAlign: 'left' }}>
                        {inlineMarkdown(cell, ctx)}
                      </th>
                    ))}
                  </tr>
                </thead>
              )}
              <tbody>
                {bodyRows.map((row, ri) => (
                  <tr key={ri}>
                    {row.map((cell, ci) => (
                      <td key={ci} style={{ border: '1px solid rgb(var(--ov) / 0.1)', padding: '5px 10px' }}>
                        {inlineMarkdown(cell, ctx)}
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )
        continue
      }
      // Not a valid table — fall through, reset i
      i -= tableLines.length
    }

    // Horizontal rule
    if (/^(---|\*\*\*|___)$/.test(line.trim())) {
      elements.push(<hr key={i} style={{ border: 'none', borderTop: '1px solid rgb(var(--ov) / 0.1)', margin: '8px 0' }} />)
      i++
      continue
    }

    // Blockquote
    if (line.startsWith('> ')) {
      const quoteLines: string[] = []
      while (i < lines.length && lines[i].startsWith('> ')) {
        quoteLines.push(lines[i].slice(2))
        i++
      }
      elements.push(
        <blockquote key={i} style={{ borderLeft: '3px solid rgba(201,168,76,0.3)', paddingLeft: '1em', margin: '6px 0', color: 'rgb(var(--ov) / 0.7)' }}>
          {quoteLines.map((ql, qi) => (
            <p key={qi} style={{ margin: '2px 0' }}>{inlineMarkdown(ql, ctx)}</p>
          ))}
        </blockquote>
      )
      continue
    }

    // Blank line
    if (line.trim() === '') {
      if (elements.length > 0) {
        elements.push(<div key={i} style={{ height: '0.75em' }} />)
      }
      i++
      continue
    }

    // Skip orphaned citation fragments (e.g., "1].", "23]", "pdf, p.", "p.")
    const trimmed = line.trim()
    if (/^\d+\]\s*\.?\s*$/.test(trimmed) || /^(?:pdf\s*,?\s*)?p\.\s*$/.test(trimmed) || /^pdf,?\s*$/.test(trimmed)) {
      i++
      continue
    }

    // Normal paragraph line
    elements.push(
      <p key={i} style={{ margin: 0 }}>{inlineMarkdown(line, ctx)}</p>
    )
    i++
  }

  return <>{elements}</>
}

// Inline markdown: **bold**, *italic*, `code`, [filename, p. N] citations, [links](url)
function inlineMarkdown(text: string, ctx?: MarkdownCtx): (string | JSX.Element)[] {
  const parts: (string | JSX.Element)[] = []
  // Groups: 1=full, 2=bold text, 3=italic text, 4=code text,
  //         5=citation filename, 6=citation page,
  //         7=link text, 8=link URL
  const re = /(\*\*(.+?)\*\*|\*(.+?)\*|`([^`]+)`|\[([^,\[\]]+),\s*p\.\s*(\d+)\]|\[([^\]]+)\]\(([^)]+)\))/g
  let last = 0
  let m: RegExpExecArray | null

  while ((m = re.exec(text)) !== null) {
    if (m.index > last) parts.push(text.slice(last, m.index))
    if (m[2]) {
      // Bold
      parts.push(<strong key={m.index} style={{ color: 'var(--text)', fontWeight: 600 }}>{m[2]}</strong>)
    } else if (m[3]) {
      // Italic
      parts.push(<em key={m.index}>{m[3]}</em>)
    } else if (m[4]) {
      // Inline code
      parts.push(
        <code
          key={m.index}
          style={{ fontFamily: "'SF Mono','Fira Mono',monospace", fontSize: '0.85em', background: 'rgb(var(--ov) / 0.07)', border: '1px solid rgb(var(--ov) / 0.08)', borderRadius: 4, padding: '0.1em 0.35em' }}
          dangerouslySetInnerHTML={{ __html: escapeHtml(m[4]) }}
        />
      )
    } else if (m[5] && m[6]) {
      // Citation: [filename, p. N]
      const fileName = m[5].trim()
      const pageNumber = parseInt(m[6], 10)
      const file = ctx?.files?.find(f =>
        f.fileName === fileName || f.fileName.startsWith(fileName)
      )
      // Look up matching citation for hover preview
      const matchedCitation = ctx?.citations?.find(
        (c) => (c.fileName === fileName || c.fileName.startsWith(fileName)) && c.pageNumber === pageNumber
      )
      const previewExcerpt = matchedCitation?.excerpt
        ? matchedCitation.excerpt.length > 200
          ? matchedCitation.excerpt.slice(0, 200) + '\u2026'
          : matchedCitation.excerpt
        : null

      if (ctx?.onViewCitation && file) {
        parts.push(
          <span key={m.index} className="group/cite" style={{ position: 'relative', display: 'inline' }}>
            <button
              onClick={() => ctx.onViewCitation!({
                fileName: file.fileName,
                filePath: file.filePath,
                pageNumber,
                excerpt: matchedCitation?.excerpt ?? '',
                score: matchedCitation?.score ?? 0,
              })}
              title={`View ${fileName}, page ${pageNumber}`}
              style={{
                color: 'var(--gold)',
                background: 'none',
                border: 'none',
                font: 'inherit',
                fontSize: 'inherit',
                padding: 0,
                cursor: 'pointer',
                textDecoration: 'none',
              }}
              onMouseEnter={(e) => { (e.currentTarget as HTMLButtonElement).style.textDecoration = 'underline' }}
              onMouseLeave={(e) => { (e.currentTarget as HTMLButtonElement).style.textDecoration = 'none' }}
            >
              [{fileName}, p. {pageNumber}]
            </button>
            {previewExcerpt && (
              <span
                className="pointer-events-none absolute left-1/2 -translate-x-1/2 bottom-full mb-2 opacity-0 group-hover/cite:opacity-100 transition-opacity duration-150 z-50"
                style={{
                  width: 280,
                  padding: '8px 10px',
                  borderRadius: 8,
                  background: 'var(--bg-alt)',
                  border: '1px solid rgb(var(--ov) / 0.12)',
                  boxShadow: '0 8px 24px rgba(0,0,0,0.35)',
                  fontSize: '0.78em',
                  lineHeight: 1.5,
                  color: 'rgb(var(--ov) / 0.6)',
                  fontStyle: 'italic',
                  whiteSpace: 'normal',
                }}
              >
                "{previewExcerpt}"
              </span>
            )}
          </span>
        )
      } else {
        parts.push(
          <span key={m.index} style={{ color: 'var(--gold)', fontSize: 'inherit' }}>
            [{fileName}, p. {pageNumber}]
          </span>
        )
      }
    } else if (m[7]) {
      // Link: [text](url)
      parts.push(
        <a key={m.index} href={m[8]} target="_blank" rel="noopener noreferrer"
          style={{ color: 'var(--gold)', textDecoration: 'none' }}
          onMouseEnter={(e) => { (e.currentTarget as HTMLAnchorElement).style.textDecoration = 'underline' }}
          onMouseLeave={(e) => { (e.currentTarget as HTMLAnchorElement).style.textDecoration = 'none' }}
        >{m[7]}</a>
      )
    }
    last = m.index + m[0].length
  }

  if (last < text.length) parts.push(text.slice(last))
  return parts
}

// ── Quality badges for answer assertions ──────────────────────────────────────
function QualityBadges({ assertions }: { assertions: AssertionResult[] }): JSX.Element {
  const [expanded, setExpanded] = useState(false)
  const passed = assertions.filter((a) => a.passed).length
  const total = assertions.length
  const allPassed = passed === total
  const notices = total - passed

  const color = allPassed ? '#3fb950' : 'rgb(var(--ov) / 0.4)'
  const bgColor = allPassed ? 'rgba(63,185,80,0.08)' : 'rgb(var(--ov) / 0.03)'
  const borderColor = allPassed ? 'rgba(63,185,80,0.2)' : 'rgb(var(--ov) / 0.08)'

  return (
    <div className="mt-2">
      <button
        onClick={() => setExpanded((v) => !v)}
        className="flex items-center gap-1.5 rounded-lg px-2.5 py-1 text-[10.5px] font-medium transition-all"
        style={{ background: bgColor, border: `1px solid ${borderColor}`, color }}
      >
        {/* Shield icon */}
        <svg width="11" height="11" viewBox="0 0 16 16" fill="none" stroke={color} strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round">
          <path d="M8 1L2 4v4c0 3.5 2.5 6.5 6 7.5 3.5-1 6-4 6-7.5V4L8 1z" />
          {allPassed && <path d="M5.5 8l2 2 3.5-3.5" />}
          {!allPassed && <circle cx="8" cy="8" r="1" fill={color} />}
        </svg>
        {allPassed ? `${passed}/${total} checks passed` : `${notices} notice${notices !== 1 ? 's' : ''}`}
        <svg
          width="8" height="8" viewBox="0 0 10 10" fill="none"
          stroke={color} strokeWidth="1.8" strokeLinecap="round"
          style={{ transform: expanded ? 'rotate(180deg)' : 'rotate(0deg)', transition: 'transform 0.18s ease' }}
        >
          <path d="M2 3.5l3 3 3-3" />
        </svg>
      </button>

      {expanded && (
        <div
          className="mt-1.5 rounded-lg px-3 py-2.5 flex flex-col gap-1.5"
          style={{ background: 'rgb(var(--ov) / 0.02)', border: '1px solid rgb(var(--ov) / 0.06)' }}
        >
          {assertions.map((a, idx) => (
            <div key={idx} className="flex items-start gap-2 text-[11px]" style={{ color: 'rgb(var(--ov) / 0.6)' }}>
              {a.passed ? (
                <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="#3fb950" strokeWidth="1.6" strokeLinecap="round" className="shrink-0 mt-0.5">
                  <path d="M2 5l2 2 4-4" />
                </svg>
              ) : (
                <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="rgb(var(--ov) / 0.5)" strokeWidth="1.6" strokeLinecap="round" className="shrink-0 mt-0.5">
                  <circle cx="5" cy="5" r="1.5" fill="rgb(var(--ov) / 0.4)" stroke="none" />
                </svg>
              )}
              <span style={{ color: a.passed ? 'rgb(var(--ov) / 0.45)' : 'rgb(var(--ov) / 0.7)' }}>{a.message}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

// ── Action bar button ────────────────────────────────────────────────────────
function ActionButton({ title, onClick, children }: { title: string; onClick: (e: React.MouseEvent) => void; children: React.ReactNode }): JSX.Element {
  return (
    <button
      onClick={onClick}
      title={title}
      aria-label={title}
      className="flex items-center justify-center h-6 w-6 rounded transition-all"
      style={{ color: 'rgb(var(--ov) / 0.4)' }}
      onMouseEnter={(e) => { (e.currentTarget as HTMLButtonElement).style.color = 'rgb(var(--ov) / 0.65)' }}
      onMouseLeave={(e) => { (e.currentTarget as HTMLButtonElement).style.color = 'rgb(var(--ov) / 0.4)' }}
    >
      {children}
    </button>
  )
}

export default function MessageBubble({ message, files, onViewCitation, onDeleteMessage, onRetryMessage, isLastAssistant }: Props): JSX.Element {
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
              <div className="flex items-center gap-0.5" style={{ position: 'absolute', top: 6, right: 6 }}>
                <CopyButton text={message.content} />
                {onDeleteMessage && (
                  <ActionButton title="Delete" onClick={(e) => { e.stopPropagation(); onDeleteMessage(message.id) }}>
                    <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor">
                      <path d="M11 1.75V3h2.25a.75.75 0 0 1 0 1.5H2.75a.75.75 0 0 1 0-1.5H5V1.75C5 .784 5.784 0 6.75 0h2.5C10.216 0 11 .784 11 1.75ZM6.5 1.75V3h3V1.75a.25.25 0 0 0-.25-.25h-2.5a.25.25 0 0 0-.25.25ZM4.997 6.178a.75.75 0 1 0-1.493.144l.684 7.084A1.75 1.75 0 0 0 5.926 15h4.148a1.75 1.75 0 0 0 1.738-1.594l.684-7.084a.75.75 0 0 0-1.493-.144l-.684 7.084a.25.25 0 0 1-.245.228H5.926a.25.25 0 0 1-.245-.228L4.997 6.178Z" />
                    </svg>
                  </ActionButton>
                )}
              </div>
            )}
          </div>
          <p className="mt-1.5 text-right text-[10px] cursor-default" title={fullTimestamp(message.timestamp)} style={{ color: 'rgb(var(--ov) / 0.45)' }}>
            {formatTime(message.timestamp)}
          </p>
        </div>
      </div>
    )
  }

  const isNotFound = message.notFound
  const mdCtx: MarkdownCtx = { files, onViewCitation, inferenceMode: message.inferenceMode, citations: message.citations }

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
        </div>

        {isNotFound ? (
          /* ── Feature 4: Enhanced Not-Found State ──────────────────── */
          <div
            className="rounded-xl px-4 py-3.5"
            style={{
              background: 'rgba(201,168,76,0.05)',
              border: '1px solid rgba(201,168,76,0.15)',
              borderLeft: '3px solid var(--gold)',
            }}
          >
            <div className="flex items-start gap-2.5">
              {/* Magnifying glass icon */}
              <svg width="14" height="14" viewBox="0 0 16 16" fill="none" className="shrink-0 mt-0.5" stroke="var(--gold)" strokeWidth="1.4" strokeLinecap="round">
                <circle cx="7" cy="7" r="4.5" />
                <line x1="10.2" y1="10.2" x2="14" y2="14" />
              </svg>
              <p
                className="text-[13px] leading-relaxed whitespace-pre-wrap"
                style={{ color: 'rgb(var(--ov) / 0.85)' }}
              >
                {message.content}
              </p>
            </div>
            <hr style={{ border: 'none', borderTop: '1px solid rgba(201,168,76,0.15)', margin: '12px 0' }} />
            <div>
              <p className="text-[11px] font-semibold mb-2" style={{ color: 'var(--gold)' }}>Suggestions</p>
              <ul className="flex flex-col gap-1.5" style={{ listStyle: 'none', padding: 0, margin: 0 }}>
                {[
                  'Using specific terms from the document (names, dates, amounts)',
                  'Asking about a specific section or clause',
                  'Switching to Discovery mode for deeper analysis',
                  'Check that the relevant documents are loaded',
                  'Rephrase your question with different keywords',
                ].map((tip, idx) => (
                  <li key={idx} className="flex items-center gap-2 text-[12px]" style={{ color: 'rgb(var(--ov) / 0.6)' }}>
                    <svg width="6" height="6" viewBox="0 0 6 6" fill="var(--gold)" className="shrink-0">
                      <circle cx="3" cy="3" r="2" />
                    </svg>
                    {tip}
                  </li>
                ))}
              </ul>
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
            {/* Low confidence banner */}
            {message.confidence !== undefined && message.confidence !== null && message.confidence < 0.4 && (
              <div className="mb-2 px-3 py-1.5 bg-red-500/10 border border-red-500/20 rounded-lg text-xs text-red-400 flex items-center gap-2">
                <svg className="w-3.5 h-3.5 flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M8.485 2.495c.673-1.167 2.357-1.167 3.03 0l6.28 10.875c.673 1.167-.168 2.625-1.516 2.625H3.72c-1.347 0-2.189-1.458-1.515-2.625L8.485 2.495zM10 6a.75.75 0 01.75.75v3.5a.75.75 0 01-1.5 0v-3.5A.75.75 0 0110 6zm0 9a1 1 0 100-2 1 1 0 000 2z" clipRule="evenodd" />
                </svg>
                Low confidence — this response may contain inaccuracies. Consider rephrasing your question.
              </div>
            )}
            <div className="text-[13.5px] leading-[1.75]" style={{ color: 'var(--text)' }}>
              {message.content.trim()
                ? <>
                    {renderMarkdown(stripInlineCitations(deduplicateLines(message.content)), mdCtx)}
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
                : <span style={{ color: 'rgb(var(--ov) / 0.5)', fontStyle: 'italic' }}>No response generated. Please try again.</span>
              }
            </div>

          </div>
        )}

        <p className="mt-1.5 text-[10px] cursor-default" title={fullTimestamp(message.timestamp)} style={{ color: 'rgb(var(--ov) / 0.45)' }}>
          {formatTime(message.timestamp)}
        </p>

        {/* Feature 3: Key Figures Summary Card */}
        {!message.isStreaming && !isNotFound && message.content.trim() && (() => {
          const figures = extractKeyFigures(message.content)
          return figures.length > 0 ? <KeyFiguresCard figures={figures} /> : null
        })()}

        {!message.isStreaming && message.qualityAssertions && message.qualityAssertions.length > 0 && (
          <QualityBadges assertions={message.qualityAssertions} />
        )}

        {isLastAssistant && !message.isStreaming && onRetryMessage && (
          <button
            onClick={() => onRetryMessage(message.id)}
            aria-label="Regenerate response"
            className="flex items-center gap-1.5 mt-2 rounded-lg px-2.5 py-1.5 text-[11px] font-medium transition-all"
            style={{
              border: '1px solid rgb(var(--ov) / 0.08)',
              color: 'rgb(var(--ov) / 0.45)',
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
              el.style.color = 'rgb(var(--ov) / 0.45)'
              el.style.borderColor = 'rgb(var(--ov) / 0.08)'
              el.style.background = 'rgb(var(--ov) / 0.02)'
            }}
          >
            <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor">
              <path d="M1.705 8.005a.75.75 0 0 1 .834.656 5.5 5.5 0 0 0 9.592 2.97l-1.204-1.204a.25.25 0 0 1 .177-.427h3.646a.25.25 0 0 1 .25.25v3.646a.25.25 0 0 1-.427.177l-1.38-1.38A7.002 7.002 0 0 1 1.05 8.84a.75.75 0 0 1 .656-.834ZM8 2.5a5.487 5.487 0 0 0-4.131 1.869l1.204 1.204A.25.25 0 0 1 4.896 6H1.25A.25.25 0 0 1 1 5.75V2.104a.25.25 0 0 1 .427-.177l1.38 1.38A7.002 7.002 0 0 1 14.95 7.16a.75.75 0 0 1-1.49.178A5.5 5.5 0 0 0 8 2.5Z" />
            </svg>
            Regenerate
          </button>
        )}

        {!isNotFound && message.citations && message.citations.length > 0 && (
          <CitationSources citations={message.citations} onViewCitation={onViewCitation} />
        )}
      </div>
    </div>
  )
}

import { useEffect, useRef, useState } from 'react'
import { Citation } from '../../../../../shared/src/types'

interface Props {
  citation: Citation | null
  onClose: () => void
}

// ── Native PDF viewer ─────────────────────────────────────────────────────────
// Uses a local HTTP server (127.0.0.1:PORT) started at app launch.
// WKWebView reliably renders PDFs in iframes from http://127.0.0.1 URLs —
// this is the only origin type that triggers WKWebView's built-in PDF renderer.
// PDF.js Web Workers do NOT work in WKWebView under Tauri's tauri:// protocol.
function PdfViewer({ citation }: { citation: Citation }): JSX.Element {
  const [port, setPort] = useState<number>(0)
  const [copied, setCopied] = useState(false)

  useEffect(() => {
    ;(window as any).api.getFileServerPort().then(setPort).catch(() => setPort(0))
  }, [])

  function copyExcerpt(): void {
    navigator.clipboard.writeText(citation.excerpt).then(() => {
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    }).catch(() => {})
  }

  return (
    <div className="flex-1 flex flex-col" style={{ minHeight: 0 }}>
      {/* Excerpt strip */}
      <div
        className="shrink-0 px-4 py-2.5 flex items-center gap-3"
        style={{ background: 'rgba(201,168,76,0.04)', borderBottom: '1px solid rgba(201,168,76,0.1)', borderLeft: '2.5px solid rgba(201,168,76,0.35)' }}
      >
        <svg width="11" height="11" viewBox="0 0 16 16" fill="none" className="shrink-0">
          <circle cx="6" cy="6" r="4.5" stroke="rgba(201,168,76,0.5)" strokeWidth="1.4" />
          <path d="M10 10l4 4" stroke="rgba(201,168,76,0.5)" strokeWidth="1.4" strokeLinecap="round" />
        </svg>
        <p className="flex-1 text-[11px] italic truncate" style={{ color: 'rgb(var(--ov) / 0.38)' }}>
          "{citation.excerpt.slice(0, 100)}{citation.excerpt.length > 100 ? '...' : ''}"
        </p>
        <button
          onClick={copyExcerpt}
          title="Copy excerpt"
          aria-label="Copy excerpt"
          className="shrink-0 flex items-center gap-1.5 text-[10px] font-semibold px-2.5 py-1 rounded-md transition-all"
          style={{
            background: copied ? 'rgba(63,185,80,0.1)' : 'rgba(201,168,76,0.08)',
            border: `1px solid ${copied ? 'rgba(63,185,80,0.3)' : 'rgba(201,168,76,0.2)'}`,
            color: copied ? '#3fb950' : 'rgba(201,168,76,0.7)',
          }}
        >
          {copied ? (
            <>
              <svg width="9" height="9" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
                <path d="M2 5l2 2 4-4" />
              </svg>
              Copied
            </>
          ) : (
            <>
              <svg width="9" height="9" viewBox="0 0 16 16" fill="currentColor">
                <path d="M0 6.75C0 5.784.784 5 1.75 5h1.5a.75.75 0 0 1 0 1.5h-1.5a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-1.5a.75.75 0 0 1 1.5 0v1.5A1.75 1.75 0 0 1 9.25 16h-7.5A1.75 1.75 0 0 1 0 14.25ZM5 1.75C5 .784 5.784 0 6.75 0h7.5C15.216 0 16 .784 16 1.75v7.5A1.75 1.75 0 0 1 14.25 11h-7.5A1.75 1.75 0 0 1 5 9.25Zm1.75-.25a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-7.5a.25.25 0 0 0-.25-.25Z" />
              </svg>
              Copy excerpt
            </>
          )}
        </button>
      </div>

      {/* PDF iframe — WKWebView's native PDF renderer */}
      {!port ? (
        <div className="flex-1 flex items-center justify-center">
          <div
            className="h-5 w-5 rounded-full animate-spin"
            style={{ border: '2px solid rgba(201,168,76,0.2)', borderTopColor: '#c9a84c' }}
          />
        </div>
      ) : (
        <iframe
          key={`${citation.filePath}-${citation.pageNumber}`}
          src={`http://127.0.0.1:${port}${encodeURI(citation.filePath)}#page=${citation.pageNumber}`}
          style={{ flex: 1, width: '100%', height: '100%', border: 'none', background: '#fff' }}
          title={citation.fileName}
        />
      )}
    </div>
  )
}

// ── Text viewer (DOCX / plain text) ──────────────────────────────────────────
function TextViewer({ citation }: { citation: Citation }): JSX.Element {
  const [text, setText] = useState<string | null>(null)
  const containerRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    window.api
      .getPageText(citation.filePath, citation.pageNumber)
      .then((t) => setText(t || '(No text available for this page)'))
      .catch(() => setText('(Failed to load page text)'))
  }, [citation.filePath, citation.pageNumber])

  // Auto-scroll to the first highlighted match after text renders
  useEffect(() => {
    if (!text || !containerRef.current) return
    const mark = containerRef.current.querySelector('mark')
    if (mark) {
      requestAnimationFrame(() => {
        mark.scrollIntoView({ behavior: 'smooth', block: 'center' })
      })
    }
  }, [text])

  if (text === null) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div
          className="h-5 w-5 rounded-full animate-spin"
          style={{ border: '2px solid rgba(201,168,76,0.2)', borderTopColor: '#c9a84c' }}
        />
      </div>
    )
  }

  const needle = citation.excerpt.replace(/\s+/g, ' ').trim().slice(0, 200)
  const escapedNeedle = needle.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  const parts = escapedNeedle.length >= 10
    ? text.split(new RegExp(`(${escapedNeedle})`, 'gi'))
    : [text]

  return (
    <div ref={containerRef} className="flex-1 overflow-auto" style={{ padding: '20px 24px' }}>
      <p
        className="text-[13px] leading-[1.9] whitespace-pre-wrap"
        style={{ color: 'rgb(var(--ov) / 0.6)' }}
      >
        {parts.map((part, i) =>
          part.toLowerCase() === needle.toLowerCase() ? (
            <mark
              key={i}
              style={{
                background: 'rgba(234,197,80,0.35)',
                color: 'var(--text)',
                borderRadius: 3,
                padding: '2px 3px',
                boxShadow: '0 0 0 1px rgba(234,197,80,0.5), 0 1px 4px rgba(234,197,80,0.15)',
              }}
            >
              {part}
            </mark>
          ) : (
            part
          )
        )}
      </p>
    </div>
  )
}

// ── Relevance badge ────────────────────────────────────────────────────────────
function ScoreBadge({ score }: { score: number }): JSX.Element {
  const label = score >= 0.40 ? 'Strong' : score >= 0.22 ? 'Good' : 'Weak'
  const color = score >= 0.40 ? '#3fb950' : score >= 0.22 ? '#c9a84c' : 'rgb(var(--ov) / 0.3)'
  const bgAlpha = score >= 0.40 ? 'rgba(63,185,80,0.08)' : score >= 0.22 ? 'rgba(201,168,76,0.08)' : 'rgb(var(--ov) / 0.04)'
  const borderAlpha = score >= 0.40 ? 'rgba(63,185,80,0.18)' : score >= 0.22 ? 'rgba(201,168,76,0.18)' : 'rgb(var(--ov) / 0.08)'
  return (
    <span
      className="shrink-0 text-[9px] font-bold uppercase tracking-wider px-1.5 py-0.5 rounded"
      style={{ background: bgAlpha, color, border: `1px solid ${borderAlpha}` }}
    >
      {label}
    </span>
  )
}

// ── Main DocumentViewer panel ─────────────────────────────────────────────────
export default function DocumentViewer({ citation, onClose }: Props): JSX.Element | null {
  const ext = citation?.fileName.split('.').pop()?.toLowerCase()
  const isPdf = ext === 'pdf'

  return (
    <aside
      className="flex h-screen flex-col shrink-0"
      style={{
        width: citation ? 520 : 0,
        minWidth: citation ? 520 : 0,
        borderLeft: citation ? '1px solid rgb(var(--ov) / 0.06)' : 'none',
        background: 'var(--modal-bg)',
        overflow: 'hidden',
        transition: 'width 0.25s ease, min-width 0.25s ease',
      }}
    >
      {citation && (
        <>
          {/* Header */}
          <div
            className="drag-region flex h-11 shrink-0 items-center gap-3 px-4"
            style={{ borderBottom: '1px solid rgb(var(--ov) / 0.05)' }}
          >
            <div className="no-drag flex-1 flex items-center gap-2 min-w-0">
              <span
                className="shrink-0 text-[9px] font-bold px-1.5 py-0.5 rounded"
                style={{ background: 'rgba(201,168,76,0.1)', color: '#c9a84c' }}
              >
                {ext?.toUpperCase() ?? 'DOC'}
              </span>
              <span
                className="text-[12px] font-medium truncate"
                style={{ color: 'rgb(var(--ov) / 0.65)' }}
                title={citation.fileName}
              >
                {citation.fileName}
              </span>
              <span
                className="shrink-0 text-[11px]"
                style={{ color: 'rgb(var(--ov) / 0.45)' }}
              >
                · p.{citation.pageNumber}
              </span>
              <ScoreBadge score={citation.score} />
            </div>

            <button
              onClick={onClose}
              aria-label="Close document viewer"
              className="no-drag shrink-0 flex h-6 w-6 items-center justify-center rounded-md transition-colors"
              style={{ color: 'rgb(var(--ov) / 0.3)' }}
              onMouseEnter={(e) => {
                ;(e.currentTarget as HTMLButtonElement).style.color = 'rgb(var(--ov) / 0.8)'
                ;(e.currentTarget as HTMLButtonElement).style.background = 'rgb(var(--ov) / 0.06)'
              }}
              onMouseLeave={(e) => {
                ;(e.currentTarget as HTMLButtonElement).style.color = 'rgb(var(--ov) / 0.3)'
                ;(e.currentTarget as HTMLButtonElement).style.background = 'transparent'
              }}
            >
              <svg width="10" height="10" viewBox="0 0 12 12" fill="currentColor">
                <path d="M1.22 1.22a.75.75 0 0 1 1.06 0L6 4.94l3.72-3.72a.75.75 0 1 1 1.06 1.06L7.06 6l3.72 3.72a.75.75 0 1 1-1.06 1.06L6 7.06l-3.72 3.72a.75.75 0 0 1-1.06-1.06L4.94 6 1.22 2.28a.75.75 0 0 1 0-1.06z" />
              </svg>
            </button>
          </div>

          {/* Document body */}
          {isPdf ? (
            <PdfViewer
              key={`${citation.filePath}-${citation.pageNumber}`}
              citation={citation}
            />
          ) : (
            <TextViewer
              key={`${citation.filePath}-${citation.pageNumber}`}
              citation={citation}
            />
          )}
        </>
      )}
    </aside>
  )
}

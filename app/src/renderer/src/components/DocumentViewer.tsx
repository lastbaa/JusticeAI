import { useEffect, useState } from 'react'
import { Citation } from '../../../../../shared/src/types'

interface Props {
  citation: Citation | null
  onClose: () => void
}

// ── Native PDF viewer ─────────────────────────────────────────────────────────
// Uses a local HTTP server (127.0.0.1:PORT) started at app launch.
// WKWebView reliably renders PDFs in iframes from http://127.0.0.1 URLs —
// this is the only origin type that triggers WKWebView's built-in PDF renderer.
function PdfViewer({ citation }: { citation: Citation }): JSX.Element {
  const [port, setPort] = useState<number>(0)

  useEffect(() => {
    window.api.getFileServerPort().then(setPort).catch(() => setPort(0))
  }, [])

  if (!port) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div
          className="h-5 w-5 rounded-full animate-spin"
          style={{ border: '2px solid rgba(201,168,76,0.2)', borderTopColor: '#c9a84c' }}
        />
      </div>
    )
  }

  // encodeURI leaves path slashes intact but encodes spaces and special chars.
  // The server strips the leading '/' to reconstruct the absolute macOS path.
  const src = `http://127.0.0.1:${port}${encodeURI(citation.filePath)}#page=${citation.pageNumber}`

  return (
    <iframe
      key={src}
      src={src}
      style={{ flex: 1, width: '100%', height: '100%', border: 'none', background: '#fff' }}
      title={citation.fileName}
    />
  )
}

// ── Text viewer (DOCX / plain text) ──────────────────────────────────────────
function TextViewer({ citation }: { citation: Citation }): JSX.Element {
  const [text, setText] = useState<string | null>(null)

  useEffect(() => {
    window.api
      .getPageText(citation.filePath, citation.pageNumber)
      .then((t) => setText(t || '(No text available for this page)'))
      .catch(() => setText('(Failed to load page text)'))
  }, [citation.filePath, citation.pageNumber])

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
    <div className="flex-1 overflow-auto" style={{ padding: '20px 24px' }}>
      <p
        className="text-[13px] leading-[1.9] whitespace-pre-wrap"
        style={{ color: 'rgba(255,255,255,0.6)' }}
      >
        {parts.map((part, i) =>
          part.toLowerCase() === needle.toLowerCase() ? (
            <mark
              key={i}
              style={{
                background: 'rgba(201,168,76,0.28)',
                color: 'rgba(255,255,255,0.9)',
                borderRadius: 3,
                padding: '1px 0',
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
        borderLeft: citation ? '1px solid rgba(255,255,255,0.06)' : 'none',
        background: '#0a0a0a',
        overflow: 'hidden',
        transition: 'width 0.25s ease, min-width 0.25s ease',
      }}
    >
      {citation && (
        <>
          {/* Header */}
          <div
            className="drag-region flex h-11 shrink-0 items-center gap-3 px-4"
            style={{ borderBottom: '1px solid rgba(255,255,255,0.05)' }}
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
                style={{ color: 'rgba(255,255,255,0.65)' }}
                title={citation.fileName}
              >
                {citation.fileName}
              </span>
              <span
                className="shrink-0 text-[11px]"
                style={{ color: 'rgba(255,255,255,0.25)' }}
              >
                · p.{citation.pageNumber}
              </span>
            </div>

            <button
              onClick={onClose}
              className="no-drag shrink-0 flex h-6 w-6 items-center justify-center rounded-md transition-colors"
              style={{ color: 'rgba(255,255,255,0.3)' }}
              onMouseEnter={(e) => {
                ;(e.currentTarget as HTMLButtonElement).style.color = 'rgba(255,255,255,0.8)'
                ;(e.currentTarget as HTMLButtonElement).style.background = 'rgba(255,255,255,0.06)'
              }}
              onMouseLeave={(e) => {
                ;(e.currentTarget as HTMLButtonElement).style.color = 'rgba(255,255,255,0.3)'
                ;(e.currentTarget as HTMLButtonElement).style.background = 'transparent'
              }}
            >
              <svg width="10" height="10" viewBox="0 0 12 12" fill="currentColor">
                <path d="M1.22 1.22a.75.75 0 0 1 1.06 0L6 4.94l3.72-3.72a.75.75 0 1 1 1.06 1.06L7.06 6l3.72 3.72a.75.75 0 1 1-1.06 1.06L6 7.06l-3.72 3.72a.75.75 0 0 1-1.06-1.06L4.94 6 1.22 2.28a.75.75 0 0 1 0-1.06z" />
              </svg>
            </button>
          </div>

          {/* Excerpt strip */}
          <div
            className="shrink-0 px-4 py-3"
            style={{ borderBottom: '1px solid rgba(255,255,255,0.05)', background: '#070707' }}
          >
            <div className="flex items-center gap-1.5 mb-1">
              <div
                className="w-2 h-2 rounded-sm shrink-0"
                style={{ background: 'rgba(201,168,76,0.45)' }}
              />
              <p
                className="text-[10px] font-semibold uppercase tracking-[0.12em]"
                style={{ color: 'rgba(201,168,76,0.55)' }}
              >
                Cited passage
              </p>
            </div>
            <p
              className="text-[11px] leading-relaxed italic"
              style={{ color: 'rgba(255,255,255,0.35)' }}
            >
              "{citation.excerpt.slice(0, 180)}{citation.excerpt.length > 180 ? '…' : ''}"
            </p>
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

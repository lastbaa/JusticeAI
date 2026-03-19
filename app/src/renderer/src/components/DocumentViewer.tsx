import { useEffect, useRef, useState } from 'react'
import * as pdfjsLib from 'pdfjs-dist'
import type { TextItem, TextMarkedContent } from 'pdfjs-dist/types/src/display/api'
import workerSrc from 'pdfjs-dist/build/pdf.worker.min.mjs?url'
import { Citation } from '../../../../../shared/src/types'

// Configure PDF.js worker once at module level
pdfjsLib.GlobalWorkerOptions.workerSrc = workerSrc

interface Props {
  citation: Citation | null
  onClose: () => void
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/**
 * Given a PDF.js text content and an excerpt string, return the text items
 * that overlap with the first match of (the first 80 chars of) the excerpt.
 */
function findHighlightItems(items: TextItem[], excerpt: string): TextItem[] {
  if (items.length === 0) return []

  const norm = (s: string) => s.replace(/\s+/g, ' ').trim().toLowerCase()

  // Pre-normalise each item's text, then build the combined string and ranges
  // entirely in that normalised space so that indexOf positions map correctly.
  const ranges: { item: TextItem; start: number; end: number }[] = []
  let combined = ''
  for (const item of items) {
    const s = norm(item.str)
    if (!s) continue
    const start = combined.length
    combined += s
    ranges.push({ item, start, end: combined.length })
    combined += ' '  // single space separator — keeps combined already-normalised
  }

  // Use up to 250 chars of the excerpt so the full citation phrase is covered
  const needle = norm(excerpt).slice(0, 250)
  if (needle.length < 5) return []

  const matchStart = combined.indexOf(needle)   // combined is already lower-cased
  if (matchStart === -1) return []
  const matchEnd = matchStart + needle.length

  return ranges
    .filter((r) => r.start < matchEnd && r.end > matchStart)
    .map((r) => r.item)
}

// ── PDF Viewer ─────────────────────────────────────────────────────────────────
function PdfViewer({ citation }: { citation: Citation }): JSX.Element {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const hlCanvasRef = useRef<HTMLCanvasElement>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [currentPage, setCurrentPage] = useState(citation.pageNumber)
  const [totalPages, setTotalPages] = useState(0)
  const [copied, setCopied] = useState(false)

  // Reset current page when citation changes
  useEffect(() => {
    setCurrentPage(citation.pageNumber)
  }, [citation.filePath, citation.pageNumber])

  useEffect(() => {
    let cancelled = false
    setLoading(true)
    setError(null)

    async function render(): Promise<void> {
      try {
        // Load file bytes via Tauri IPC (avoids WKWebView fetch restrictions on http://)
        const b64: string = await (window as any).api.getFileData(citation.filePath)
        if (cancelled) return
        const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0))

        const pdfDoc = await pdfjsLib.getDocument({ data: bytes }).promise
        if (cancelled) { pdfDoc.destroy(); return }

        setTotalPages(pdfDoc.numPages)

        const page = await pdfDoc.getPage(currentPage)
        if (cancelled) return

        // Scale to fit the 520px panel (with 32px padding on each side → 456px usable)
        const unscaled = page.getViewport({ scale: 1 })
        const scale = 456 / unscaled.width
        const viewport = page.getViewport({ scale })

        // ── Render PDF page to main canvas ──────────────────────────────────
        const canvas = canvasRef.current
        const hlCanvas = hlCanvasRef.current
        if (!canvas || !hlCanvas || cancelled) return

        canvas.width = viewport.width
        canvas.height = viewport.height
        hlCanvas.width = viewport.width
        hlCanvas.height = viewport.height

        await page.render({ canvas, viewport }).promise
        if (cancelled) return

        // ── Draw highlight overlay ───────────────────────────────────────────
        // Wrapped in its own try/catch: if text extraction fails the PDF still shows.
        // We use streamTextContent() + getReader().read() instead of getTextContent()
        // because getTextContent() uses `for await...of` on a ReadableStream which
        // requires ReadableStream[Symbol.asyncIterator] — not available in all WKWebViews.
        try {
          const hlCtx = hlCanvas.getContext('2d')
          if (hlCtx && !cancelled) {
            const stream = page.streamTextContent()
            const reader = stream.getReader()
            const allItems: TextItem[] = []
            while (true) {
              const { done, value } = await reader.read()
              if (done) break
              for (const it of (value as { items: Array<TextItem | TextMarkedContent> }).items) {
                if ('str' in it && it.str.length > 0) allItems.push(it)
              }
            }
            reader.releaseLock()

            if (!cancelled) {
              const matchedItems = findHighlightItems(allItems, citation.excerpt)

              hlCtx.clearRect(0, 0, hlCanvas.width, hlCanvas.height)
              hlCtx.fillStyle = 'rgba(201, 168, 76, 0.38)'

              for (const item of matchedItems) {
                // applyTransform mutates the point array in-place (returns void in v5)
                const pt = [item.transform[4], item.transform[5]]
                pdfjsLib.Util.applyTransform(pt, viewport.transform)
                const tx = pt[0]
                const ty = pt[1]
                const fontH = Math.abs(item.transform[3]) * viewport.scale || 12
                const w = item.width * viewport.scale

                // ty is the text baseline in canvas coords; draw rect upward from baseline
                hlCtx.fillRect(tx, ty - fontH * 1.15, w, fontH * 1.25)
              }
            }
          }
        } catch {
          // Highlight extraction failed — PDF still shows without highlights
        }

        setLoading(false)
      } catch (err) {
        if (!cancelled) {
          console.error('PdfViewer render error:', err)
          setError(err instanceof Error ? err.message : String(err))
          setLoading(false)
        }
      }
    }

    render()
    return () => { cancelled = true }
  }, [citation.filePath, currentPage, citation.excerpt])

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
        style={{ background: 'rgba(201,168,76,0.04)', borderBottom: '1px solid rgba(201,168,76,0.1)' }}
      >
        <svg width="11" height="11" viewBox="0 0 16 16" fill="none" className="shrink-0">
          <circle cx="6" cy="6" r="4.5" stroke="rgba(201,168,76,0.5)" strokeWidth="1.4" />
          <path d="M10 10l4 4" stroke="rgba(201,168,76,0.5)" strokeWidth="1.4" strokeLinecap="round" />
        </svg>
        <p className="flex-1 text-[11px] italic truncate" style={{ color: 'rgb(var(--ov) / 0.38)' }}>
          "{citation.excerpt.slice(0, 100)}{citation.excerpt.length > 100 ? '…' : ''}"
        </p>
        <button
          onClick={copyExcerpt}
          title="Copy excerpt"
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

      {/* Canvas viewport — white bg matches PDF page colour */}
      <div
        className="flex-1 overflow-auto flex justify-center"
        style={{ padding: '16px', background: '#f5f5f5' }}
      >
        {loading && (
          <div className="flex items-center justify-center" style={{ minHeight: 200, width: '100%' }}>
            <div
              className="h-5 w-5 rounded-full animate-spin"
              style={{ border: '2px solid rgba(201,168,76,0.2)', borderTopColor: '#c9a84c' }}
            />
          </div>
        )}
        {error && (
          <div
            className="flex items-center justify-center text-[12px] text-center"
            style={{ minHeight: 200, width: '100%', color: 'rgba(201,168,76,0.5)', padding: '0 24px' }}
          >
            Could not render PDF — {error}
          </div>
        )}
        {!error && (
          <div style={{ position: 'relative', display: loading ? 'none' : 'block' }}>
            <canvas ref={canvasRef} style={{ display: 'block', boxShadow: '0 2px 12px rgba(0,0,0,0.18)' }} />
            <canvas
              ref={hlCanvasRef}
              style={{ position: 'absolute', top: 0, left: 0, pointerEvents: 'none' }}
            />
          </div>
        )}
      </div>

      {/* Page navigation */}
      {totalPages > 1 && (
        <div
          className="shrink-0 flex items-center justify-center gap-3 py-2"
          style={{ borderTop: '1px solid rgba(201,168,76,0.08)' }}
        >
          <button
            onClick={() => setCurrentPage((p) => Math.max(1, p - 1))}
            disabled={currentPage <= 1}
            className="flex items-center justify-center h-6 w-6 rounded text-[11px] transition-all"
            style={{
              background: currentPage <= 1 ? 'transparent' : 'rgba(201,168,76,0.08)',
              color: currentPage <= 1 ? 'rgb(var(--ov) / 0.2)' : 'rgba(201,168,76,0.7)',
              border: '1px solid rgba(201,168,76,0.15)',
              cursor: currentPage <= 1 ? 'default' : 'pointer',
            }}
          >
            ‹
          </button>
          <span className="text-[11px]" style={{ color: 'rgb(var(--ov) / 0.35)' }}>
            Page {currentPage} of {totalPages}
          </span>
          <button
            onClick={() => setCurrentPage((p) => Math.min(totalPages, p + 1))}
            disabled={currentPage >= totalPages}
            className="flex items-center justify-center h-6 w-6 rounded text-[11px] transition-all"
            style={{
              background: currentPage >= totalPages ? 'transparent' : 'rgba(201,168,76,0.08)',
              color: currentPage >= totalPages ? 'rgb(var(--ov) / 0.2)' : 'rgba(201,168,76,0.7)',
              border: '1px solid rgba(201,168,76,0.15)',
              cursor: currentPage >= totalPages ? 'default' : 'pointer',
            }}
          >
            ›
          </button>
        </div>
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
      // Small delay so layout is stable before scrolling
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
                background: 'rgba(201,168,76,0.32)',
                color: 'var(--text)',
                borderRadius: 3,
                padding: '1px 2px',
                boxShadow: '0 0 0 1px rgba(201,168,76,0.4)',
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
  return (
    <span
      className="shrink-0 text-[9px] font-semibold px-1.5 py-0.5 rounded"
      style={{ background: `${color}18`, color, border: `1px solid ${color}30` }}
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
                style={{ color: 'rgb(var(--ov) / 0.25)' }}
              >
                · p.{citation.pageNumber}
              </span>
              <ScoreBadge score={citation.score} />
            </div>

            <button
              onClick={onClose}
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

          {/* Document body — PDF has canvas+highlight; text has inline highlights */}
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

import { useEffect, useRef, useState } from 'react'
import { Citation } from '../../../../../shared/src/types'
import * as pdfjsLib from 'pdfjs-dist'
import type { TextItem } from 'pdfjs-dist/types/src/display/api'

// Configure PDF.js worker — use the bundled worker via ESM URL
pdfjsLib.GlobalWorkerOptions.workerSrc = new URL(
  'pdfjs-dist/build/pdf.worker.min.mjs',
  import.meta.url
).href

interface Props {
  citation: Citation | null
  onClose: () => void
}

// ── Text position finder ─────────────────────────────────────────────────────
interface HighlightRect {
  x: number
  y: number
  w: number
  h: number
}

/**
 * Finds canvas-space bounding boxes for the excerpt within the page's text items.
 *
 * Key design: we build a single normalized string from the PDF text items,
 * tracking each item's [start, end) in that *same* string. Then we search
 * the excerpt (also normalized) in the same string — so positions always match.
 */
function findHighlightRects(
  items: TextItem[],
  excerpt: string,
  viewport: pdfjsLib.PageViewport
): HighlightRect[] {
  // Build a normalized string from items, tracking each item's position in it.
  let norm = ''
  const meta: { start: number; end: number; item: TextItem }[] = []

  for (const item of items) {
    if (!item.str) continue
    // Normalize this item's whitespace before adding to norm
    const s = item.str.replace(/\s+/g, ' ')
    if (!s.trim()) continue
    // Ensure words from adjacent items don't merge
    if (norm.length > 0 && !norm.endsWith(' ') && !s.startsWith(' ')) {
      norm += ' '
    }
    const start = norm.length
    norm += s
    meta.push({ start, end: norm.length, item })
  }

  const haystack = norm.toLowerCase()

  // Try progressively shorter needles so partial PDF-text differences still match
  const rawNeedle = excerpt.replace(/\s+/g, ' ').trim().toLowerCase()
  const candidates = [
    rawNeedle.slice(0, 200),
    rawNeedle.slice(0, 120),
    rawNeedle.slice(0, 60),
  ]

  let foundAt = -1
  let needle = ''
  for (const c of candidates) {
    if (c.length < 10) break
    const idx = haystack.indexOf(c)
    if (idx !== -1) {
      foundAt = idx
      needle = c
      break
    }
  }
  if (foundAt === -1) return []

  const foundEnd = foundAt + needle.length
  const rects: HighlightRect[] = []

  for (const { start, end, item } of meta) {
    if (end <= foundAt || start >= foundEnd) continue

    // Compute canvas coordinates from the item's affine transform
    const tx = pdfjsLib.Util.transform(viewport.transform, item.transform)
    // tx = [a, b, c, d, e, f]
    const x = tx[4]
    const y = tx[5]
    // Vertical scale → glyph height in canvas pixels
    const h = Math.sqrt(tx[2] * tx[2] + tx[3] * tx[3])
    // Horizontal scale → used to convert item.width (user units) → canvas pixels
    const xScale = Math.sqrt(tx[0] * tx[0] + tx[1] * tx[1])
    const w = item.width * xScale

    // y is the text baseline; move up by h to get the top of the glyph
    rects.push({ x, y: y - h, w, h: h * 1.3 })
  }

  return rects
}

// ── PDF Viewer ───────────────────────────────────────────────────────────────
function PdfViewer({ citation }: { citation: Citation }): JSX.Element {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const scrollRef = useRef<HTMLDivElement>(null)
  const [status, setStatus] = useState<'loading' | 'done' | 'error'>('loading')
  const [totalPages, setTotalPages] = useState<number>(0)
  const SCALE = 1.6

  useEffect(() => {
    let cancelled = false
    setStatus('loading')

    async function render(): Promise<void> {
      try {
        const b64 = await window.api.getFileData(citation.filePath)
        if (cancelled) return

        // Decode base64 → Uint8Array
        const binary = atob(b64)
        const bytes = new Uint8Array(binary.length)
        for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i)

        const pdf = await pdfjsLib.getDocument({ data: bytes }).promise
        if (cancelled) return

        setTotalPages(pdf.numPages)
        const pageNum = Math.min(citation.pageNumber, pdf.numPages)
        const page = await pdf.getPage(pageNum)
        if (cancelled) return

        const viewport = page.getViewport({ scale: SCALE })
        const canvas = canvasRef.current
        if (!canvas) return

        canvas.width = viewport.width
        canvas.height = viewport.height
        const ctx = canvas.getContext('2d')
        if (!ctx) {
          if (!cancelled) setStatus('error')
          return
        }

        // Render the PDF page
        await page.render({ canvasContext: ctx, viewport, canvas }).promise
        if (cancelled) return

        // Overlay highlights
        const textContent = await page.getTextContent()
        if (cancelled) return

        const textItems = textContent.items.filter((i): i is TextItem => 'str' in i)
        const rects = findHighlightRects(textItems, citation.excerpt, viewport)

        if (rects.length > 0) {
          ctx.save()
          ctx.fillStyle = 'rgba(201, 168, 76, 0.32)'
          ctx.strokeStyle = 'rgba(201, 168, 76, 0.7)'
          ctx.lineWidth = 1.5
          for (const r of rects) {
            ctx.fillRect(r.x, r.y, r.w, r.h)
            ctx.strokeRect(r.x, r.y, r.w, r.h)
          }
          ctx.restore()
        }

        setStatus('done')

        // After React paints the canvas as visible, scroll to the first highlight
        if (rects.length > 0) {
          const firstRect = rects[0]
          requestAnimationFrame(() => {
            if (!cancelled && scrollRef.current && canvasRef.current) {
              const cssScale = canvasRef.current.clientWidth / canvasRef.current.width
              // Scroll so the highlight is roughly 1/4 from the top
              scrollRef.current.scrollTop = Math.max(0, firstRect.y * cssScale - 100)
            }
          })
        }
      } catch (err) {
        console.error('DocumentViewer render error:', err)
        if (!cancelled) setStatus('error')
      }
    }

    render()
    return () => { cancelled = true }
  }, [citation.filePath, citation.pageNumber, citation.excerpt])

  return (
    <div ref={scrollRef} className="flex-1 overflow-auto" style={{ background: '#111' }}>
      {status === 'loading' && (
        <div className="flex h-full items-center justify-center">
          <div className="flex flex-col items-center gap-3">
            <div
              className="h-5 w-5 rounded-full animate-spin"
              style={{ border: '2px solid rgba(201,168,76,0.2)', borderTopColor: '#c9a84c' }}
            />
            <p className="text-[11px]" style={{ color: 'rgba(255,255,255,0.3)' }}>
              Loading page {citation.pageNumber}…
            </p>
          </div>
        </div>
      )}
      {status === 'error' && (
        <div className="flex h-full items-center justify-center px-6 text-center">
          <div>
            <p className="text-[13px] font-medium" style={{ color: 'rgba(248,81,73,0.8)' }}>
              Could not render this document
            </p>
            <p className="mt-1 text-[11px]" style={{ color: 'rgba(255,255,255,0.25)' }}>
              The file may have moved or is password-protected.
            </p>
          </div>
        </div>
      )}
      <div style={{ display: status === 'done' ? 'block' : 'none', padding: '16px' }}>
        {totalPages > 0 && (
          <p
            className="mb-3 text-center text-[10px] font-semibold uppercase tracking-[0.12em]"
            style={{ color: 'rgba(255,255,255,0.18)' }}
          >
            Page {citation.pageNumber} of {totalPages}
          </p>
        )}
        <canvas
          ref={canvasRef}
          className="rounded-xl shadow-2xl mx-auto block"
          style={{ maxWidth: '100%' }}
        />
      </div>
    </div>
  )
}

// ── DOCX / text fallback ─────────────────────────────────────────────────────
function TextViewer({ citation }: { citation: Citation }): JSX.Element {
  const [text, setText] = useState<string | null>(null)

  useEffect(() => {
    window.api
      .getPageText(citation.filePath, citation.pageNumber)
      .then((t) => setText(t || '(No text extracted for this page)'))
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

  // Highlight the excerpt in the text
  const needle = citation.excerpt.replace(/\s+/g, ' ').trim().slice(0, 200)
  const parts = text.split(new RegExp(`(${needle.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi'))

  return (
    <div className="flex-1 overflow-auto p-5">
      <p
        className="text-[12.5px] leading-relaxed whitespace-pre-wrap font-mono"
        style={{ color: 'rgba(255,255,255,0.55)' }}
      >
        {parts.map((part, i) =>
          part.toLowerCase() === needle.toLowerCase() ? (
            <mark
              key={i}
              style={{ background: 'rgba(201,168,76,0.35)', color: 'white', borderRadius: 2 }}
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
              {/* File type badge */}
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
                (e.currentTarget as HTMLButtonElement).style.color = 'rgba(255,255,255,0.8)'
                ;(e.currentTarget as HTMLButtonElement).style.background = 'rgba(255,255,255,0.06)'
              }}
              onMouseLeave={(e) => {
                (e.currentTarget as HTMLButtonElement).style.color = 'rgba(255,255,255,0.3)'
                ;(e.currentTarget as HTMLButtonElement).style.background = 'transparent'
              }}
            >
              <svg width="10" height="10" viewBox="0 0 12 12" fill="currentColor">
                <path d="M1.22 1.22a.75.75 0 0 1 1.06 0L6 4.94l3.72-3.72a.75.75 0 1 1 1.06 1.06L7.06 6l3.72 3.72a.75.75 0 1 1-1.06 1.06L6 7.06l-3.72 3.72a.75.75 0 0 1-1.06-1.06L4.94 6 1.22 2.28a.75.75 0 0 1 0-1.06z" />
              </svg>
            </button>
          </div>

          {/* Excerpt strip — shows exactly what text is highlighted */}
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
                Highlighted in document
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
            <PdfViewer key={`${citation.filePath}-${citation.pageNumber}-${citation.excerpt.slice(0,40)}`} citation={citation} />
          ) : (
            <TextViewer key={`${citation.filePath}-${citation.pageNumber}`} citation={citation} />
          )}
        </>
      )}
    </aside>
  )
}

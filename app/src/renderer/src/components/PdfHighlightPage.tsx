/**
 * PdfHighlightPage — Renders a single PDF page as an image with highlight overlays.
 *
 * Uses macOS CoreGraphics (via Tauri command) to render the PDF page to a PNG image,
 * then overlays highlight rectangles based on text positions extracted from the PDF.
 *
 * This avoids all pdfjs-dist/WKWebView/worker/CSP compatibility issues by doing
 * the heavy lifting in Rust and only displaying an <img> + overlay divs in React.
 */

import { useEffect, useRef, useState, useCallback } from 'react'

// ── Types ────────────────────────────────────────────────────────────────────

interface HighlightRect {
  left: number
  top: number
  width: number
  height: number
}

interface Props {
  /** Absolute filesystem path to the PDF */
  filePath: string
  /** 1-based page number */
  pageNumber: number
  /** Excerpt text to highlight on the page */
  excerpt: string
  /** Called when user wants to dismiss the highlight view */
  onDismiss: () => void
}

// ── Text position extraction using pdfjs getTextContent (no canvas needed) ──

let pdfjsLib: typeof import('pdfjs-dist') | null = null
let pdfjsLoadPromise: Promise<typeof import('pdfjs-dist')> | null = null

function getPdfjs(): Promise<typeof import('pdfjs-dist')> {
  if (pdfjsLib) return Promise.resolve(pdfjsLib)
  if (!pdfjsLoadPromise) {
    pdfjsLoadPromise = import('pdfjs-dist').then((mod) => {
      pdfjsLib = mod
      // No workerSrc — fake worker mode (main thread). Only used for text extraction.
      return mod
    })
  }
  return pdfjsLoadPromise
}

interface TextItem {
  str: string
  transform: number[]
  width: number
  height: number
  idx: number
}

async function extractTextItems(fileUrl: string, pageNumber: number): Promise<{ items: TextItem[]; pdfWidth: number; pdfHeight: number }> {
  const pdfjs = await getPdfjs()
  // Fetch PDF bytes on main thread to avoid worker network issues
  const resp = await fetch(fileUrl)
  if (!resp.ok) throw new Error(`HTTP ${resp.status}`)
  const data = await resp.arrayBuffer()
  const pdf = await pdfjs.getDocument({ data }).promise
  const page = await pdf.getPage(pageNumber)
  const viewport = page.getViewport({ scale: 1.0 })

  const textContent = await page.getTextContent()
  const items = textContent.items
    .filter((item: any) => 'str' in item && item.str)
    .map((item: any, idx: number) => ({
      str: item.str as string,
      transform: item.transform as number[],
      width: item.width as number,
      height: item.height as number,
      idx,
    }))

  return { items, pdfWidth: viewport.width, pdfHeight: viewport.height }
}

// ── Fuzzy text matcher ──────────────────────────────────────────────────────

function normalize(text: string): string {
  return text
    .replace(/[\u{E000}-\u{F8FF}\u{FFF0}-\u{FFFF}]/gu, '')
    .replace(/[^\S\n]+/g, ' ')
    .trim()
    .toLowerCase()
}

function findMatchingItems(
  textItems: { str: string; idx: number }[],
  excerpt: string,
): { startIdx: number; endIdx: number } | null {
  if (!excerpt || excerpt.length < 8) return null

  const fullText: string[] = []
  const charToItem: number[] = []

  for (let i = 0; i < textItems.length; i++) {
    const str = textItems[i].str
    for (let c = 0; c < str.length; c++) {
      fullText.push(str[c])
      charToItem.push(i)
    }
    if (i < textItems.length - 1) {
      fullText.push(' ')
      charToItem.push(i)
    }
  }

  const joined = fullText.join('')
  const normPage = normalize(joined)
  const normExcerpt = normalize(excerpt)

  if (normExcerpt.length < 8) return null

  let idx = normPage.indexOf(normExcerpt)

  if (idx === -1) {
    for (
      let len = Math.floor(normExcerpt.length * 0.7);
      len >= Math.min(35, normExcerpt.length);
      len = Math.floor(len * 0.75)
    ) {
      idx = normPage.indexOf(normExcerpt.slice(0, len))
      if (idx !== -1) break
    }
  }

  if (idx === -1) {
    const sentences = normExcerpt
      .split(/[.!?]+\s*/)
      .map((s) => s.trim())
      .filter((s) => s.length > 15)
    for (const sentence of sentences) {
      idx = normPage.indexOf(sentence)
      if (idx !== -1) break
    }
  }

  if (idx === -1) return null

  const normCharsToOrig: number[] = []
  {
    let lastWasSpace = false
    const lowerJoined = joined.toLowerCase()
    for (let i = 0; i < lowerJoined.length; i++) {
      const code = joined.charCodeAt(i)
      if (code >= 0xe000 && code <= 0xf8ff) continue
      if (code >= 0xfff0) continue
      if (/\s/.test(lowerJoined[i])) {
        if (!lastWasSpace && normCharsToOrig.length > 0) {
          normCharsToOrig.push(i)
          lastWasSpace = true
        }
        continue
      }
      normCharsToOrig.push(i)
      lastWasSpace = false
    }
  }

  const origStart = normCharsToOrig[idx] ?? 0
  const matchEndNorm = idx + normExcerpt.length - 1
  const origEnd = (normCharsToOrig[Math.min(matchEndNorm, normCharsToOrig.length - 1)] ?? joined.length - 1) + 1

  const startItemIdx = charToItem[Math.min(origStart, charToItem.length - 1)]
  const endItemIdx = charToItem[Math.min(origEnd - 1, charToItem.length - 1)]

  if (startItemIdx === undefined || endItemIdx === undefined) return null

  return { startIdx: startItemIdx, endIdx: endItemIdx + 1 }
}

// ── Component ────────────────────────────────────────────────────────────────

export default function PdfHighlightPage({
  filePath,
  pageNumber,
  excerpt,
  onDismiss,
}: Props): JSX.Element {
  const containerRef = useRef<HTMLDivElement>(null)
  const [imageSrc, setImageSrc] = useState<string | null>(null)
  const [imageSize, setImageSize] = useState<{ w: number; h: number }>({ w: 0, h: 0 })
  const [pdfSize, setPdfSize] = useState<{ w: number; h: number }>({ w: 0, h: 0 })
  const [highlights, setHighlights] = useState<HighlightRect[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const render = useCallback(async () => {
    setLoading(true)
    setError(null)
    setHighlights([])
    setImageSrc(null)

    try {
      const api = (window as any).api

      // Step 1: Render PDF page to PNG via Rust/CoreGraphics
      const renderScale = 2.0 // Retina quality
      const result = await api.renderPdfPage(filePath, pageNumber, renderScale)
      setImageSrc(`data:image/png;base64,${result.image_base64}`)
      setPdfSize({ w: result.pdf_width, h: result.pdf_height })

      // Display at 1x size (image is 2x for sharpness)
      const displayW = result.pdf_width
      const displayH = result.pdf_height
      setImageSize({ w: displayW, h: displayH })

      // Step 2: Get text positions via pdfjs getTextContent (no canvas needed)
      // Build file server URL for pdfjs to fetch
      const port = await api.getFileServerPort()
      const fileUrl = `http://127.0.0.1:${port}${encodeURI(filePath)}`

      let textItems: TextItem[] = []
      let textPdfWidth = result.pdf_width
      let textPdfHeight = result.pdf_height

      try {
        const textResult = await extractTextItems(fileUrl, pageNumber)
        textItems = textResult.items
        textPdfWidth = textResult.pdfWidth
        textPdfHeight = textResult.pdfHeight
      } catch (textErr) {
        console.warn('Text extraction failed, showing page without highlights:', textErr)
        // Still show the page image, just without highlights
      }

      // Step 3: Find matching text and compute highlight rects
      if (textItems.length > 0) {
        const match = findMatchingItems(
          textItems.map((t) => ({ str: t.str, idx: t.idx })),
          excerpt,
        )

        if (match) {
          const rects: HighlightRect[] = []
          // Scale from PDF points to display pixels
          const scaleX = displayW / textPdfWidth
          const scaleY = displayH / textPdfHeight

          for (let i = match.startIdx; i < match.endIdx && i < textItems.length; i++) {
            const item = textItems[i]
            if (!item.str.trim()) continue

            // PDF coordinates: origin bottom-left, Y up
            // Display coordinates: origin top-left, Y down
            const fontHeight = Math.abs(item.transform[3])
            const x = item.transform[4] * scaleX
            const y = (textPdfHeight - item.transform[5]) * scaleY - (fontHeight * scaleY)
            const w = item.width * scaleX
            const h = fontHeight * scaleY + 2

            rects.push({ left: x, top: y, width: w, height: h })
          }
          setHighlights(rects)
        }
      }

      setLoading(false)
    } catch (err: any) {
      console.error('PdfHighlightPage error:', err)
      setError(`Failed to render PDF page: ${err?.message ?? err}`)
      setLoading(false)
    }
  }, [filePath, pageNumber, excerpt])

  useEffect(() => {
    render()
  }, [render])

  // Scroll first highlight into view
  useEffect(() => {
    if (highlights.length === 0 || !containerRef.current) return
    const firstHighlight = containerRef.current.querySelector('[data-highlight]')
    if (firstHighlight) {
      requestAnimationFrame(() => {
        firstHighlight.scrollIntoView({ behavior: 'smooth', block: 'center' })
      })
    }
  }, [highlights])

  if (error) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center gap-2" style={{ color: 'rgb(var(--ov) / 0.4)' }}>
        <span className="text-xs">{error}</span>
        <button
          onClick={onDismiss}
          className="text-[10px] px-3 py-1 rounded"
          style={{ background: 'rgba(201,168,76,0.1)', color: '#c9a84c' }}
        >
          Back to PDF
        </button>
      </div>
    )
  }

  return (
    <div
      ref={containerRef}
      className="flex-1 overflow-auto"
      style={{ background: '#525659', minHeight: 0 }}
    >
      {loading && (
        <div className="flex items-center justify-center py-8">
          <div
            className="h-5 w-5 rounded-full animate-spin"
            style={{ border: '2px solid rgba(201,168,76,0.2)', borderTopColor: '#c9a84c' }}
          />
        </div>
      )}

      {imageSrc && !loading && (
        <div
          style={{
            position: 'relative',
            display: 'inline-block',
            margin: '16px auto',
          }}
        >
          <img
            src={imageSrc}
            alt={`PDF page ${pageNumber}`}
            style={{
              display: 'block',
              width: imageSize.w,
              height: imageSize.h,
              boxShadow: '0 2px 12px rgba(0,0,0,0.3)',
            }}
          />

          {/* Highlight overlays */}
          {highlights.map((rect, i) => (
            <div
              key={i}
              data-highlight
              style={{
                position: 'absolute',
                left: rect.left,
                top: rect.top,
                width: rect.width,
                height: rect.height,
                backgroundColor: 'rgba(253, 224, 71, 0.35)',
                borderRadius: 2,
                pointerEvents: 'none',
                mixBlendMode: 'multiply',
                transition: 'opacity 0.3s ease',
              }}
            />
          ))}
        </div>
      )}
    </div>
  )
}

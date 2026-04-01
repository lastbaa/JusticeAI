import { useEffect, useRef, useState } from 'react'
import { Citation } from '../../../../../shared/src/types'

interface Props {
  citation: Citation | null
  onClose: () => void
}

// ── Fuzzy text highlight matcher ──────────────────────────────────────────────
// The excerpt from best_excerpt() is sanitized & re-joined differently from the
// raw page text returned by get_page_text(), so exact regex matching is fragile.
// This normalizes both sides, finds the match, and maps back to original positions.

interface CharMap {
  normalized: string
  /** toOriginal[i] = index in original text for normalized char i */
  toOriginal: number[]
}

function buildCharMap(text: string): CharMap {
  const chars: string[] = []
  const toOriginal: number[] = []
  let lastWasSpace = false

  for (let i = 0; i < text.length; i++) {
    const code = text.charCodeAt(i)

    // Skip PUA (U+E000–F8FF), specials (U+FFF0+), and control chars
    // (matching Rust best_excerpt sanitization)
    if (code >= 0xe000 && code <= 0xf8ff) continue
    if (code >= 0xfff0) continue
    if (code < 0x20 && code !== 0x0a && code !== 0x09) continue

    // Collapse all whitespace (space, tab, newline) to single space
    if (/\s/.test(text[i])) {
      if (!lastWasSpace && chars.length > 0) {
        chars.push(' ')
        toOriginal.push(i)
        lastWasSpace = true
      }
      continue
    }

    chars.push(text[i])
    toOriginal.push(i)
    lastWasSpace = false
  }

  return { normalized: chars.join(''), toOriginal }
}

function findHighlightRange(
  pageText: string,
  excerpt: string,
): { start: number; end: number } | null {
  if (!excerpt || excerpt.length < 8) return null

  const pageMap = buildCharMap(pageText)
  const excMap = buildCharMap(excerpt)
  const normPage = pageMap.normalized.toLowerCase()
  const normExcerpt = excMap.normalized.toLowerCase()

  if (normExcerpt.length < 8) return null

  function mapRange(normStart: number, normLen: number): { start: number; end: number } {
    const s = pageMap.toOriginal[normStart]
    const eIdx = Math.min(normStart + normLen - 1, pageMap.toOriginal.length - 1)
    const e = pageMap.toOriginal[eIdx] + 1
    return { start: s, end: e }
  }

  // Strategy 1: Full normalized exact match
  let idx = normPage.indexOf(normExcerpt)
  if (idx !== -1) return mapRange(idx, normExcerpt.length)

  // Strategy 2: Progressively shorter prefixes
  for (
    let len = Math.floor(normExcerpt.length * 0.7);
    len >= Math.min(35, normExcerpt.length);
    len = Math.floor(len * 0.75)
  ) {
    idx = normPage.indexOf(normExcerpt.slice(0, len))
    if (idx !== -1) return mapRange(idx, len)
  }

  // Strategy 3: Try matching individual sentences from the excerpt
  const sentences = normExcerpt
    .split(/[.!?]+\s*/)
    .map((s) => s.trim())
    .filter((s) => s.length > 15)
  if (sentences.length >= 2) {
    const firstIdx = normPage.indexOf(sentences[0])
    const lastSentence = sentences[sentences.length - 1]
    const lastIdx = normPage.indexOf(lastSentence, firstIdx !== -1 ? firstIdx : 0)
    if (firstIdx !== -1 && lastIdx !== -1 && lastIdx >= firstIdx) {
      return mapRange(firstIdx, lastIdx - firstIdx + lastSentence.length)
    }
  }
  for (const sentence of sentences) {
    idx = normPage.indexOf(sentence)
    if (idx !== -1) return mapRange(idx, sentence.length)
  }

  // Strategy 4: Word n-gram sliding window
  const excerptWords = normExcerpt.split(/\s+/).filter((w) => w.length > 2)
  if (excerptWords.length < 3) return null

  const excerptWordSet = new Set(excerptWords)
  const wordEntries: { word: string; start: number; end: number }[] = []
  const wordRe = /\S+/g
  let wm: RegExpExecArray | null
  while ((wm = wordRe.exec(normPage)) !== null) {
    wordEntries.push({ word: wm[0], start: wm.index, end: wm.index + wm[0].length })
  }

  const windowSize = Math.min(excerptWords.length, wordEntries.length)
  let bestScore = 0
  let bestStart = -1
  let bestEnd = -1

  for (let i = 0; i <= wordEntries.length - windowSize; i++) {
    let score = 0
    for (let j = i; j < i + windowSize; j++) {
      if (excerptWordSet.has(wordEntries[j].word)) score++
    }
    if (score > bestScore) {
      bestScore = score
      bestStart = wordEntries[i].start
      bestEnd = wordEntries[Math.min(i + windowSize - 1, wordEntries.length - 1)].end
    }
  }

  if (bestScore >= excerptWords.length * 0.4 && bestStart !== -1) {
    return mapRange(bestStart, bestEnd - bestStart)
  }

  return null
}

// ── Highlighted text panel ─────────────────────────────────────────────────────
function HighlightedText({ citation }: { citation: Citation }): JSX.Element | null {
  const [text, setText] = useState<string | null>(null)
  const [expanded, setExpanded] = useState(true)
  const hlRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    ;(window as any).api
      .getPageText(citation.filePath, citation.pageNumber)
      .then((t: string) => setText(t || ''))
      .catch(() => setText(''))
  }, [citation.filePath, citation.pageNumber])

  useEffect(() => {
    if (!text || !expanded || !hlRef.current) return
    const mark = hlRef.current.querySelector('mark')
    if (mark) {
      requestAnimationFrame(() => {
        mark.scrollIntoView({ behavior: 'smooth', block: 'center' })
      })
    }
  }, [text, expanded])

  if (text === null) return null
  if (!text) return null

  const range = findHighlightRange(text, citation.excerpt)
  if (!range) return null

  const before = text.slice(0, range.start)
  const match = text.slice(range.start, range.end)
  const after = text.slice(range.end)

  return (
    <div style={{ borderBottom: '1px solid rgba(201,168,76,0.1)' }}>
      <button
        onClick={() => setExpanded((e) => !e)}
        className="w-full flex items-center gap-2 px-4 py-2 text-left"
        style={{ background: 'rgba(234,197,80,0.03)' }}
      >
        <svg
          width="8" height="8" viewBox="0 0 8 8" fill="rgba(201,168,76,0.5)"
          style={{ transform: expanded ? 'rotate(90deg)' : 'rotate(0deg)', transition: 'transform 0.15s' }}
        >
          <path d="M2 1l4 3-4 3V1z" />
        </svg>
        <span className="text-[10px] font-semibold uppercase tracking-wider" style={{ color: 'rgba(201,168,76,0.55)' }}>
          Highlighted Text
        </span>
        <span className="text-[9px]" style={{ color: 'rgb(var(--ov) / 0.25)' }}>
          p.{citation.pageNumber}
        </span>
      </button>

      {expanded && (
        <div
          ref={hlRef}
          className="overflow-auto px-4 pb-3"
          style={{ maxHeight: 180 }}
        >
          <p
            className="text-[11px] leading-[1.8] whitespace-pre-wrap"
            style={{ color: 'rgb(var(--ov) / 0.45)' }}
          >
            {before}
            <mark
              style={{
                background: 'rgba(234,197,80,0.35)',
                color: 'rgb(var(--ov) / 0.9)',
                borderRadius: 3,
                padding: '1px 3px',
                boxShadow: '0 0 0 1px rgba(234,197,80,0.5), 0 1px 4px rgba(234,197,80,0.15)',
              }}
            >
              {match}
            </mark>
            {after}
          </p>
        </div>
      )}
    </div>
  )
}

// ── Cited text strip (compact highlight for PDFs) ────────────────────────────
// Since we can't inject highlights into the iframe PDF renderer, this shows
// the cited passage with surrounding context in a small collapsible strip.
function CitedTextStrip({ citation }: { citation: Citation }): JSX.Element | null {
  const [text, setText] = useState<string | null>(null)
  const [expanded, setExpanded] = useState(true)
  const hlRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    ;(window as any).api
      .getPageText(citation.filePath, citation.pageNumber)
      .then((t: string) => setText(t || ''))
      .catch(() => setText(''))
  }, [citation.filePath, citation.pageNumber])

  useEffect(() => {
    if (!text || !expanded || !hlRef.current) return
    const mark = hlRef.current.querySelector('mark')
    if (mark) {
      requestAnimationFrame(() => {
        mark.scrollIntoView({ behavior: 'smooth', block: 'center' })
      })
    }
  }, [text, expanded])

  if (!text) return null

  const range = findHighlightRange(text, citation.excerpt)
  if (!range) return null

  // Show ~120 chars of context around the match
  const contextPad = 120
  const ctxStart = Math.max(0, range.start - contextPad)
  const ctxEnd = Math.min(text.length, range.end + contextPad)
  const before = (ctxStart > 0 ? '…' : '') + text.slice(ctxStart, range.start)
  const match = text.slice(range.start, range.end)
  const after = text.slice(range.end, ctxEnd) + (ctxEnd < text.length ? '…' : '')

  return (
    <div style={{ borderBottom: '1px solid rgba(201,168,76,0.1)' }}>
      <button
        onClick={() => setExpanded((e) => !e)}
        className="w-full flex items-center gap-2 px-4 py-1.5 text-left"
        style={{ background: 'rgba(234,197,80,0.03)' }}
      >
        <svg
          width="7" height="7" viewBox="0 0 8 8" fill="rgba(201,168,76,0.5)"
          style={{ transform: expanded ? 'rotate(90deg)' : 'rotate(0deg)', transition: 'transform 0.15s' }}
        >
          <path d="M2 1l4 3-4 3V1z" />
        </svg>
        <span className="text-[9px] font-semibold uppercase tracking-wider" style={{ color: 'rgba(201,168,76,0.5)' }}>
          Cited passage
        </span>
      </button>

      {expanded && (
        <div
          ref={hlRef}
          className="px-4 pb-2.5"
          style={{ maxHeight: 100, overflow: 'auto' }}
        >
          <p
            className="text-[11px] leading-[1.7]"
            style={{ color: 'rgb(var(--ov) / 0.4)' }}
          >
            {before}
            <mark
              style={{
                background: 'rgba(234,197,80,0.30)',
                color: 'rgb(var(--ov) / 0.85)',
                borderRadius: 2,
                padding: '1px 2px',
              }}
            >
              {match}
            </mark>
            {after}
          </p>
        </div>
      )}
    </div>
  )
}

// ── PDF viewer ───────────────────────────────────────────────────────────────
function PdfViewer({ citation }: { citation: Citation }): JSX.Element {
  const [port, setPort] = useState<number>(0)

  useEffect(() => {
    ;(window as any).api.getFileServerPort().then(setPort).catch(() => setPort(0))
  }, [])

  return (
    <div className="flex-1 flex flex-col" style={{ minHeight: 0 }}>
      {/* Compact cited text highlight */}
      <CitedTextStrip citation={citation} />

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

  const range = findHighlightRange(text, citation.excerpt)

  return (
    <div ref={containerRef} className="flex-1 overflow-auto" style={{ padding: '20px 24px' }}>
      <p
        className="text-[13px] leading-[1.9] whitespace-pre-wrap"
        style={{ color: 'rgb(var(--ov) / 0.6)' }}
      >
        {range ? (
          <>
            {text.slice(0, range.start)}
            <mark
              style={{
                background: 'rgba(234,197,80,0.35)',
                color: 'var(--text)',
                borderRadius: 3,
                padding: '2px 3px',
                boxShadow: '0 0 0 1px rgba(234,197,80,0.5), 0 1px 4px rgba(234,197,80,0.15)',
              }}
            >
              {text.slice(range.start, range.end)}
            </mark>
            {text.slice(range.end)}
          </>
        ) : (
          text
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

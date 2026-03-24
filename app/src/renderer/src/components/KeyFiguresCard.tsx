import { useState } from 'react'
import { KeyFigure } from '../../../../../shared/src/types'

export function extractKeyFigures(text: string): KeyFigure[] {
  const figures: KeyFigure[] = []
  const seen = new Set<string>()

  function addFigure(type: KeyFigure['type'], value: string, idx: number): void {
    if (seen.has(value)) return
    seen.add(value)
    // Extract label from ~60 chars before the match
    const before = text.slice(Math.max(0, idx - 60), idx).trim()
    // Take last sentence fragment or line
    const parts = before.split(/[.;\n]/)
    let label = (parts[parts.length - 1] || '').trim()
    // Clean up leading bullets/numbers
    label = label.replace(/^[-*•\d.)\s]+/, '').trim()
    // Trim to something reasonable
    if (label.length > 40) label = label.slice(label.length - 40).replace(/^\S*\s/, '')
    if (!label) label = type === 'dollar' ? 'Amount' : type === 'percentage' ? 'Rate' : 'Date'
    figures.push({ type, value, label })
  }

  // Dollar amounts
  const dollarRe = /\$[\d,]+(?:\.\d{1,2})?/g
  let m: RegExpExecArray | null
  while ((m = dollarRe.exec(text)) !== null) {
    addFigure('dollar', m[0], m.index)
  }

  // Percentages
  const pctRe = /\d+(?:\.\d+)?%/g
  while ((m = pctRe.exec(text)) !== null) {
    addFigure('percentage', m[0], m.index)
  }

  // Dates: "January 1, 2024" style
  const dateWordsRe = /(?:January|February|March|April|May|June|July|August|September|October|November|December)\s+\d{1,2},?\s+\d{4}/g
  while ((m = dateWordsRe.exec(text)) !== null) {
    addFigure('date', m[0], m.index)
  }

  // Dates: MM/DD/YYYY style
  const dateSlashRe = /\d{1,2}\/\d{1,2}\/\d{2,4}/g
  while ((m = dateSlashRe.exec(text)) !== null) {
    addFigure('date', m[0], m.index)
  }

  return figures.slice(0, 8)
}

function TypeIcon({ type }: { type: KeyFigure['type'] }): JSX.Element {
  if (type === 'dollar') {
    return (
      <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="var(--gold)" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round">
        <line x1="8" y1="1" x2="8" y2="15" />
        <path d="M11.5 4.5C11.5 3.12 10 2 8 2S4.5 3.12 4.5 4.5 6 7 8 7.5s3.5 1.38 3.5 2.75S10 13 8 13s-3.5-1.12-3.5-2.5" />
      </svg>
    )
  }
  if (type === 'percentage') {
    return (
      <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="var(--gold)" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round">
        <line x1="13" y1="3" x2="3" y2="13" />
        <circle cx="4.5" cy="4.5" r="1.5" />
        <circle cx="11.5" cy="11.5" r="1.5" />
      </svg>
    )
  }
  // date / calendar
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="var(--gold)" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round">
      <rect x="2" y="3" width="12" height="11" rx="1.5" />
      <line x1="2" y1="7" x2="14" y2="7" />
      <line x1="5.5" y1="1" x2="5.5" y2="4" />
      <line x1="10.5" y1="1" x2="10.5" y2="4" />
    </svg>
  )
}

export default function KeyFiguresCard({ figures }: { figures: KeyFigure[] }): JSX.Element {
  const [expanded, setExpanded] = useState(false)

  return (
    <div className="mt-2">
      <button
        onClick={() => setExpanded((v) => !v)}
        className="flex items-center gap-1.5 rounded-lg px-2.5 py-1 text-[10.5px] font-medium transition-all"
        style={{
          background: 'rgba(201,168,76,0.08)',
          border: '1px solid rgba(201,168,76,0.2)',
          color: 'var(--gold)',
        }}
      >
        {/* Bar chart icon */}
        <svg width="11" height="11" viewBox="0 0 16 16" fill="none" stroke="var(--gold)" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round">
          <rect x="1" y="8" width="3" height="6" />
          <rect x="6.5" y="4" width="3" height="10" />
          <rect x="12" y="1" width="3" height="13" />
        </svg>
        {figures.length} key figure{figures.length !== 1 ? 's' : ''}
        <svg
          width="8" height="8" viewBox="0 0 10 10" fill="none"
          stroke="var(--gold)" strokeWidth="1.8" strokeLinecap="round"
          style={{ transform: expanded ? 'rotate(180deg)' : 'rotate(0deg)', transition: 'transform 0.18s ease' }}
        >
          <path d="M2 3.5l3 3 3-3" />
        </svg>
      </button>

      {expanded && (
        <div
          className="mt-1.5 rounded-lg px-3 py-2.5"
          style={{
            background: 'rgb(var(--ov) / 0.02)',
            border: '1px solid rgb(var(--ov) / 0.06)',
            display: 'grid',
            gridTemplateColumns: 'repeat(2, 1fr)',
            gap: '8px',
          }}
        >
          {figures.map((fig, idx) => (
            <div
              key={idx}
              className="flex items-center gap-2 rounded-lg px-2.5 py-2"
              style={{
                background: 'rgba(201,168,76,0.04)',
                border: '1px solid rgba(201,168,76,0.1)',
              }}
            >
              <TypeIcon type={fig.type} />
              <div className="min-w-0 flex-1">
                <p className="text-[10px] truncate" style={{ color: 'rgb(var(--ov) / 0.45)' }}>
                  {fig.label}
                </p>
                <p className="text-[12px] font-semibold truncate" style={{ color: 'var(--text)' }}>
                  {fig.value}
                </p>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

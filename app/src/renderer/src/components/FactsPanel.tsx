import { useState, useMemo } from 'react'

// ── Extraction Logic ────────────────────────────────────────────────────────

const MONTH_NAMES = 'January|February|March|April|May|June|July|August|September|October|November|December'

const DATE_PATTERNS = [
  new RegExp(`\\b(?:${MONTH_NAMES})\\s+\\d{1,2},?\\s+\\d{4}\\b`, 'gi'),
  /\b\d{1,2}[\/\-\.]\d{1,2}[\/\-\.]\d{2,4}\b/g,
]

const AMOUNT_PATTERN = /\$[\d,]+\.?\d*/g

const PARTY_PATTERN =
  /(?:between|by and between|party|parties|tenant|landlord|employer|employee|buyer|seller|lessor|lessee|licensor|licensee|borrower|lender)\s+([A-Z][a-zA-Z\s,.]+?)(?:\s*(?:and|,|\(|")|$)/gi

const LEGAL_TERMS = [
  'indemnification', 'termination', 'liability', 'confidentiality',
  'non-compete', 'non-disclosure', 'arbitration', 'jurisdiction',
  'force majeure', 'severability', 'warranty', 'breach',
  'default', 'damages', 'negligence', 'fiduciary',
  'escrow', 'lien', 'injunction', 'statute of limitations',
  'consideration', 'amendment', 'waiver', 'assignment',
  'subpoena', 'deposition', 'interrogatories', 'discovery',
]

export interface ExtractedFacts {
  dates: string[]
  amounts: string[]
  parties: string[]
  terms: string[]
}

export function extractFacts(texts: string[]): ExtractedFacts {
  const combined = texts.join('\n')
  const dates = new Set<string>()
  const amounts = new Set<string>()
  const parties = new Set<string>()
  const terms = new Set<string>()

  for (const pat of DATE_PATTERNS) {
    for (const m of combined.matchAll(pat)) dates.add(m[0])
  }
  for (const m of combined.matchAll(AMOUNT_PATTERN)) amounts.add(m[0])
  for (const m of combined.matchAll(PARTY_PATTERN)) {
    const name = m[1].trim().replace(/[,.]$/, '').trim()
    if (name.length > 2 && name.length < 60) parties.add(name)
  }
  const lower = combined.toLowerCase()
  for (const term of LEGAL_TERMS) {
    if (lower.includes(term)) terms.add(term)
  }

  return {
    dates: [...dates].slice(0, 8),
    amounts: [...amounts].slice(0, 8),
    parties: [...parties].slice(0, 6),
    terms: [...terms].slice(0, 10),
  }
}

// ── Component ───────────────────────────────────────────────────────────────

interface Props {
  chunkTexts: string[]
  onClickFact: (question: string) => void
}

function FactTag({
  label,
  color,
  onClick,
}: {
  label: string
  color: string
  onClick: () => void
}): JSX.Element {
  const [hovered, setHovered] = useState(false)
  return (
    <button
      onClick={onClick}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      className="rounded-lg px-2.5 py-1 text-[10.5px] font-medium transition-all"
      style={{
        background: hovered ? `${color}18` : `${color}0a`,
        border: `1px solid ${hovered ? `${color}40` : `${color}20`}`,
        color: hovered ? color : `${color}bb`,
        cursor: 'pointer',
      }}
    >
      {label}
    </button>
  )
}

export default function FactsPanel({ chunkTexts, onClickFact }: Props): JSX.Element | null {
  const [collapsed, setCollapsed] = useState(false)

  const facts = useMemo(() => extractFacts(chunkTexts), [chunkTexts])

  const hasAny = facts.dates.length + facts.amounts.length + facts.parties.length + facts.terms.length > 0
  if (!hasAny) return null

  return (
    <div
      className="rounded-xl mb-3 overflow-hidden"
      style={{
        background: 'var(--bg-alt)',
        border: '1px solid rgb(var(--ov) / 0.06)',
      }}
    >
      {/* Header */}
      <button
        onClick={() => setCollapsed((v) => !v)}
        className="flex items-center justify-between w-full px-4 py-2.5 text-left"
      >
        <div className="flex items-center gap-2">
          <svg width="10" height="10" viewBox="0 0 16 16" fill="rgba(201,168,76,0.6)">
            <path d="M7.775 3.275a.75.75 0 0 0 1.06 1.06l1.25-1.25a2 2 0 1 1 2.83 2.83l-2.5 2.5a2 2 0 0 1-2.83 0 .75.75 0 0 0-1.06 1.06 3.5 3.5 0 0 0 4.95 0l2.5-2.5a3.5 3.5 0 0 0-4.95-4.95l-1.25 1.25zm-.025 9.45a.75.75 0 0 0-1.06-1.06l-1.25 1.25a2 2 0 0 1-2.83-2.83l2.5-2.5a2 2 0 0 1 2.83 0 .75.75 0 1 0 1.06-1.06 3.5 3.5 0 0 0-4.95 0l-2.5 2.5a3.5 3.5 0 0 0 4.95 4.95l1.25-1.25z" />
          </svg>
          <span className="text-[10px] font-semibold uppercase tracking-[0.12em]" style={{ color: 'rgb(var(--ov) / 0.25)' }}>
            Key Facts
          </span>
          <span
            className="text-[9px] px-1.5 py-0.5 rounded-full font-semibold"
            style={{ background: 'rgba(201,168,76,0.08)', color: 'rgba(201,168,76,0.55)' }}
          >
            {facts.dates.length + facts.amounts.length + facts.parties.length + facts.terms.length}
          </span>
        </div>
        <svg
          width="10"
          height="10"
          viewBox="0 0 16 16"
          fill="rgb(var(--ov) / 0.2)"
          style={{ transform: collapsed ? 'rotate(-90deg)' : 'rotate(0)', transition: 'transform 0.15s ease' }}
        >
          <path d="M12.78 5.22a.75.75 0 0 1 0 1.06l-4.25 4.25a.75.75 0 0 1-1.06 0L3.22 6.28a.75.75 0 0 1 1.06-1.06L8 8.94l3.72-3.72a.75.75 0 0 1 1.06 0z" />
        </svg>
      </button>

      {/* Body */}
      {!collapsed && (
        <div className="px-4 pb-3 flex flex-col gap-2.5" style={{ animation: 'fadeUp 0.2s ease both' }}>
          {facts.dates.length > 0 && (
            <div>
              <p className="text-[9px] font-semibold uppercase tracking-[0.1em] mb-1.5" style={{ color: 'rgb(var(--ov) / 0.18)' }}>Dates</p>
              <div className="flex flex-wrap gap-1.5">
                {facts.dates.map((d) => (
                  <FactTag key={d} label={d} color="#58a6ff" onClick={() => onClickFact(`What is the significance of ${d}?`)} />
                ))}
              </div>
            </div>
          )}
          {facts.amounts.length > 0 && (
            <div>
              <p className="text-[9px] font-semibold uppercase tracking-[0.1em] mb-1.5" style={{ color: 'rgb(var(--ov) / 0.18)' }}>Amounts</p>
              <div className="flex flex-wrap gap-1.5">
                {facts.amounts.map((a) => (
                  <FactTag key={a} label={a} color="#3fb950" onClick={() => onClickFact(`What is the ${a} for?`)} />
                ))}
              </div>
            </div>
          )}
          {facts.parties.length > 0 && (
            <div>
              <p className="text-[9px] font-semibold uppercase tracking-[0.1em] mb-1.5" style={{ color: 'rgb(var(--ov) / 0.18)' }}>Parties</p>
              <div className="flex flex-wrap gap-1.5">
                {facts.parties.map((p) => (
                  <FactTag key={p} label={p} color="#d2a8ff" onClick={() => onClickFact(`What is the role of ${p}?`)} />
                ))}
              </div>
            </div>
          )}
          {facts.terms.length > 0 && (
            <div>
              <p className="text-[9px] font-semibold uppercase tracking-[0.1em] mb-1.5" style={{ color: 'rgb(var(--ov) / 0.18)' }}>Legal Terms</p>
              <div className="flex flex-wrap gap-1.5">
                {facts.terms.map((t) => (
                  <FactTag key={t} label={t} color="#c9a84c" onClick={() => onClickFact(`What does the document say about ${t}?`)} />
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  )
}

import { useState } from 'react'
import { Citation } from '../../../../../shared/src/types'

interface Props {
  citation: Citation
  onView?: (citation: Citation) => void
  isPrimary?: boolean
}

function relevanceLabel(score: number): { label: string; color: string } {
  if (score >= 0.40) return { label: 'Strong', color: '#3fb950' }
  if (score >= 0.22) return { label: 'Good',   color: '#c9a84c' }
  return                    { label: 'Weak',   color: 'rgb(var(--ov) / 0.28)' }
}

export default function SourceCard({ citation, onView, isPrimary = false }: Props): JSX.Element {
  const [hovered, setHovered] = useState(false)
  const [copied, setCopied] = useState(false)
  const ext = citation.fileName.split('.').pop()?.toUpperCase() ?? 'DOC'
  const rel = relevanceLabel(citation.score)

  function handleCopyExcerpt(e: React.MouseEvent): void {
    e.stopPropagation()
    navigator.clipboard.writeText(citation.excerpt).then(() => {
      setCopied(true)
      setTimeout(() => setCopied(false), 1800)
    }).catch(() => {})
  }

  return (
    <div
      className="rounded-xl px-3.5 py-2.5 flex items-start gap-3 cursor-pointer transition-colors"
      style={
        isPrimary
          ? {
              background: 'var(--surface-raised)',
              border: '1px solid rgba(201,168,76,0.22)',
              borderLeft: '3px solid rgba(201,168,76,0.75)',
              boxShadow: '0 2px 8px rgba(201,168,76,0.06)',
            }
          : {
              background: hovered ? 'var(--surface-hover)' : 'var(--surface)',
              border: '1px solid rgb(var(--ov) / 0.07)',
              borderLeft: '2px solid rgba(201,168,76,0.35)',
              opacity: 0.85,
            }
      }
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onClick={() => onView?.(citation)}
    >
      {/* Left: file icon + badge */}
      <div className="shrink-0 mt-0.5 flex flex-col items-start gap-1">
        <span
          className="text-[9px] font-bold px-1.5 py-0.5 rounded"
          style={{ background: 'rgba(201,168,76,0.1)', color: 'rgba(201,168,76,0.7)' }}
        >
          {ext}
        </span>
        {isPrimary && (
          <span
            style={{
              background: 'rgba(201,168,76,0.15)',
              color: 'rgba(201,168,76,0.9)',
              border: '1px solid rgba(201,168,76,0.3)',
              fontSize: 8,
              fontWeight: 700,
              padding: '2px 6px',
              borderRadius: 4,
              letterSpacing: '0.04em',
              textTransform: 'uppercase',
            }}
          >
            Key Source
          </span>
        )}
      </div>

      {/* Right: file info + excerpt */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center justify-between gap-2 mb-1">
          <span
            className="text-[12px] font-medium truncate"
            style={{ color: 'rgb(var(--ov) / 0.75)' }}
            title={citation.fileName}
          >
            {citation.fileName}
          </span>
          <div className="shrink-0 flex items-center gap-1.5">
            <span
              style={{
                display: 'inline-block',
                width: 6,
                height: 6,
                borderRadius: '50%',
                background: rel.color,
                opacity: 0.85,
              }}
            />
            <span className="text-[10px]" style={{ color: rel.color, opacity: 0.85 }}>
              {rel.label}
            </span>
            <span className="text-[11px]" style={{ color: 'rgb(var(--ov) / 0.45)' }}>
              · p.{citation.pageNumber}
            </span>
          </div>
        </div>
        <p
          className="text-[11px] leading-relaxed italic"
          style={{ color: 'rgb(var(--ov) / 0.35)' }}
        >
          "{isPrimary
            ? citation.excerpt
            : citation.excerpt.length > 140
              ? citation.excerpt.slice(0, 140) + '…'
              : citation.excerpt}"
        </p>
        <div className="mt-1.5 flex items-center gap-3">
            {onView && (
              <p className="text-[10px] font-semibold" style={{ color: hovered ? 'rgba(201,168,76,0.75)' : 'rgba(201,168,76,0.35)' }}>
                View in document →
              </p>
            )}
            <button
              onClick={handleCopyExcerpt}
              aria-label="Copy excerpt"
              className="flex items-center gap-1 text-[9px] font-semibold px-1.5 py-0.5 rounded transition-all"
              style={{
                background: copied ? 'rgba(63,185,80,0.1)' : 'rgb(var(--ov) / 0.05)',
                border: `1px solid ${copied ? 'rgba(63,185,80,0.3)' : 'rgb(var(--ov) / 0.1)'}`,
                color: copied ? '#3fb950' : 'rgb(var(--ov) / 0.35)',
              }}
            >
              {copied ? (
                <>
                  <svg width="8" height="8" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M2 5l2 2 4-4" />
                  </svg>
                  Copied
                </>
              ) : (
                <>
                  <svg width="8" height="8" viewBox="0 0 16 16" fill="currentColor">
                    <path d="M0 6.75C0 5.784.784 5 1.75 5h1.5a.75.75 0 0 1 0 1.5h-1.5a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-1.5a.75.75 0 0 1 1.5 0v1.5A1.75 1.75 0 0 1 9.25 16h-7.5A1.75 1.75 0 0 1 0 14.25ZM5 1.75C5 .784 5.784 0 6.75 0h7.5C15.216 0 16 .784 16 1.75v7.5A1.75 1.75 0 0 1 14.25 11h-7.5A1.75 1.75 0 0 1 5 9.25Zm1.75-.25a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-7.5a.25.25 0 0 0-.25-.25Z" />
                  </svg>
                  Copy
                </>
              )}
            </button>
          </div>
      </div>
    </div>
  )
}

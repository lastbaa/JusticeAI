import { useState } from 'react'
import { Citation } from '../../../../../shared/src/types'

interface Props {
  citation: Citation
  onView?: (citation: Citation) => void
}

export default function SourceCard({ citation, onView }: Props): JSX.Element {
  const score = Math.round(citation.score * 100)
  const [hovered, setHovered] = useState(false)

  return (
    <div
      className="rounded-xl px-4 py-3 flex flex-col gap-2.5 transition-colors cursor-pointer"
      style={{
        background: hovered ? '#111' : '#0c0c0c',
        border: '1px solid rgba(255,255,255,0.07)',
        borderLeft: '2px solid rgba(201,168,76,0.4)',
        transition: 'background 0.15s ease',
      }}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onClick={() => onView?.(citation)}
    >
      {/* Header row */}
      <div className="flex items-center justify-between gap-3">
        <div className="flex items-center gap-2 min-w-0">
          <svg width="11" height="11" viewBox="0 0 16 16" fill="#c9a84c" className="shrink-0">
            <path d="M2 1.75C2 .784 2.784 0 3.75 0h6.586c.464 0 .909.184 1.237.513l2.914 2.914c.329.328.513.773.513 1.237v9.586A1.75 1.75 0 0 1 13.25 16h-9.5A1.75 1.75 0 0 1 2 14.25z" />
          </svg>
          <span
            className="text-[12px] font-medium truncate"
            style={{ color: 'rgba(255,255,255,0.75)' }}
            title={citation.fileName}
          >
            {citation.fileName}
          </span>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <span className="text-[11px]" style={{ color: 'rgba(255,255,255,0.28)' }}>
            p.&nbsp;
            <span className="font-medium" style={{ color: 'rgba(255,255,255,0.5)' }}>
              {citation.pageNumber}
            </span>
          </span>
          <span
            className="text-[10px] px-2 py-0.5 rounded-full font-semibold"
            style={{
              background: 'rgba(201,168,76,0.07)',
              color: 'rgba(201,168,76,0.65)',
              border: '1px solid rgba(201,168,76,0.15)',
            }}
          >
            {score}%
          </span>
          {hovered && onView && (
            <span
              className="text-[10px] font-semibold px-2 py-0.5 rounded-md"
              style={{
                background: 'rgba(201,168,76,0.1)',
                border: '1px solid rgba(201,168,76,0.22)',
                color: '#c9a84c',
              }}
            >
              View →
            </span>
          )}
        </div>
      </div>

      {/* Excerpt */}
      <blockquote
        className="text-[11.5px] leading-relaxed italic pl-3"
        style={{
          color: 'rgba(255,255,255,0.38)',
          borderLeft: '1.5px solid rgba(201,168,76,0.2)',
        }}
      >
        "{citation.excerpt.length > 160
          ? citation.excerpt.slice(0, 160) + '…'
          : citation.excerpt}"
      </blockquote>
    </div>
  )
}

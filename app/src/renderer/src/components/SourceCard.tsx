import { useState } from 'react'
import { Citation } from '../../../../../shared/src/types'

interface Props {
  citation: Citation
  onView?: (citation: Citation) => void
}

export default function SourceCard({ citation, onView }: Props): JSX.Element {
  const [hovered, setHovered] = useState(false)
  const ext = citation.fileName.split('.').pop()?.toUpperCase() ?? 'DOC'

  return (
    <div
      className="rounded-xl px-3.5 py-2.5 flex items-start gap-3 cursor-pointer transition-colors"
      style={{
        background: hovered ? '#111' : '#0c0c0c',
        border: '1px solid rgba(255,255,255,0.07)',
        borderLeft: '2px solid rgba(201,168,76,0.35)',
      }}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onClick={() => onView?.(citation)}
    >
      {/* Left: file icon + badge */}
      <div className="shrink-0 mt-0.5">
        <span
          className="text-[9px] font-bold px-1.5 py-0.5 rounded"
          style={{ background: 'rgba(201,168,76,0.1)', color: 'rgba(201,168,76,0.7)' }}
        >
          {ext}
        </span>
      </div>

      {/* Right: file info + excerpt */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center justify-between gap-2 mb-1">
          <span
            className="text-[12px] font-medium truncate"
            style={{ color: 'rgba(255,255,255,0.75)' }}
            title={citation.fileName}
          >
            {citation.fileName}
          </span>
          <span className="shrink-0 text-[11px]" style={{ color: 'rgba(255,255,255,0.28)' }}>
            p.{citation.pageNumber}
          </span>
        </div>
        <p
          className="text-[11px] leading-relaxed italic"
          style={{ color: 'rgba(255,255,255,0.35)' }}
        >
          "{citation.excerpt.length > 140
            ? citation.excerpt.slice(0, 140) + '…'
            : citation.excerpt}"
        </p>
        {hovered && onView && (
          <p
            className="mt-1.5 text-[10px] font-semibold"
            style={{ color: 'rgba(201,168,76,0.6)' }}
          >
            Click to view in document →
          </p>
        )}
      </div>
    </div>
  )
}

import { useState } from 'react'
import { Citation, FileInfo } from '../../../../../shared/src/types'

interface Props {
  files: FileInfo[]
  citations: Citation[]
  isQuerying: boolean
  isLoading: boolean
  collapsed: boolean
  onAddFiles: () => void
  onRemoveFile: (id: string) => void
  onClearFiles: () => void
  onViewCitation: (citation: Citation) => void
  onExportCitations?: () => void
}

function CitationRow({
  citation,
  index,
  onView,
}: {
  citation: Citation
  index: number
  onView: (c: Citation) => void
}): JSX.Element {
  const [expanded, setExpanded] = useState(false)
  const [hovered, setHovered] = useState(false)
  const short = citation.excerpt.slice(0, 120)
  const hasMore = citation.excerpt.length > 120

  return (
    <div
      className="rounded-xl px-4 py-3 flex flex-col gap-2 transition-all cursor-pointer"
      style={{
        background: hovered ? '#111' : '#0d0d0d',
        border: '1px solid rgba(201,168,76,0.14)',
        borderLeft: '2px solid rgba(201,168,76,0.45)',
        transition: 'background 0.15s ease',
      }}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      {/* Row header */}
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-1.5 min-w-0">
          <span
            className="shrink-0 text-[9px] font-bold w-4 h-4 rounded flex items-center justify-center"
            style={{ background: 'rgba(201,168,76,0.12)', color: '#c9a84c' }}
          >
            {index + 1}
          </span>
          <span
            className="text-[11px] font-medium truncate"
            style={{ color: 'rgba(255,255,255,0.7)' }}
            title={citation.fileName}
          >
            {citation.fileName}
          </span>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <span className="text-[10px]" style={{ color: 'rgba(255,255,255,0.3)' }}>
            p.{citation.pageNumber}
          </span>
          {hovered && (
            <button
              onClick={(e) => { e.stopPropagation(); onView(citation) }}
              className="flex items-center gap-1 text-[9px] font-semibold px-2 py-0.5 rounded-md transition-all"
              style={{
                background: 'rgba(201,168,76,0.12)',
                border: '1px solid rgba(201,168,76,0.25)',
                color: '#c9a84c',
              }}
            >
              <svg width="8" height="8" viewBox="0 0 16 16" fill="currentColor">
                <path d="M2 1.75C2 .784 2.784 0 3.75 0h6.586c.464 0 .909.184 1.237.513l2.914 2.914c.329.328.513.773.513 1.237v9.586A1.75 1.75 0 0 1 13.25 16h-9.5A1.75 1.75 0 0 1 2 14.25z" />
              </svg>
              View
            </button>
          )}
        </div>
      </div>

      {/* Excerpt */}
      <p
        className="text-[11px] leading-relaxed italic"
        style={{ color: 'rgba(255,255,255,0.35)' }}
      >
        "{expanded ? citation.excerpt : short}
        {!expanded && hasMore && '…'}"
        {hasMore && (
          <button
            onClick={(e) => { e.stopPropagation(); setExpanded((v) => !v) }}
            className="ml-1.5 text-[10px] not-italic"
            style={{ color: 'rgba(201,168,76,0.55)' }}
          >
            {expanded ? 'less' : 'more'}
          </button>
        )}
      </p>
    </div>
  )
}

function FileRow({
  file,
  onRemove,
}: {
  file: FileInfo
  onRemove: () => void
}): JSX.Element {
  const [hovered, setHovered] = useState(false)
  const ext = file.fileName.split('.').pop()?.toUpperCase() ?? 'DOC'

  return (
    <div
      className="flex items-center gap-2.5 px-2 py-2.5 rounded-lg transition-colors"
      style={{ background: hovered ? 'rgba(255,255,255,0.03)' : 'transparent' }}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      <span
        className="shrink-0 text-[9px] font-bold px-1.5 py-0.5 rounded"
        style={{ background: 'rgba(201,168,76,0.08)', color: 'rgba(201,168,76,0.65)' }}
      >
        {ext}
      </span>
      <div className="flex-1 min-w-0">
        <p
          className="text-[11.5px] truncate leading-snug"
          style={{ color: 'rgba(255,255,255,0.6)' }}
          title={file.fileName}
        >
          {file.fileName}
        </p>
        <p className="text-[10px] mt-0.5" style={{ color: 'rgba(255,255,255,0.22)' }}>
          {file.totalPages} {file.totalPages === 1 ? 'page' : 'pages'}
        </p>
      </div>
      {hovered && (
        <button
          onClick={onRemove}
          className="shrink-0 h-5 w-5 flex items-center justify-center rounded text-[#383838] hover:text-[#f85149] transition-colors"
        >
          <svg width="9" height="9" viewBox="0 0 12 12" fill="currentColor">
            <path d="M1.22 1.22a.75.75 0 0 1 1.06 0L6 4.94l3.72-3.72a.75.75 0 1 1 1.06 1.06L7.06 6l3.72 3.72a.75.75 0 1 1-1.06 1.06L6 7.06l-3.72 3.72a.75.75 0 0 1-1.06-1.06L4.94 6 1.22 2.28a.75.75 0 0 1 0-1.06z" />
          </svg>
        </button>
      )}
    </div>
  )
}

export default function ContextPanel({
  files,
  citations,
  isQuerying,
  isLoading,
  collapsed,
  onAddFiles,
  onRemoveFile,
  onClearFiles,
  onViewCitation,
  onExportCitations,
}: Props): JSX.Element {
  const hasCitations = citations.length > 0

  return (
    <aside
      className="flex h-screen shrink-0 flex-col"
      style={{
        width: collapsed ? 0 : 300,
        minWidth: collapsed ? 0 : 300,
        borderLeft: collapsed ? 'none' : '1px solid rgba(255,255,255,0.05)',
        background: '#050505',
        overflow: 'hidden',
        transition: 'width 0.25s ease, min-width 0.25s ease',
      }}
    >
      {/* Header — matches other panels' h-11 drag region */}
      <div
        className="drag-region flex h-11 shrink-0 items-center justify-between px-4"
        style={{ borderBottom: '1px solid rgba(255,255,255,0.05)' }}
      >
        <div className="no-drag flex items-center gap-2">
          <svg width="11" height="11" viewBox="0 0 16 16" fill="rgba(201,168,76,0.55)">
            <path d="M2 1.75C2 .784 2.784 0 3.75 0h6.586c.464 0 .909.184 1.237.513l2.914 2.914c.329.328.513.773.513 1.237v9.586A1.75 1.75 0 0 1 13.25 16h-9.5A1.75 1.75 0 0 1 2 14.25z" />
          </svg>
          <span className="text-[12px] font-semibold text-white tracking-[-0.01em]">
            Documents
          </span>
          {files.length > 0 && (
            <span
              className="text-[10px] px-1.5 py-0.5 rounded-full font-semibold"
              style={{ background: 'rgba(255,255,255,0.06)', color: 'rgba(255,255,255,0.3)' }}
            >
              {files.length}
            </span>
          )}
        </div>
        <button
          onClick={onAddFiles}
          disabled={isLoading}
          title="Add documents"
          className="no-drag flex h-6 w-6 items-center justify-center rounded-md text-[#444] hover:bg-[#1a1a1a] hover:text-[#aaa] transition-all disabled:opacity-40"
        >
          <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor">
            <path d="M7.75 2a.75.75 0 0 1 .75.75V7h4.25a.75.75 0 0 1 0 1.5H8.5v4.25a.75.75 0 0 1-1.5 0V8.5H2.75a.75.75 0 0 1 0-1.5H7V2.75A.75.75 0 0 1 7.75 2z" />
          </svg>
        </button>
      </div>

      <div className="flex-1 overflow-y-auto">

        {/* ── Retrieved chunks section ── */}
        {(hasCitations || isQuerying) && (
          <div className="px-4 pt-4 pb-2">
            <div className="flex items-center justify-between gap-2 mb-3">
              <div className="flex items-center gap-2">
                <p className="text-[10px] font-semibold uppercase tracking-[0.14em]" style={{ color: 'rgba(255,255,255,0.2)' }}>
                  Retrieved Context
                </p>
                {isQuerying && (
                  <div
                    className="h-3 w-3 rounded-full animate-spin shrink-0"
                    style={{ border: '1.5px solid rgba(201,168,76,0.2)', borderTopColor: '#c9a84c' }}
                  />
                )}
                {hasCitations && !isQuerying && (
                  <span
                    className="text-[10px] px-1.5 py-0.5 rounded-full font-semibold"
                    style={{ background: 'rgba(201,168,76,0.08)', color: 'rgba(201,168,76,0.6)' }}
                  >
                    {citations.length}
                  </span>
                )}
              </div>
              {hasCitations && !isQuerying && onExportCitations && (
                <button
                  onClick={onExportCitations}
                  title="Export citations as CSV"
                  className="flex items-center gap-1 text-[9px] font-semibold px-2 py-0.5 rounded-md transition-all"
                  style={{
                    background: 'rgba(255,255,255,0.04)',
                    border: '1px solid rgba(255,255,255,0.08)',
                    color: 'rgba(255,255,255,0.3)',
                  }}
                  onMouseEnter={(e) => {
                    const el = e.currentTarget as HTMLButtonElement
                    el.style.color = 'rgba(201,168,76,0.8)'
                    el.style.borderColor = 'rgba(201,168,76,0.25)'
                  }}
                  onMouseLeave={(e) => {
                    const el = e.currentTarget as HTMLButtonElement
                    el.style.color = 'rgba(255,255,255,0.3)'
                    el.style.borderColor = 'rgba(255,255,255,0.08)'
                  }}
                >
                  <svg width="8" height="8" viewBox="0 0 16 16" fill="currentColor">
                    <path d="M2.75 14A1.75 1.75 0 0 1 1 12.25v-2.5a.75.75 0 0 1 1.5 0v2.5c0 .138.112.25.25.25h10.5a.25.25 0 0 0 .25-.25v-2.5a.75.75 0 0 1 1.5 0v2.5A1.75 1.75 0 0 1 13.25 14ZM7.25 7.689V2a.75.75 0 0 1 1.5 0v5.689l1.97-1.97a.749.749 0 1 1 1.06 1.06l-3.25 3.25a.749.749 0 0 1-1.06 0L4.22 6.779a.749.749 0 1 1 1.06-1.06l1.97 1.97Z" />
                  </svg>
                  Export CSV
                </button>
              )}
            </div>

            {isQuerying && !hasCitations ? (
              <div className="flex flex-col gap-2">
                {[80, 65, 72].map((w, i) => (
                  <div
                    key={i}
                    className="h-16 rounded-xl"
                    style={{
                      background: '#0d0d0d',
                      border: '1px solid rgba(255,255,255,0.05)',
                      width: `${w}%`,
                      animation: `blink 1.4s ease ${i * 0.2}s infinite`,
                    }}
                  />
                ))}
              </div>
            ) : (
              <div className="flex flex-col gap-2">
                {citations.map((c, i) => (
                  <CitationRow key={i} citation={c} index={i} onView={onViewCitation} />
                ))}
              </div>
            )}

            {/* Divider */}
            {files.length > 0 && (
              <div
                className="mt-4 mb-0 h-px"
                style={{ background: 'rgba(255,255,255,0.05)' }}
              />
            )}
          </div>
        )}

        {/* ── Loaded documents section ── */}
        {files.length > 0 ? (
          <div className="px-4 pt-4 pb-4">
            <div className="mb-2 flex items-center justify-between">
              <p className="text-[10px] font-semibold uppercase tracking-[0.14em]" style={{ color: 'rgba(255,255,255,0.2)' }}>
                Given Documents
              </p>
              <button
                onClick={onClearFiles}
                disabled={isLoading}
                title="Clear all documents"
                className="flex items-center gap-1 text-[9px] font-semibold px-2 py-0.5 rounded-md transition-all disabled:opacity-40"
                style={{
                  background: 'rgba(255,255,255,0.04)',
                  border: '1px solid rgba(255,255,255,0.08)',
                  color: 'rgba(255,255,255,0.3)',
                }}
                onMouseEnter={(e) => {
                  const el = e.currentTarget as HTMLButtonElement
                  el.style.color = '#f85149'
                  el.style.borderColor = 'rgba(248,81,73,0.3)'
                }}
                onMouseLeave={(e) => {
                  const el = e.currentTarget as HTMLButtonElement
                  el.style.color = 'rgba(255,255,255,0.3)'
                  el.style.borderColor = 'rgba(255,255,255,0.08)'
                }}
              >
                <svg width="9" height="9" viewBox="0 0 16 16" fill="currentColor">
                  <path d="M11 1.75V3h2.25a.75.75 0 0 1 0 1.5H2.75a.75.75 0 0 1 0-1.5H5V1.75C5 .784 5.784 0 6.75 0h2.5C10.216 0 11 .784 11 1.75zM4.496 6.675l.66 6.6a.25.25 0 0 0 .249.225h5.19a.25.25 0 0 0 .249-.225l.66-6.6a.75.75 0 0 1 1.492.149l-.66 6.6A1.748 1.748 0 0 1 10.595 15h-5.19a1.75 1.75 0 0 1-1.741-1.575l-.66-6.6a.75.75 0 1 1 1.492-.15z" />
                </svg>
                Clear
              </button>
            </div>
            <div className="flex flex-col gap-0.5">
              {files.map((file) => (
                <FileRow
                  key={file.id}
                  file={file}
                  onRemove={() => onRemoveFile(file.id)}
                />
              ))}
            </div>
          </div>
        ) : (
          /* Empty state */
          <div className="flex flex-col items-center py-16 px-4 text-center">
            <div
              className="mb-3 flex h-9 w-9 items-center justify-center rounded-xl"
              style={{ background: 'rgba(255,255,255,0.03)', border: '1px solid rgba(255,255,255,0.06)' }}
            >
              <svg width="15" height="15" viewBox="0 0 16 16" fill="none">
                <path
                  d="M2 1.75C2 .784 2.784 0 3.75 0h6.586c.464 0 .909.184 1.237.513l2.914 2.914c.329.328.513.773.513 1.237v9.586A1.75 1.75 0 0 1 13.25 16h-9.5A1.75 1.75 0 0 1 2 14.25z"
                  stroke="rgba(255,255,255,0.12)"
                  strokeWidth="1.2"
                  fill="none"
                />
              </svg>
            </div>
            <p className="text-[11px]" style={{ color: 'rgba(255,255,255,0.25)' }}>No documents loaded</p>
            <p className="mt-0.5 text-[10px]" style={{ color: 'rgba(255,255,255,0.14)' }}>Add files to begin</p>
          </div>
        )}
      </div>

      {/* Bottom add button */}
      <div className="shrink-0 px-4 py-3" style={{ borderTop: '1px solid rgba(255,255,255,0.05)' }}>
        <button
          onClick={onAddFiles}
          disabled={isLoading}
          className="flex w-full items-center gap-2.5 rounded-lg px-4 py-2.5 text-[11.5px] font-medium transition-all disabled:opacity-40"
          style={{ background: 'rgba(201,168,76,0.07)', border: '1px solid rgba(201,168,76,0.14)', color: 'rgba(201,168,76,0.7)' }}
          onMouseEnter={(e) => {
            const el = e.currentTarget as HTMLButtonElement
            el.style.background = 'rgba(201,168,76,0.12)'
          }}
          onMouseLeave={(e) => {
            const el = e.currentTarget as HTMLButtonElement
            el.style.background = 'rgba(201,168,76,0.07)'
          }}
        >
          <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
            <path d="M2 1.75C2 .784 2.784 0 3.75 0h6.586c.464 0 .909.184 1.237.513l2.914 2.914c.329.328.513.773.513 1.237v9.586A1.75 1.75 0 0 1 13.25 16h-9.5A1.75 1.75 0 0 1 2 14.25V1.75zM8.75 9.25a.75.75 0 0 0-1.5 0v1.5H5.75a.75.75 0 0 0 0 1.5h1.5v1.5a.75.75 0 0 0 1.5 0v-1.5h1.5a.75.75 0 0 0 0-1.5H8.75v-1.5z" />
          </svg>
          {isLoading ? 'Processing…' : 'Add Documents…'}
        </button>
      </div>
    </aside>
  )
}

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
  activeCitation?: Citation | null
  onExportCitations?: () => void
  caseName?: string
}

function CitationRow({
  citation,
  index,
  isActive,
  onView,
}: {
  citation: Citation
  index: number
  isActive: boolean
  onView: (c: Citation) => void
}): JSX.Element {
  const [expanded, setExpanded] = useState(false)
  const [hovered, setHovered] = useState(false)
  const short = citation.excerpt.slice(0, 120)
  const hasMore = citation.excerpt.length > 120

  return (
    <div
      className="rounded-xl px-4 py-3 flex flex-col gap-2 cursor-pointer"
      style={{
        background: isActive ? 'rgba(201,168,76,0.07)' : hovered ? 'var(--surface-hover)' : 'var(--bg-alt)',
        border: `1px solid ${isActive ? 'rgba(201,168,76,0.3)' : hovered ? 'rgba(201,168,76,0.2)' : 'rgba(201,168,76,0.1)'}`,
        borderLeft: `2.5px solid ${isActive ? 'rgba(201,168,76,0.8)' : 'rgba(201,168,76,0.45)'}`,
        boxShadow: isActive ? '0 2px 8px rgba(201,168,76,0.06)' : 'none',
        transition: 'all 0.18s ease',
      }}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onClick={() => onView(citation)}
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
            style={{ color: 'rgb(var(--ov) / 0.7)' }}
            title={citation.fileName}
          >
            {citation.fileName}
          </span>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          {/* Confidence score */}
          <span
            className="text-[9px] font-semibold px-1.5 py-0.5 rounded-md"
            style={{
              background: citation.score > 0.8
                ? 'rgba(63,185,80,0.1)' : citation.score >= 0.5
                ? 'rgba(210,168,50,0.1)' : 'rgba(248,81,73,0.1)',
              color: citation.score > 0.8
                ? 'rgba(63,185,80,0.8)' : citation.score >= 0.5
                ? 'rgba(210,168,50,0.8)' : 'rgba(248,81,73,0.8)',
              border: `1px solid ${citation.score > 0.8
                ? 'rgba(63,185,80,0.2)' : citation.score >= 0.5
                ? 'rgba(210,168,50,0.2)' : 'rgba(248,81,73,0.2)'}`,
            }}
            title={`Relevance: ${Math.round(citation.score * 100)}%`}
          >
            {Math.round(citation.score * 100)}%
          </span>
          <span className="text-[10px]" style={{ color: 'rgb(var(--ov) / 0.3)' }}>
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
        style={{ color: 'rgb(var(--ov) / 0.35)' }}
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
      style={{ background: hovered ? 'rgb(var(--ov) / 0.03)' : 'transparent' }}
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
          style={{ color: 'rgb(var(--ov) / 0.6)' }}
          title={file.fileName}
        >
          {file.fileName}
        </p>
        <p className="text-[10px] mt-0.5" style={{ color: 'rgb(var(--ov) / 0.22)' }}>
          {file.totalPages} {file.totalPages === 1 ? 'page' : 'pages'}
        </p>
      </div>
      {hovered && (
        <button
          onClick={onRemove}
          aria-label={`Remove ${file.fileName}`}
          className="shrink-0 h-5 w-5 flex items-center justify-center rounded transition-colors"
          style={{ color: 'rgb(var(--ov) / 0.2)' }}
          onMouseEnter={(e) => { (e.currentTarget as HTMLButtonElement).style.color = '#f85149' }}
          onMouseLeave={(e) => { (e.currentTarget as HTMLButtonElement).style.color = 'rgb(var(--ov) / 0.2)' }}
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
  activeCitation,
  onAddFiles,
  onRemoveFile,
  onClearFiles,
  onViewCitation,
  onExportCitations,
  caseName,
}: Props): JSX.Element {
  const hasCitations = citations.length > 0
  const showSources = hasCitations || isQuerying

  return (
    <aside
      className="flex h-screen shrink-0 flex-col"
      style={{
        width: collapsed ? 0 : 300,
        minWidth: collapsed ? 0 : 300,
        borderLeft: collapsed ? 'none' : '1px solid rgb(var(--ov) / 0.05)',
        background: 'var(--panel)',
        overflow: 'hidden',
        transition: 'width 0.25s ease, min-width 0.25s ease',
      }}
    >
      {/* Header — matches other panels' h-11 drag region */}
      <div
        className="drag-region flex h-11 shrink-0 items-center justify-between px-4"
        style={{ borderBottom: '1px solid rgb(var(--ov) / 0.05)' }}
      >
        <div className="no-drag flex items-center gap-2">
          <svg width="11" height="11" viewBox="0 0 16 16" fill="rgba(201,168,76,0.55)">
            <path d="M2 1.75C2 .784 2.784 0 3.75 0h6.586c.464 0 .909.184 1.237.513l2.914 2.914c.329.328.513.773.513 1.237v9.586A1.75 1.75 0 0 1 13.25 16h-9.5A1.75 1.75 0 0 1 2 14.25z" />
          </svg>
          <span className="text-[12px] font-semibold tracking-[-0.01em] truncate" style={{ color: 'var(--text)' }} title={caseName ? `${caseName} — Documents` : 'Documents'}>
            {caseName ? `${caseName} — Documents` : 'Documents'}
          </span>
          {files.length > 0 && (
            <span
              className="text-[10px] px-1.5 py-0.5 rounded-full font-semibold"
              style={{ background: 'rgb(var(--ov) / 0.06)', color: 'rgb(var(--ov) / 0.3)' }}
            >
              {files.length}
            </span>
          )}
        </div>
        <button
          onClick={onAddFiles}
          disabled={isLoading}
          title="Add documents"
          aria-label="Add documents"
          className="no-drag flex h-6 w-6 items-center justify-center rounded-md transition-all disabled:opacity-40"
          style={{ color: 'rgb(var(--ov) / 0.3)' }}
          onMouseEnter={(e) => {
            (e.currentTarget as HTMLButtonElement).style.color = 'rgb(var(--ov) / 0.6)'
            ;(e.currentTarget as HTMLButtonElement).style.background = 'rgb(var(--ov) / 0.05)'
          }}
          onMouseLeave={(e) => {
            (e.currentTarget as HTMLButtonElement).style.color = 'rgb(var(--ov) / 0.3)'
            ;(e.currentTarget as HTMLButtonElement).style.background = 'transparent'
          }}
        >
          <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor">
            <path d="M7.75 2a.75.75 0 0 1 .75.75V7h4.25a.75.75 0 0 1 0 1.5H8.5v4.25a.75.75 0 0 1-1.5 0V8.5H2.75a.75.75 0 0 1 0-1.5H7V2.75A.75.75 0 0 1 7.75 2z" />
          </svg>
        </button>
      </div>

      {/* ── Two-pane body: sources (top) + documents (bottom) ── */}
      <div className="flex-1 flex flex-col min-h-0">

        {/* ── TOP: Answer Sources (own scroll) ── */}
        {showSources && (
          <div
            className="flex flex-col min-h-0 shrink-0"
            style={{
              maxHeight: files.length > 0 ? '55%' : '100%',
              borderBottom: files.length > 0 ? '1px solid rgb(var(--ov) / 0.05)' : 'none',
            }}
          >
            <div className="shrink-0 flex items-center justify-between gap-2 px-4 pt-4 pb-2">
              <div className="flex items-center gap-2">
                <p className="text-[10px] font-semibold uppercase tracking-[0.14em]" style={{ color: 'rgb(var(--ov) / 0.2)' }}>
                  Answer Sources
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
                  aria-label="Export citations as CSV"
                  className="flex items-center gap-1 text-[9px] font-semibold px-2 py-0.5 rounded-md transition-all"
                  style={{
                    background: 'rgb(var(--ov) / 0.04)',
                    border: '1px solid rgb(var(--ov) / 0.08)',
                    color: 'rgb(var(--ov) / 0.3)',
                  }}
                  onMouseEnter={(e) => {
                    const el = e.currentTarget as HTMLButtonElement
                    el.style.color = 'rgba(201,168,76,0.8)'
                    el.style.borderColor = 'rgba(201,168,76,0.25)'
                  }}
                  onMouseLeave={(e) => {
                    const el = e.currentTarget as HTMLButtonElement
                    el.style.color = 'rgb(var(--ov) / 0.3)'
                    el.style.borderColor = 'rgb(var(--ov) / 0.08)'
                  }}
                >
                  <svg width="8" height="8" viewBox="0 0 16 16" fill="currentColor">
                    <path d="M2.75 14A1.75 1.75 0 0 1 1 12.25v-2.5a.75.75 0 0 1 1.5 0v2.5c0 .138.112.25.25.25h10.5a.25.25 0 0 0 .25-.25v-2.5a.75.75 0 0 1 1.5 0v2.5A1.75 1.75 0 0 1 13.25 14ZM7.25 7.689V2a.75.75 0 0 1 1.5 0v5.689l1.97-1.97a.749.749 0 1 1 1.06 1.06l-3.25 3.25a.749.749 0 0 1-1.06 0L4.22 6.779a.749.749 0 1 1 1.06-1.06l1.97 1.97Z" />
                  </svg>
                  Export CSV
                </button>
              )}
            </div>

            <div className="overflow-y-auto px-4 pb-3 flex-1 min-h-0">
              {isQuerying && !hasCitations ? (
                <div className="flex flex-col gap-2">
                  {[80, 65, 72].map((w, i) => (
                    <div
                      key={i}
                      className="h-16 rounded-xl"
                      style={{
                        background: 'var(--bg-alt)',
                        border: '1px solid rgb(var(--ov) / 0.05)',
                        width: `${w}%`,
                        animation: `blink 1.4s ease ${i * 0.2}s infinite`,
                      }}
                    />
                  ))}
                </div>
              ) : (
                <div className="flex flex-col gap-2">
                  {citations.map((c, i) => (
                    <CitationRow
                      key={i}
                      citation={c}
                      index={i}
                      isActive={activeCitation?.filePath === c.filePath && activeCitation?.pageNumber === c.pageNumber && activeCitation?.excerpt === c.excerpt}
                      onView={onViewCitation}
                    />
                  ))}
                </div>
              )}
            </div>
          </div>
        )}

        {/* ── BOTTOM: Your Documents (own scroll) ── */}
        <div className="flex-1 flex flex-col min-h-0">
          {files.length > 0 ? (
            <>
              <div className="shrink-0 px-4 pt-4 pb-2 flex items-center justify-between">
                <p className="text-[10px] font-semibold uppercase tracking-[0.14em]" style={{ color: 'rgb(var(--ov) / 0.2)' }}>
                  Your Documents
                </p>
                <button
                  onClick={onClearFiles}
                  disabled={isLoading}
                  title="Clear all documents"
                  aria-label="Clear all documents"
                  className="flex items-center gap-1 text-[9px] font-semibold px-2 py-0.5 rounded-md transition-all disabled:opacity-40"
                  style={{
                    background: 'rgb(var(--ov) / 0.04)',
                    border: '1px solid rgb(var(--ov) / 0.08)',
                    color: 'rgb(var(--ov) / 0.3)',
                  }}
                  onMouseEnter={(e) => {
                    const el = e.currentTarget as HTMLButtonElement
                    el.style.color = '#f85149'
                    el.style.borderColor = 'rgba(248,81,73,0.3)'
                  }}
                  onMouseLeave={(e) => {
                    const el = e.currentTarget as HTMLButtonElement
                    el.style.color = 'rgb(var(--ov) / 0.3)'
                    el.style.borderColor = 'rgb(var(--ov) / 0.08)'
                  }}
                >
                  <svg width="9" height="9" viewBox="0 0 16 16" fill="currentColor">
                    <path d="M11 1.75V3h2.25a.75.75 0 0 1 0 1.5H2.75a.75.75 0 0 1 0-1.5H5V1.75C5 .784 5.784 0 6.75 0h2.5C10.216 0 11 .784 11 1.75zM4.496 6.675l.66 6.6a.25.25 0 0 0 .249.225h5.19a.25.25 0 0 0 .249-.225l.66-6.6a.75.75 0 0 1 1.492.149l-.66 6.6A1.748 1.748 0 0 1 10.595 15h-5.19a1.75 1.75 0 0 1-1.741-1.575l-.66-6.6a.75.75 0 1 1 1.492-.15z" />
                  </svg>
                  Clear
                </button>
              </div>
              <div className="overflow-y-auto px-4 pb-3 flex-1 min-h-0">
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
            </>
          ) : (
            /* Empty state */
            <div className="flex flex-col items-center py-16 px-6 text-center">
              <div
                className="mb-4 flex h-11 w-11 items-center justify-center rounded-2xl"
                style={{ background: 'rgba(201,168,76,0.04)', border: '1px solid rgba(201,168,76,0.1)' }}
              >
                <svg width="18" height="18" viewBox="0 0 16 16" fill="none">
                  <path
                    d="M2 1.75C2 .784 2.784 0 3.75 0h6.586c.464 0 .909.184 1.237.513l2.914 2.914c.329.328.513.773.513 1.237v9.586A1.75 1.75 0 0 1 13.25 16h-9.5A1.75 1.75 0 0 1 2 14.25z"
                    stroke="rgba(201,168,76,0.3)"
                    strokeWidth="1.2"
                    fill="none"
                  />
                </svg>
              </div>
              <p className="text-[12px] font-medium" style={{ color: 'rgb(var(--ov) / 0.3)' }}>No documents loaded</p>
              <p className="mt-1 text-[10.5px] leading-relaxed" style={{ color: 'rgb(var(--ov) / 0.16)', maxWidth: 180 }}>
                Add PDF or DOCX files to start querying
              </p>
            </div>
          )}
        </div>
      </div>

      {/* Bottom add button */}
      <div className="shrink-0 px-4 py-3" style={{ borderTop: '1px solid rgb(var(--ov) / 0.05)' }}>
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

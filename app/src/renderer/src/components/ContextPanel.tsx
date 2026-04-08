import { useState } from 'react'
import { Citation, DocumentRole, FileInfo } from '../../../../../shared/src/types'

const ROLE_LABELS: Record<DocumentRole, { label: string; color: string }> = {
  ClientDocument: { label: 'Client', color: 'text-blue-400 bg-blue-400/10' },
  LegalAuthority: { label: 'Legal', color: 'text-amber-400 bg-amber-400/10' },
  Evidence: { label: 'Evidence', color: 'text-emerald-400 bg-emerald-400/10' },
  Reference: { label: 'Ref', color: 'text-gray-400 bg-gray-400/10' },
}

interface Props {
  files: FileInfo[]
  citations: Citation[]
  isQuerying: boolean
  isLoading: boolean
  collapsed: boolean
  minimized: boolean
  onToggleMinimize: () => void
  onAddFiles: () => void
  onRemoveFile: (id: string) => void
  onClearFiles: () => void
  onViewCitation: (citation: Citation) => void
  activeCitation?: Citation | null
  onExportCitations?: () => void
  caseName?: string
  onSetDocumentRole?: (fileId: string, role: DocumentRole) => void
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
  index,
  onRemove,
  onSetRole,
}: {
  file: FileInfo
  index: number
  onRemove: () => void
  onSetRole?: (fileId: string, role: DocumentRole) => void
}): JSX.Element {
  const [hovered, setHovered] = useState(false)
  const [showFacts, setShowFacts] = useState(false)
  const ext = file.fileName.split('.').pop()?.toUpperCase() ?? 'DOC'
  const exhibitLabel = index < 26 ? `Ex. ${String.fromCharCode(65 + index)}` : `Ex. ${index + 1}`
  const roleInfo = ROLE_LABELS[file.role || 'ClientDocument']

  return (
    <div
      className="px-2 py-2.5 rounded-lg transition-colors"
      style={{ background: hovered ? 'rgb(var(--ov) / 0.03)' : 'transparent' }}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      <div className="flex items-center gap-2.5">
        <span
          className="shrink-0 text-[9px] font-bold px-1.5 py-0.5 rounded"
          style={{ background: 'rgba(201,168,76,0.08)', color: 'rgba(201,168,76,0.65)' }}
        >
          {ext}
        </span>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-1.5">
            <p
              className="text-[11.5px] truncate leading-snug flex-1"
              style={{ color: 'rgb(var(--ov) / 0.6)' }}
              title={file.fileName}
            >
              <span className="font-semibold" style={{ color: 'rgba(201,168,76,0.55)' }}>{exhibitLabel}</span>
              <span style={{ color: 'rgb(var(--ov) / 0.15)', margin: '0 4px' }}>{'\u00B7'}</span>
              {file.fileName}
            </p>
            {/* Role badge — click to cycle through roles */}
            {onSetRole ? (
              <button
                onClick={(e) => {
                  e.stopPropagation()
                  const roles: DocumentRole[] = ['ClientDocument', 'LegalAuthority', 'Evidence', 'Reference']
                  const currentIdx = roles.indexOf(file.role || 'ClientDocument')
                  const nextRole = roles[(currentIdx + 1) % roles.length]
                  onSetRole(file.id, nextRole)
                }}
                className={`inline-flex items-center gap-1 text-[11px] font-medium px-2 py-1 rounded shrink-0 cursor-pointer transition-all hover:brightness-125 ${roleInfo.color}`}
                title="Click to change document role"
              >
                {roleInfo.label}
                <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" className="opacity-50"><path d="M21 2v6h-6"/><path d="M3 12a9 9 0 0 1 15-6.7L21 8"/><path d="M3 22v-6h6"/><path d="M21 12a9 9 0 0 1-15 6.7L3 16"/></svg>
              </button>
            ) : (
              <span className={`inline-block text-[9px] font-medium px-1.5 py-0.5 rounded shrink-0 ${roleInfo.color}`}>
                {roleInfo.label}
              </span>
            )}
          </div>
          <div className="flex items-center gap-2 mt-0.5">
            <p className="text-[10px]" style={{ color: 'rgb(var(--ov) / 0.22)' }}>
              {file.totalPages} {file.totalPages === 1 ? 'page' : 'pages'}
            </p>
          </div>
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

      {/* Fact Sheet (collapsible) */}
      {file.factSheet && (
        <>
          <button
            onClick={() => setShowFacts((v) => !v)}
            className="mt-1 text-[9px] font-medium transition-colors"
            style={{ color: 'rgba(201,168,76,0.5)' }}
            onMouseEnter={(e) => { (e.currentTarget as HTMLButtonElement).style.color = '#c9a84c' }}
            onMouseLeave={(e) => { (e.currentTarget as HTMLButtonElement).style.color = 'rgba(201,168,76,0.5)' }}
          >
            {showFacts ? 'Hide facts' : 'Show facts'}
          </button>
          {showFacts && (
            <div
              className="mt-1.5 px-2 py-1.5 rounded text-[10px]"
              style={{ background: 'var(--bg-alt)', border: '1px solid rgb(var(--ov) / 0.06)' }}
            >
              {file.factSheet.amounts.length > 0 && (
                <div className="mb-1">
                  <span style={{ color: 'rgb(var(--ov) / 0.3)' }}>Amounts:</span>{' '}
                  <span style={{ color: '#c9a84c' }}>{file.factSheet.amounts.slice(0, 4).join(' \u2022 ')}</span>
                </div>
              )}
              {file.factSheet.dates.length > 0 && (
                <div className="mb-1">
                  <span style={{ color: 'rgb(var(--ov) / 0.3)' }}>Dates:</span>{' '}
                  <span style={{ color: 'var(--text)' }}>{file.factSheet.dates.slice(0, 4).join(' \u2022 ')}</span>
                </div>
              )}
              {file.factSheet.keyClauses.length > 0 && (
                <div className="mb-1">
                  <span style={{ color: 'rgb(var(--ov) / 0.3)' }}>Key Clauses:</span>{' '}
                  <span style={{ color: 'var(--text)' }}>{file.factSheet.keyClauses.slice(0, 3).join(' \u2022 ')}</span>
                </div>
              )}
              {file.factSheet.summary && (
                <div>
                  <span style={{ color: 'rgb(var(--ov) / 0.3)' }}>Summary:</span>{' '}
                  <span style={{ color: 'var(--text)' }}>{file.factSheet.summary}</span>
                </div>
              )}
            </div>
          )}
        </>
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
  minimized,
  onToggleMinimize,
  activeCitation,
  onAddFiles,
  onRemoveFile,
  onClearFiles,
  onViewCitation,
  onExportCitations,
  caseName,
  onSetDocumentRole,
}: Props): JSX.Element {
  const hasCitations = citations.length > 0
  const showSources = hasCitations || isQuerying

  // Completely hidden when no files
  if (collapsed) {
    return <aside style={{ width: 0, minWidth: 0, overflow: 'hidden' }} />
  }

  // Minimized strip
  if (minimized) {
    return (
      <aside
        className="flex h-screen shrink-0 flex-col items-center py-0 cursor-pointer"
        style={{
          width: 44,
          minWidth: 44,
          borderLeft: '1px solid rgb(var(--ov) / 0.05)',
          background: 'var(--panel)',
          transition: 'width 0.2s ease, min-width 0.2s ease',
        }}
        onClick={onToggleMinimize}
        title="Expand documents panel"
      >
        {/* Drag region spacer */}
        <div className="drag-region h-11 w-full shrink-0" />

        {/* Document icon with count badge */}
        <div className="relative mt-2 mb-3">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="rgba(201,168,76,0.45)">
            <path d="M2 1.75C2 .784 2.784 0 3.75 0h6.586c.464 0 .909.184 1.237.513l2.914 2.914c.329.328.513.773.513 1.237v9.586A1.75 1.75 0 0 1 13.25 16h-9.5A1.75 1.75 0 0 1 2 14.25z" />
          </svg>
          {files.length > 0 && (
            <span
              className="absolute -top-1.5 -right-2 text-[8px] font-bold px-1 rounded-full"
              style={{ background: '#c9a84c', color: '#0d1117', minWidth: 14, textAlign: 'center' }}
            >
              {files.length}
            </span>
          )}
        </div>

        {/* Citation indicators */}
        {hasCitations && (
          <div className="flex flex-col items-center gap-1.5 mb-3">
            <svg width="12" height="12" viewBox="0 0 16 16" fill="rgba(201,168,76,0.35)">
              <path d="M1 2.75C1 1.784 1.784 1 2.75 1h10.5c.966 0 1.75.784 1.75 1.75v7.5A1.75 1.75 0 0 1 13.25 12H9.06l-2.573 2.573A1.458 1.458 0 0 1 4 13.543V12H2.75A1.75 1.75 0 0 1 1 10.25z" />
            </svg>
            <span className="text-[8px] font-bold" style={{ color: 'rgba(201,168,76,0.5)' }}>
              {citations.length}
            </span>
          </div>
        )}

        {/* Querying indicator */}
        {isQuerying && (
          <div
            className="h-3 w-3 rounded-full animate-spin mt-1"
            style={{ border: '1.5px solid rgba(201,168,76,0.15)', borderTopColor: 'rgba(201,168,76,0.6)' }}
          />
        )}

        {/* Expand chevron at bottom */}
        <div className="mt-auto mb-4" style={{ color: 'rgb(var(--ov) / 0.2)' }}>
          <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor">
            <path d="M9.78 12.78a.75.75 0 0 1-1.06 0L4.47 8.53a.75.75 0 0 1 0-1.06l4.25-4.25a.75.75 0 0 1 1.06 1.06L6.06 8l3.72 3.72a.75.75 0 0 1 0 1.06z"/>
          </svg>
        </div>
      </aside>
    )
  }

  return (
    <aside
      className="flex h-screen shrink-0 flex-col"
      style={{
        width: 300,
        minWidth: 300,
        borderLeft: '1px solid rgb(var(--ov) / 0.05)',
        background: 'var(--panel)',
        overflow: 'hidden',
        transition: 'width 0.2s ease, min-width 0.2s ease',
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
        {/* Minimize button */}
        <button
          onClick={onToggleMinimize}
          title="Minimize panel"
          aria-label="Minimize documents panel"
          className="no-drag flex h-5 w-5 items-center justify-center rounded transition-colors"
          style={{ color: 'rgb(var(--ov) / 0.2)' }}
          onMouseEnter={(e) => { (e.currentTarget as HTMLButtonElement).style.color = 'rgb(var(--ov) / 0.5)' }}
          onMouseLeave={(e) => { (e.currentTarget as HTMLButtonElement).style.color = 'rgb(var(--ov) / 0.2)' }}
        >
          <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor">
            <path d="M6.22 3.22a.75.75 0 0 1 1.06 0l4.25 4.25a.75.75 0 0 1 0 1.06l-4.25 4.25a.75.75 0 0 1-1.06-1.06L9.94 8 6.22 4.28a.75.75 0 0 1 0-1.06z"/>
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
                  {files.map((file, i) => (
                    <FileRow
                      key={file.id}
                      file={file}
                      index={i}
                      onRemove={() => onRemoveFile(file.id)}
                      onSetRole={onSetDocumentRole}
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

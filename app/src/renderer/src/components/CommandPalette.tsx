import { useEffect, useRef, useState, KeyboardEvent } from 'react'

export interface PaletteAction {
  id: string
  label: string
  icon: JSX.Element
  shortcut?: string
  onAction: () => void
}

interface Props {
  actions: PaletteAction[]
  onClose: () => void
}

export default function CommandPalette({ actions, onClose }: Props): JSX.Element {
  const [query, setQuery] = useState('')
  const [selectedIdx, setSelectedIdx] = useState(0)
  const inputRef = useRef<HTMLInputElement>(null)

  const filtered = query.trim()
    ? actions.filter((a) => a.label.toLowerCase().includes(query.toLowerCase()))
    : actions

  useEffect(() => {
    setSelectedIdx(0)
  }, [query])

  useEffect(() => {
    inputRef.current?.focus()
  }, [])

  function handleKeyDown(e: KeyboardEvent<HTMLInputElement>): void {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setSelectedIdx((i) => (i + 1) % filtered.length)
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setSelectedIdx((i) => (i - 1 + filtered.length) % filtered.length)
    } else if (e.key === 'Enter' && filtered.length > 0) {
      e.preventDefault()
      filtered[selectedIdx].onAction()
      onClose()
    } else if (e.key === 'Escape') {
      e.preventDefault()
      onClose()
    }
  }

  return (
    <div
      className="fixed inset-0 z-[9999] flex items-start justify-center pt-[18vh]"
      style={{ background: 'var(--backdrop)', backdropFilter: 'blur(8px)' }}
      onClick={onClose}
    >
      <div
        className="w-full max-w-md rounded-2xl overflow-hidden"
        style={{
          background: 'var(--modal-bg)',
          border: '1px solid rgb(var(--ov) / 0.08)',
          boxShadow: '0 40px 100px var(--shadow-heavy), 0 0 0 1px rgb(var(--ov) / 0.03)',
          animation: 'scaleIn 0.15s ease both',
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Search input */}
        <div
          className="flex items-center gap-3 px-4 py-3"
          style={{ borderBottom: '1px solid rgb(var(--ov) / 0.06)' }}
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="rgb(var(--ov) / 0.3)" strokeWidth="1.5" strokeLinecap="round">
            <circle cx="7" cy="7" r="4.5" />
            <line x1="10.2" y1="10.2" x2="14" y2="14" />
          </svg>
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Type a command\u2026"
            className="flex-1 bg-transparent text-[13px] outline-none"
            style={{ color: 'var(--text)' }}
          />
          <span
            className="text-[10px] font-mono px-1.5 py-0.5 rounded"
            style={{ background: 'rgb(var(--ov) / 0.05)', color: 'rgb(var(--ov) / 0.25)', border: '1px solid rgb(var(--ov) / 0.06)' }}
          >
            esc
          </span>
        </div>

        {/* Actions list */}
        <div className="py-1.5 max-h-[300px] overflow-y-auto">
          {filtered.length === 0 ? (
            <p className="px-4 py-6 text-center text-[12px]" style={{ color: 'rgb(var(--ov) / 0.3)' }}>
              No matching commands
            </p>
          ) : (
            filtered.map((action, idx) => {
              const isSelected = idx === selectedIdx
              return (
                <button
                  key={action.id}
                  onClick={() => { action.onAction(); onClose() }}
                  className="w-full flex items-center gap-3 px-4 py-2.5 text-left transition-colors"
                  style={{
                    background: isSelected ? 'rgba(201,168,76,0.08)' : 'transparent',
                    color: isSelected ? 'var(--gold)' : 'var(--text)',
                  }}
                  onMouseEnter={() => setSelectedIdx(idx)}
                >
                  <span style={{ color: isSelected ? 'var(--gold)' : 'rgb(var(--ov) / 0.35)' }}>
                    {action.icon}
                  </span>
                  <span className="flex-1 text-[12.5px] font-medium">{action.label}</span>
                  {action.shortcut && (
                    <span
                      className="text-[10px] font-mono px-1.5 py-0.5 rounded"
                      style={{ background: 'rgb(var(--ov) / 0.04)', color: 'rgb(var(--ov) / 0.2)', border: '1px solid rgb(var(--ov) / 0.06)' }}
                    >
                      {action.shortcut}
                    </span>
                  )}
                </button>
              )
            })
          )}
        </div>
      </div>
    </div>
  )
}

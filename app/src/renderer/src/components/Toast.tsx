import { useEffect } from 'react'

export interface ToastMessage {
  id: string
  type: 'success' | 'error' | 'info'
  message: string
}

function ToastItem({
  toast,
  onDismiss,
}: {
  toast: ToastMessage
  onDismiss: (id: string) => void
}): JSX.Element {
  useEffect(() => {
    const t = setTimeout(() => onDismiss(toast.id), 3000)
    return () => clearTimeout(t)
  }, [toast.id, onDismiss])

  const s = {
    success: { border: 'rgba(63,185,80,0.28)', accent: '#3fb950' },
    error:   { border: 'rgba(248,81,73,0.28)',  accent: '#f85149' },
    info:    { border: 'rgba(201,168,76,0.28)', accent: '#c9a84c' },
  }[toast.type]

  return (
    <div
      className="pointer-events-auto flex items-center gap-3 px-4 py-3 rounded-xl"
      style={{
        background: 'var(--surface-raised)',
        border: `1px solid ${s.border}`,
        boxShadow: '0 8px 32px var(--shadow-heavy)',
        animation: 'fadeUp 0.22s ease both',
        minWidth: 220,
        maxWidth: 340,
      }}
    >
      <div
        className="shrink-0 w-[18px] h-[18px] rounded-full flex items-center justify-center"
        style={{ background: `${s.accent}18`, border: `1px solid ${s.accent}40` }}
      >
        {toast.type === 'success' && (
          <svg width="9" height="9" viewBox="0 0 10 10" fill="none" stroke={s.accent} strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
            <path d="M2 5l2 2 4-4" />
          </svg>
        )}
        {toast.type === 'error' && (
          <svg width="8" height="8" viewBox="0 0 10 10" fill="none" stroke={s.accent} strokeWidth="1.8" strokeLinecap="round">
            <path d="M2 2l6 6M8 2l-6 6" />
          </svg>
        )}
        {toast.type === 'info' && (
          <svg width="7" height="7" viewBox="0 0 10 10" fill="none" stroke={s.accent} strokeWidth="2" strokeLinecap="round">
            <path d="M5 4.5v3M5 2.5h.01" />
          </svg>
        )}
      </div>

      <p className="flex-1 text-[12px] leading-snug" style={{ color: 'rgb(var(--ov) / 0.78)' }}>
        {toast.message}
      </p>

      <button
        onClick={() => onDismiss(toast.id)}
        aria-label="Dismiss notification"
        style={{ color: 'rgb(var(--ov) / 0.2)' }}
        onMouseEnter={(e) => { (e.currentTarget as HTMLButtonElement).style.color = 'rgb(var(--ov) / 0.5)' }}
        onMouseLeave={(e) => { (e.currentTarget as HTMLButtonElement).style.color = 'rgb(var(--ov) / 0.2)' }}
      >
        <svg width="10" height="10" viewBox="0 0 12 12" fill="currentColor">
          <path d="M1.22 1.22a.75.75 0 0 1 1.06 0L6 4.94l3.72-3.72a.75.75 0 1 1 1.06 1.06L7.06 6l3.72 3.72a.75.75 0 1 1-1.06 1.06L6 7.06l-3.72 3.72a.75.75 0 0 1-1.06-1.06L4.94 6 1.22 2.28a.75.75 0 0 1 0-1.06z" />
        </svg>
      </button>
    </div>
  )
}

interface Props {
  toasts: ToastMessage[]
  onDismiss: (id: string) => void
}

export default function Toast({ toasts, onDismiss }: Props): JSX.Element | null {
  if (toasts.length === 0) return null
  return (
    <div className="fixed bottom-6 right-6 z-[60] flex flex-col gap-2 pointer-events-none" role="status" aria-live="polite">
      {toasts.map((t) => (
        <ToastItem key={t.id} toast={t} onDismiss={onDismiss} />
      ))}
    </div>
  )
}

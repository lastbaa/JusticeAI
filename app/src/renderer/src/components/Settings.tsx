import { useState } from 'react'
import { AppSettings, OllamaStatus } from '../../../../../shared/src/types'

interface Props {
  settings: AppSettings
  ollamaStatus: OllamaStatus | null
  onSave: (settings: AppSettings) => void
  onClose: () => void
  onCheckStatus: () => Promise<void>
}

function Field({
  label,
  description,
  children,
}: {
  label: string
  description?: string
  children: React.ReactNode
}): JSX.Element {
  return (
    <div>
      <label className="block text-[12px] font-medium text-white mb-1">{label}</label>
      {description && (
        <p className="text-[11px] mb-2" style={{ color: 'rgba(255,255,255,0.35)' }}>
          {description}
        </p>
      )}
      {children}
    </div>
  )
}

function HfTokenInput({
  value,
  onChange,
}: {
  value: string
  onChange: (v: string) => void
}): JSX.Element {
  const [visible, setVisible] = useState(false)
  return (
    <div className="relative flex items-center">
      <input
        type={visible ? 'text' : 'password'}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder="hf_••••••••••••••••••••"
        className="w-full rounded-lg px-3 py-2 pr-10 text-[12px] text-white placeholder-white/20 outline-none transition-colors font-mono"
        style={{ background: '#060606', border: '1px solid rgba(255,255,255,0.08)' }}
        onFocus={(e) => {
          ;(e.target as HTMLInputElement).style.borderColor = 'rgba(201,168,76,0.4)'
        }}
        onBlur={(e) => {
          ;(e.target as HTMLInputElement).style.borderColor = 'rgba(255,255,255,0.08)'
        }}
      />
      <button
        type="button"
        onClick={() => setVisible((v) => !v)}
        className="absolute right-2.5 flex h-5 w-5 items-center justify-center"
        style={{ color: 'rgba(255,255,255,0.3)' }}
      >
        {visible ? (
          <svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor">
            <path d="M.143 2.31a.75.75 0 0 1 1.047-.167l14.5 10.5a.75.75 0 1 1-.88 1.214l-2.248-1.628C11.346 12.769 9.792 13 8 13c-3.73 0-6.849-2.07-8.123-5.062a.75.75 0 0 1 0-.876C.515 5.796 1.48 4.57 2.72 3.65L.31 2.007A.75.75 0 0 1 .143 2.31zm5.56 4.036a2.5 2.5 0 0 0 3.408 3.408l-3.408-3.408z" />
            <path d="M12.034 9.512A2.5 2.5 0 0 0 8.488 5.966l-.68-.492A3.99 3.99 0 0 1 12 8c0 .553-.107 1.082-.304 1.566l.338.245-.001-.001z" />
          </svg>
        ) : (
          <svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor">
            <path d="M8 2c3.73 0 6.849 2.07 8.123 5.062a.75.75 0 0 1 0 .876C14.849 11.93 11.73 14 8 14c-3.73 0-6.849-2.07-8.123-5.062a.75.75 0 0 1 0-.876C1.151 5.07 4.27 3 8 3zm0 1.5a5.5 5.5 0 1 0 0 11 5.5 5.5 0 0 0 0-11zM8 6a2 2 0 1 1 0 4 2 2 0 0 1 0-4z" />
          </svg>
        )}
      </button>
    </div>
  )
}

function NumberInput({
  value,
  onChange,
  min,
  max,
}: {
  value: number
  onChange: (v: number) => void
  min: number
  max: number
}): JSX.Element {
  return (
    <input
      type="number"
      value={value}
      onChange={(e) => onChange(Number(e.target.value))}
      min={min}
      max={max}
      className="w-full rounded-lg px-3 py-2 text-[12px] text-white outline-none transition-colors"
      style={{ background: '#060606', border: '1px solid rgba(255,255,255,0.08)' }}
      onFocus={(e) => {
        ;(e.target as HTMLInputElement).style.borderColor = 'rgba(201,168,76,0.4)'
      }}
      onBlur={(e) => {
        ;(e.target as HTMLInputElement).style.borderColor = 'rgba(255,255,255,0.08)'
      }}
    />
  )
}

function SectionHeader({ children }: { children: React.ReactNode }): JSX.Element {
  return (
    <h3
      className="text-[10px] font-semibold uppercase tracking-[0.12em] mb-4 pb-2 border-b"
      style={{ color: 'rgba(255,255,255,0.3)', borderColor: 'rgba(255,255,255,0.06)' }}
    >
      {children}
    </h3>
  )
}

export default function Settings({
  settings,
  ollamaStatus,
  onSave,
  onClose,
  onCheckStatus,
}: Props): JSX.Element {
  const [local, setLocal] = useState<AppSettings>({ ...settings })
  const [isChecking, setIsChecking] = useState(false)

  function update<K extends keyof AppSettings>(key: K, value: AppSettings[K]): void {
    setLocal((prev) => ({ ...prev, [key]: value }))
  }

  async function handleCheckStatus(): Promise<void> {
    setIsChecking(true)
    await onCheckStatus()
    setIsChecking(false)
  }

  function handleSave(): void {
    onSave(local)
  }

  const hasToken = local.hfToken.trim().length > 0

  const statusInfo = (() => {
    if (!hasToken)
      return {
        dot: '#f85149',
        label: 'HuggingFace token required',
        detail: 'Add your free token below to get started.',
        ready: false,
      }
    if (!ollamaStatus)
      return {
        dot: '#e3b341',
        label: 'Not verified',
        detail: 'Click "Check Connection" to verify your token.',
        ready: false,
      }
    if (!ollamaStatus.running)
      return {
        dot: '#e3b341',
        label: 'Cannot reach HuggingFace',
        detail: 'Check your internet connection and try again.',
        ready: false,
      }
    return {
      dot: '#3fb950',
      label: 'Ready',
      detail: 'Saul-7B-Instruct · all-MiniLM-L6-v2 embeddings · via HuggingFace',
      ready: true,
    }
  })()

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center"
      style={{ background: 'rgba(0,0,0,0.7)', backdropFilter: 'blur(8px)' }}
    >
      <div
        className="w-full max-w-lg rounded-2xl overflow-hidden"
        style={{
          background: '#0a0a0a',
          border: '1px solid rgba(255,255,255,0.08)',
          boxShadow: '0 40px 100px rgba(0,0,0,0.8)',
        }}
      >
        {/* Header */}
        <div
          className="flex items-center justify-between px-6 py-4"
          style={{ borderBottom: '1px solid rgba(255,255,255,0.06)' }}
        >
          <div className="flex items-center gap-2.5">
            <svg width="15" height="15" viewBox="0 0 20 20" fill="none">
              <rect x="1" y="3" width="11" height="4" rx="1.25" fill="#c9a84c" transform="rotate(45 6.5 5)" />
              <line x1="10.5" y1="10.5" x2="18.5" y2="18.5" stroke="#c9a84c" strokeWidth="2.5" strokeLinecap="round" />
              <rect x="0.5" y="16.5" width="8.5" height="2.5" rx="0.75" fill="#c9a84c" opacity="0.38" />
            </svg>
            <h2 className="text-[14px] font-semibold text-white">Settings</h2>
          </div>
          <button
            onClick={onClose}
            className="flex h-7 w-7 items-center justify-center rounded-lg transition-colors"
            style={{ color: 'rgba(255,255,255,0.3)' }}
            onMouseEnter={(e) => {
              ;(e.currentTarget as HTMLButtonElement).style.color = 'rgba(255,255,255,0.7)'
            }}
            onMouseLeave={(e) => {
              ;(e.currentTarget as HTMLButtonElement).style.color = 'rgba(255,255,255,0.3)'
            }}
          >
            <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
              <path d="M3.72 3.72a.75.75 0 0 1 1.06 0L8 6.94l3.22-3.22a.75.75 0 1 1 1.06 1.06L9.06 8l3.22 3.22a.75.75 0 1 1-1.06 1.06L8 9.06l-3.22 3.22a.75.75 0 0 1-1.06-1.06L6.94 8 3.72 4.78a.75.75 0 0 1 0-1.06z" />
            </svg>
          </button>
        </div>

        {/* Body */}
        <div
          className="overflow-y-auto px-6 py-5 flex flex-col gap-6"
          style={{ maxHeight: '65vh' }}
        >
          {/* ── First-run guide ── */}
          {!hasToken && (
            <div
              className="rounded-xl px-4 py-4 flex flex-col gap-3"
              style={{ background: 'rgba(201,168,76,0.05)', border: '1px solid rgba(201,168,76,0.18)' }}
            >
              <p
                className="text-[11px] font-bold uppercase tracking-[0.14em]"
                style={{ color: 'rgba(201,168,76,0.8)' }}
              >
                One-Time Setup — 2 Steps
              </p>
              <div className="flex flex-col gap-3">
                <div className="flex items-start gap-3">
                  <span
                    className="shrink-0 flex h-5 w-5 items-center justify-center rounded-full text-[10px] font-bold"
                    style={{ background: 'rgba(201,168,76,0.15)', color: '#c9a84c' }}
                  >
                    1
                  </span>
                  <div>
                    <p className="text-[11px] font-semibold text-white mb-0.5">
                      Create a free HuggingFace account
                    </p>
                    <p
                      className="text-[10px] leading-relaxed"
                      style={{ color: 'rgba(255,255,255,0.35)' }}
                    >
                      Go to{' '}
                      <span style={{ color: 'rgba(201,168,76,0.7)' }}>
                        huggingface.co/settings/tokens
                      </span>{' '}
                      and create a token with Read access. It&apos;s free.
                    </p>
                  </div>
                </div>
                <div className="flex items-start gap-3">
                  <span
                    className="shrink-0 flex h-5 w-5 items-center justify-center rounded-full text-[10px] font-bold"
                    style={{ background: 'rgba(201,168,76,0.15)', color: '#c9a84c' }}
                  >
                    2
                  </span>
                  <div>
                    <p className="text-[11px] font-semibold text-white mb-0.5">
                      Paste your token below
                    </p>
                    <p
                      className="text-[10px] leading-relaxed"
                      style={{ color: 'rgba(255,255,255,0.35)' }}
                    >
                      That&apos;s it. No installs. No command line. Justice AI handles the rest.
                    </p>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* ── Connection Status ── */}
          <div>
            <SectionHeader>Connection Status</SectionHeader>
            <div
              className="rounded-xl p-4 flex flex-col gap-3"
              style={{ background: '#060606', border: '1px solid rgba(255,255,255,0.06)' }}
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2.5">
                  <div className="w-2 h-2 rounded-full shrink-0" style={{ background: statusInfo.dot }} />
                  <span className="text-[13px] font-semibold text-white">{statusInfo.label}</span>
                </div>
                <button
                  onClick={handleCheckStatus}
                  disabled={isChecking}
                  className="flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-[11px] font-medium transition-all disabled:opacity-50"
                  style={{
                    background: 'rgba(255,255,255,0.06)',
                    border: '1px solid rgba(255,255,255,0.08)',
                    color: 'rgba(255,255,255,0.6)',
                  }}
                >
                  {isChecking ? (
                    <svg className="animate-spin w-3 h-3" viewBox="0 0 24 24" fill="none">
                      <circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="3" opacity="0.2" />
                      <path
                        d="M12 2a10 10 0 0110 10"
                        stroke="currentColor"
                        strokeWidth="3"
                        strokeLinecap="round"
                      />
                    </svg>
                  ) : (
                    <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor">
                      <path d="M8 2.5a5.487 5.487 0 0 0-4.131 1.869l1.204 1.204A.25.25 0 0 1 4.896 6H1.25A.25.25 0 0 1 1 5.75V2.104a.25.25 0 0 1 .427-.177l1.38 1.38A7.001 7.001 0 0 1 14.95 7.16a.75.75 0 0 1-1.49.178A5.501 5.501 0 0 0 8 2.5zM1.705 8.005a.75.75 0 0 1 .834.656 5.501 5.501 0 0 0 9.592 2.97l-1.204-1.204a.25.25 0 0 1 .177-.427h3.646a.25.25 0 0 1 .25.25v3.646a.25.25 0 0 1-.427.177l-1.38-1.38A7.001 7.001 0 0 1 1.05 8.84a.75.75 0 0 1 .656-.834z" />
                    </svg>
                  )}
                  {isChecking ? 'Checking…' : 'Check Connection'}
                </button>
              </div>

              <p
                className="text-[11px] leading-relaxed"
                style={{ color: 'rgba(255,255,255,0.35)' }}
              >
                {statusInfo.detail}
              </p>

              {statusInfo.ready && (
                <div className="flex gap-2 flex-wrap">
                  <span
                    className="text-[10px] font-medium px-2 py-1 rounded-md"
                    style={{
                      background: 'rgba(63,185,80,0.08)',
                      border: '1px solid rgba(63,185,80,0.2)',
                      color: '#3fb950',
                    }}
                  >
                    ✓ Saul-7B via HuggingFace
                  </span>
                  <span
                    className="text-[10px] font-medium px-2 py-1 rounded-md"
                    style={{
                      background: 'rgba(63,185,80,0.08)',
                      border: '1px solid rgba(63,185,80,0.2)',
                      color: '#3fb950',
                    }}
                  >
                    ✓ Embeddings via HuggingFace
                  </span>
                </div>
              )}
            </div>
          </div>

          {/* ── HuggingFace Token ── */}
          <div>
            <SectionHeader>HuggingFace API</SectionHeader>
            <div className="flex flex-col gap-3">
              <div
                className="rounded-xl px-4 py-3 flex items-start gap-3"
                style={{ background: 'rgba(201,168,76,0.04)', border: '1px solid rgba(201,168,76,0.12)' }}
              >
                <svg width="13" height="13" viewBox="0 0 16 16" fill="#c9a84c" className="shrink-0 mt-0.5">
                  <path d="M8 1a2 2 0 0 1 2 2v4H6V3a2 2 0 0 1 2-2zm3 6V3a3 3 0 0 0-6 0v4a2 2 0 0 0-2 2v5a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2z" />
                </svg>
                <div>
                  <p className="text-[11px] font-semibold mb-0.5" style={{ color: '#c9a84c' }}>
                    All AI runs via HuggingFace — no local installs
                  </p>
                  <p
                    className="text-[11px] leading-relaxed"
                    style={{ color: 'rgba(255,255,255,0.32)' }}
                  >
                    Get a free token at{' '}
                    <span style={{ color: 'rgba(201,168,76,0.7)' }}>
                      huggingface.co/settings/tokens
                    </span>{' '}
                    (read access is enough). Both Saul-7B and embeddings use this token.
                  </p>
                </div>
              </div>
              <Field
                label="HuggingFace Token"
                description="Used for Saul-7B-Instruct answers and document embeddings."
              >
                <HfTokenInput value={local.hfToken} onChange={(v) => update('hfToken', v)} />
              </Field>
            </div>
          </div>

          {/* ── RAG Configuration ── */}
          <div>
            <SectionHeader>Search Configuration</SectionHeader>
            <div className="grid grid-cols-3 gap-4">
              <Field label="Chunk Size" description="Characters per chunk (100–2000)">
                <NumberInput
                  value={local.chunkSize}
                  onChange={(v) => update('chunkSize', Math.max(100, Math.min(2000, v)))}
                  min={100}
                  max={2000}
                />
              </Field>
              <Field label="Chunk Overlap" description="Overlap between chunks (0–200)">
                <NumberInput
                  value={local.chunkOverlap}
                  onChange={(v) => update('chunkOverlap', Math.max(0, Math.min(200, v)))}
                  min={0}
                  max={200}
                />
              </Field>
              <Field label="Top-K Results" description="Chunks retrieved per query (1–20)">
                <NumberInput
                  value={local.topK}
                  onChange={(v) => update('topK', Math.max(1, Math.min(20, v)))}
                  min={1}
                  max={20}
                />
              </Field>
            </div>
          </div>

          {/* ── Privacy notice ── */}
          <div
            className="rounded-xl px-4 py-3 flex items-start gap-3"
            style={{ background: 'rgba(63,185,80,0.04)', border: '1px solid rgba(63,185,80,0.12)' }}
          >
            <svg
              width="13"
              height="13"
              viewBox="0 0 16 16"
              fill="none"
              className="shrink-0 mt-0.5"
            >
              <path
                d="M8.533.133a1.75 1.75 0 0 0-1.066 0l-5.25 1.68A1.75 1.75 0 0 0 1 3.48V7c0 1.566.832 3.125 2.561 4.608.458.391.978.752 1.535 1.078a11.865 11.865 0 0 0 2.904 1.218c1.11 0 3.028-.877 4.439-2.296C13.168 10.125 14 8.566 14 7V3.48a1.75 1.75 0 0 0-1.217-1.667L8.533.133zm-.61 1.429a.25.25 0 0 1 .153 0l5.25 1.68a.25.25 0 0 1 .174.237V7c0 1.32-.69 2.6-2.249 3.933C10.157 12.022 8.63 12.75 8 12.75c-.63 0-2.157-.728-3.251-1.817C3.19 9.6 2.5 8.32 2.5 7V3.48a.25.25 0 0 1 .174-.238z"
                stroke="#3fb950"
                strokeWidth="0.3"
                fill="#3fb950"
                opacity="0.7"
              />
              <path
                d="M5.5 8l2 2 3-3"
                stroke="#3fb950"
                strokeWidth="1.3"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
            <div>
              <p className="text-[11px] font-semibold mb-0.5" style={{ color: '#3fb950' }}>
                Privacy Guarantee
              </p>
              <p
                className="text-[11px] leading-relaxed"
                style={{ color: 'rgba(255,255,255,0.32)' }}
              >
                Your documents are never uploaded — they stay on your machine. Only your query text
                is sent to HuggingFace for processing.
              </p>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div
          className="flex items-center justify-end gap-3 px-6 py-4"
          style={{ borderTop: '1px solid rgba(255,255,255,0.06)' }}
        >
          <button
            onClick={onClose}
            className="rounded-lg px-4 py-2 text-[12px] font-medium transition-colors"
            style={{
              background: 'transparent',
              border: '1px solid rgba(255,255,255,0.08)',
              color: 'rgba(255,255,255,0.4)',
            }}
            onMouseEnter={(e) => {
              ;(e.currentTarget as HTMLButtonElement).style.color = 'rgba(255,255,255,0.7)'
            }}
            onMouseLeave={(e) => {
              ;(e.currentTarget as HTMLButtonElement).style.color = 'rgba(255,255,255,0.4)'
            }}
          >
            Cancel
          </button>
          <button
            onClick={handleSave}
            className="rounded-lg px-5 py-2 text-[12px] font-semibold transition-colors"
            style={{ background: '#c9a84c', color: '#080808' }}
            onMouseEnter={(e) => {
              ;(e.currentTarget as HTMLButtonElement).style.background = '#e8c97e'
            }}
            onMouseLeave={(e) => {
              ;(e.currentTarget as HTMLButtonElement).style.background = '#c9a84c'
            }}
          >
            Save Settings
          </button>
        </div>
      </div>
    </div>
  )
}

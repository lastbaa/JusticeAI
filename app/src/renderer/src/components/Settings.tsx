import { useState } from 'react'
import { AppSettings } from '../../../../../shared/src/types'

interface Props {
  settings: AppSettings
  onSave: (settings: AppSettings) => void
  onClose: () => void
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

export default function Settings({ settings, onSave, onClose }: Props): JSX.Element {
  const [local, setLocal] = useState<AppSettings>({ ...settings })

  function update<K extends keyof AppSettings>(key: K, value: AppSettings[K]): void {
    setLocal((prev) => ({ ...prev, [key]: value }))
  }

  function handleSave(): void {
    onSave(local)
  }

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
          {/* ── Model Status ── */}
          <div>
            <SectionHeader>AI Model</SectionHeader>
            <div
              className="rounded-xl p-4 flex items-center gap-3"
              style={{ background: '#060606', border: '1px solid rgba(255,255,255,0.06)' }}
            >
              <div className="w-2 h-2 rounded-full shrink-0" style={{ background: '#3fb950' }} />
              <div>
                <p className="text-[13px] font-semibold text-white">Saul-7B running locally</p>
                <p className="text-[11px] mt-0.5" style={{ color: 'rgba(255,255,255,0.35)' }}>
                  all-MiniLM-L6-v2 embeddings · Saul-7B-Instruct · fully on-device
                </p>
              </div>
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
                100% Private
              </p>
              <p
                className="text-[11px] leading-relaxed"
                style={{ color: 'rgba(255,255,255,0.32)' }}
              >
                Documents and queries never leave your machine. All AI processing runs locally —
                no accounts, no API keys, no network traffic.
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

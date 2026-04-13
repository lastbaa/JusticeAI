import { useState, useEffect, useRef } from 'react'
import { AppSettings, Jurisdiction, JurisdictionLevel, Theme } from '../../../../../shared/src/types'

// ── Practice Area Presets ─────────────────────────────────────────────────────

interface Preset {
  name: string
  chunkSize: number
  chunkOverlap: number
  topK: number
}

const PRESETS: Preset[] = [
  { name: 'General',               chunkSize: 1000, chunkOverlap: 150, topK: 6 },
  { name: 'Criminal Law',          chunkSize: 1200, chunkOverlap: 200, topK: 8 },
  { name: 'Family / Domestic',     chunkSize: 800,  chunkOverlap: 100, topK: 5 },
  { name: 'Corporate / Contract',  chunkSize: 1500, chunkOverlap: 200, topK: 8 },
  { name: 'Immigration',           chunkSize: 1000, chunkOverlap: 150, topK: 7 },
  { name: 'Personal Injury',       chunkSize: 1000, chunkOverlap: 120, topK: 7 },
  { name: 'Real Estate / Property',chunkSize: 1200, chunkOverlap: 180, topK: 7 },
  { name: 'Employment / Labor',    chunkSize: 1100, chunkOverlap: 160, topK: 7 },
  { name: 'Regulatory / Compliance', chunkSize: 1400, chunkOverlap: 200, topK: 8 },
]

const US_STATES = [
  'Alabama', 'Alaska', 'Arizona', 'Arkansas', 'California', 'Colorado',
  'Connecticut', 'Delaware', 'Florida', 'Georgia', 'Hawaii', 'Idaho',
  'Illinois', 'Indiana', 'Iowa', 'Kansas', 'Kentucky', 'Louisiana',
  'Maine', 'Maryland', 'Massachusetts', 'Michigan', 'Minnesota',
  'Mississippi', 'Missouri', 'Montana', 'Nebraska', 'Nevada',
  'New Hampshire', 'New Jersey', 'New Mexico', 'New York',
  'North Carolina', 'North Dakota', 'Ohio', 'Oklahoma', 'Oregon',
  'Pennsylvania', 'Rhode Island', 'South Carolina', 'South Dakota',
  'Tennessee', 'Texas', 'Utah', 'Vermont', 'Virginia', 'Washington',
  'West Virginia', 'Wisconsin', 'Wyoming',
]

function findActivePreset(s: AppSettings): string | null {
  const match = PRESETS.find(
    (p) => p.chunkSize === s.chunkSize && p.chunkOverlap === s.chunkOverlap && p.topK === s.topK
  )
  return match?.name ?? 'General'
}

// ── Shared UI Components ──────────────────────────────────────────────────────

interface Props {
  settings: AppSettings
  onSave: (settings: AppSettings) => void
  onClose: () => void
  onReindex?: () => void
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
      <label className="block text-[12px] font-medium mb-1" style={{ color: 'var(--text)' }}>{label}</label>
      {description && (
        <p className="text-[11px] mb-2" style={{ color: 'rgb(var(--ov) / 0.45)' }}>
          {description}
        </p>
      )}
      {children}
    </div>
  )
}

// ── Custom styled select (replaces ugly native <select>) ─────────────────────
function StyledSelect({
  value,
  options,
  onChange,
}: {
  value: string
  options: { value: string; label: string }[]
  onChange: (value: string) => void
}): JSX.Element {
  const [open, setOpen] = useState(false)
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    function handleClick(e: MouseEvent): void {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false)
    }
    document.addEventListener('mousedown', handleClick)
    return () => document.removeEventListener('mousedown', handleClick)
  }, [open])

  const selected = options.find((o) => o.value === value)

  return (
    <div ref={ref} className="relative">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        onKeyDown={(e) => {
          if (e.key === 'Escape' && open) { e.stopPropagation(); setOpen(false) }
          if ((e.key === 'ArrowDown' || e.key === 'ArrowUp') && !open) { setOpen(true) }
        }}
        role="combobox"
        aria-expanded={open}
        aria-haspopup="listbox"
        className="w-full flex items-center justify-between rounded-lg px-3 py-2 text-[12px] outline-none transition-colors"
        style={{
          background: 'var(--surface-dark)',
          color: 'var(--text)',
          border: open ? '1px solid rgba(201,168,76,0.35)' : '1px solid rgb(var(--ov) / 0.08)',
        }}
      >
        <span>{selected?.label ?? value}</span>
        <svg
          width="10" height="10" viewBox="0 0 10 10" fill="none"
          stroke="rgb(var(--ov) / 0.35)" strokeWidth="1.6" strokeLinecap="round"
          style={{ transform: open ? 'rotate(180deg)' : 'rotate(0deg)', transition: 'transform 0.15s ease' }}
        >
          <path d="M2 3.5l3 3 3-3" />
        </svg>
      </button>
      {open && (
        <div
          className="absolute z-50 mt-1 w-full rounded-lg py-1 overflow-auto"
          style={{
            background: 'var(--surface-dark)',
            border: '1px solid rgba(201,168,76,0.15)',
            boxShadow: '0 8px 24px rgba(0,0,0,0.4)',
            maxHeight: 200,
          }}
        >
          {options.map((opt) => (
            <button
              key={opt.value}
              type="button"
              onClick={() => { onChange(opt.value); setOpen(false) }}
              className="w-full text-left px-3 py-1.5 text-[12px] transition-colors"
              style={{
                background: opt.value === value ? 'rgba(201,168,76,0.1)' : 'transparent',
                color: opt.value === value ? 'var(--gold)' : 'var(--text)',
              }}
              onMouseEnter={(e) => {
                if (opt.value !== value) (e.currentTarget as HTMLButtonElement).style.background = 'rgb(var(--ov) / 0.06)'
              }}
              onMouseLeave={(e) => {
                (e.currentTarget as HTMLButtonElement).style.background = opt.value === value ? 'rgba(201,168,76,0.1)' : 'transparent'
              }}
            >
              {opt.label}
            </button>
          ))}
        </div>
      )}
    </div>
  )
}

function SectionHeader({ children }: { children: React.ReactNode }): JSX.Element {
  return (
    <h3
      className="text-[11px] font-semibold uppercase tracking-[0.12em] mb-4 pb-2 border-b"
      style={{ color: 'var(--gold)', borderColor: 'rgba(201,168,76,0.15)' }}
    >
      {children}
    </h3>
  )
}

function ThemeToggle({ value, onChange }: { value: Theme; onChange: (t: Theme) => void }): JSX.Element {
  return (
    <div
      className="flex rounded-lg overflow-hidden w-full"
      style={{ border: '1px solid rgb(var(--ov) / 0.08)', background: 'var(--surface-dark)' }}
    >
      {(['dark', 'light'] as Theme[]).map((t) => (
        <button
          key={t}
          onClick={() => onChange(t)}
          aria-label={`Switch to ${t} theme`}
          className="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 text-[12px] font-medium transition-all"
          style={{
            background: value === t ? 'rgba(201,168,76,0.12)' : 'transparent',
            color: value === t ? 'var(--gold)' : 'rgb(var(--ov) / 0.4)',
            borderRight: t === 'dark' ? '1px solid rgb(var(--ov) / 0.08)' : 'none',
          }}
        >
          {t === 'dark' ? (
            <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
              <path d="M6.2 1.013a.75.75 0 0 1 .206.735A5.5 5.5 0 0 0 14.252 9.8a.75.75 0 0 1 .943.936A8 8 0 1 1 5.467.207a.75.75 0 0 1 .733.806z" />
            </svg>
          ) : (
            <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
              <path d="M8 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8zm0-1.5a2.5 2.5 0 1 1 0-5 2.5 2.5 0 0 1 0 5zm5.657-8.157a.75.75 0 0 1 0 1.06l-1.061 1.061a.75.75 0 0 1-1.06-1.06l1.06-1.061a.75.75 0 0 1 1.06 0zm-9.193 9.193a.75.75 0 0 1 0 1.06l-1.06 1.061a.75.75 0 1 1-1.061-1.06l1.06-1.061a.75.75 0 0 1 1.061 0zM8 0a.75.75 0 0 1 .75.75v1.5a.75.75 0 0 1-1.5 0V.75A.75.75 0 0 1 8 0zM3.404 2.343a.75.75 0 0 1 0 1.06L2.343 4.404a.75.75 0 0 1-1.06-1.06l1.06-1.061a.75.75 0 0 1 1.06 0zM0 8a.75.75 0 0 1 .75-.75h1.5a.75.75 0 0 1 0 1.5H.75A.75.75 0 0 1 0 8zm2.343 4.596a.75.75 0 0 1 1.06 0l1.061 1.06a.75.75 0 0 1-1.06 1.061l-1.061-1.06a.75.75 0 0 1 0-1.06zM8 14.25a.75.75 0 0 1 .75.75v1a.75.75 0 0 1-1.5 0v-1a.75.75 0 0 1 .75-.75zm4.596-1.907a.75.75 0 0 1 1.06 0l1.061 1.06a.75.75 0 0 1-1.06 1.061l-1.061-1.06a.75.75 0 0 1 0-1.06zM14.25 8a.75.75 0 0 1 .75-.75h1a.75.75 0 0 1 0 1.5h-1a.75.75 0 0 1-.75-.75z" />
            </svg>
          )}
          {t === 'dark' ? 'Dark' : 'Light'}
        </button>
      ))}
    </div>
  )
}

// ── Slider ────────────────────────────────────────────────────────────────────

function Slider({
  label,
  value,
  onChange,
  min,
  max,
  step,
}: {
  label: string
  value: number
  onChange: (v: number) => void
  min: number
  max: number
  step: number
}): JSX.Element {
  const pct = ((value - min) / (max - min)) * 100

  return (
    <div className="mb-1">
      <div className="flex items-center justify-between mb-2">
        <span className="text-[12px] font-medium" style={{ color: 'var(--text)' }}>{label}</span>
        <span
          className="text-[13px] font-mono font-semibold tabular-nums"
          style={{ color: 'var(--gold)' }}
        >
          {value}
        </span>
      </div>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        aria-label={`${label}: ${value}`}
        className="slider-range w-full h-[6px] rounded-full appearance-none cursor-pointer outline-none"
        style={{
          background: `linear-gradient(to right, var(--gold) 0%, var(--gold) ${pct}%, rgb(var(--ov) / 0.1) ${pct}%, rgb(var(--ov) / 0.1) 100%)`,
        }}
      />
      <div className="flex justify-between mt-1">
        <span className="text-[10px]" style={{ color: 'rgb(var(--ov) / 0.4)' }}>{min}</span>
        <span className="text-[10px]" style={{ color: 'rgb(var(--ov) / 0.4)' }}>{max}</span>
      </div>
    </div>
  )
}

// Inject slider thumb styles (can't style ::-webkit-slider-thumb inline)
function useSliderStyles(): void {
  useEffect(() => {
    if (document.getElementById('slider-thumb-styles')) return
    const style = document.createElement('style')
    style.id = 'slider-thumb-styles'
    style.textContent = `
      .slider-range::-webkit-slider-thumb {
        -webkit-appearance: none;
        appearance: none;
        width: 16px;
        height: 16px;
        border-radius: 50%;
        background: var(--gold);
        border: 2px solid var(--surface-dark);
        cursor: pointer;
        box-shadow: 0 1px 4px rgba(0,0,0,0.3);
        transition: transform 0.1s;
      }
      .slider-range::-webkit-slider-thumb:hover {
        transform: scale(1.15);
      }
      .slider-range::-moz-range-thumb {
        width: 16px;
        height: 16px;
        border-radius: 50%;
        background: var(--gold);
        border: 2px solid var(--surface-dark);
        cursor: pointer;
        box-shadow: 0 1px 4px rgba(0,0,0,0.3);
      }
    `
    document.head.appendChild(style)
  }, [])
}

// ── Main Component ────────────────────────────────────────────────────────────

export default function Settings({ settings, onSave, onClose, onReindex }: Props): JSX.Element {
  const [local, setLocal] = useState<AppSettings>({ ...settings })
  const [validationError, setValidationError] = useState<string | null>(null)
  const [buildInfo, setBuildInfo] = useState<string>('')
  const [showReindexWarning, setShowReindexWarning] = useState(false)

  useSliderStyles()

  useEffect(() => {
    window.api.getBuildInfo().then(setBuildInfo).catch(() => {})
  }, [])

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent): void {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [onClose])

  function update<K extends keyof AppSettings>(key: K, value: AppSettings[K]): void {
    setLocal((prev) => {
      const next = { ...prev, [key]: value }
      // Dismiss re-index warning if chunk settings revert to original
      if (showReindexWarning && next.chunkSize === settings.chunkSize && next.chunkOverlap === settings.chunkOverlap) {
        setShowReindexWarning(false)
      }
      return next
    })
  }

  // Apply theme preview instantly while editing
  useEffect(() => {
    document.documentElement.setAttribute('data-theme', local.theme)
  }, [local.theme])

  // Revert theme if user cancels
  function handleClose(): void {
    document.documentElement.setAttribute('data-theme', settings.theme)
    onClose()
  }

  function handleSave(): void {
    if (local.chunkOverlap >= local.chunkSize) {
      setValidationError('Chunk overlap must be less than chunk size.')
      return
    }
    setValidationError(null)

    // Show re-index warning if chunk settings changed
    if (local.chunkSize !== settings.chunkSize || local.chunkOverlap !== settings.chunkOverlap) {
      setShowReindexWarning(true)
      return
    }

    onSave(local)
  }

  function handleSaveAndReindex(): void {
    onSave(local)
    onReindex?.()
  }

  function handleSaveSkipReindex(): void {
    onSave(local)
  }

  const activePreset = findActivePreset(local)

  function applyPreset(p: Preset): void {
    setLocal((prev) => ({
      ...prev,
      chunkSize: p.chunkSize,
      chunkOverlap: p.chunkOverlap,
      topK: p.topK,
    }))
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center"
      style={{ background: 'var(--backdrop)', backdropFilter: 'blur(8px)' }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Settings"
        className="w-full max-w-xl rounded-2xl overflow-hidden"
        style={{
          background: 'var(--modal-bg)',
          border: '1px solid rgb(var(--ov) / 0.08)',
          boxShadow: '0 40px 100px var(--shadow-heavy), 0 0 0 1px rgb(var(--ov) / 0.03)',
          animation: 'scaleIn 0.2s ease both',
        }}
      >
        {/* Header */}
        <div
          className="flex items-center justify-between px-6 py-4"
          style={{ borderBottom: '1px solid rgb(var(--ov) / 0.06)' }}
        >
          <div className="flex items-center gap-2.5">
            <svg width="15" height="15" viewBox="0 0 28 28" fill="none">
              <circle cx="14" cy="5" r="1.5" fill="#c9a84c" />
              <rect x="13.25" y="5" width="1.5" height="16" fill="#c9a84c" />
              <rect x="9" y="21" width="10" height="1.5" rx="0.75" fill="#c9a84c" />
              <rect x="12" y="22.5" width="4" height="1.5" rx="0.75" fill="#c9a84c" />
              <rect x="5" y="8.25" width="18" height="1.5" rx="0.75" fill="#c9a84c" />
              <line x1="7" y1="9.75" x2="5.5" y2="17" stroke="#c9a84c" strokeWidth="1.2" strokeLinecap="round" />
              <line x1="21" y1="9.75" x2="22.5" y2="17" stroke="#c9a84c" strokeWidth="1.2" strokeLinecap="round" />
              <path d="M3 17 Q5.5 20 8 17" stroke="#c9a84c" strokeWidth="1.3" fill="none" strokeLinecap="round" />
              <path d="M20 17 Q22.5 20 25 17" stroke="#c9a84c" strokeWidth="1.3" fill="none" strokeLinecap="round" />
            </svg>
            <h2 className="text-[14px] font-semibold" style={{ color: 'var(--text)' }}>Settings</h2>
          </div>
          <button
            onClick={handleClose}
            aria-label="Close settings"
            className="flex h-7 w-7 items-center justify-center rounded-lg transition-colors"
            style={{ color: 'rgb(var(--ov) / 0.3)' }}
            onMouseEnter={(e) => {
              ;(e.currentTarget as HTMLButtonElement).style.color = 'rgb(var(--ov) / 0.7)'
            }}
            onMouseLeave={(e) => {
              ;(e.currentTarget as HTMLButtonElement).style.color = 'rgb(var(--ov) / 0.3)'
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
          style={{ maxHeight: '75vh' }}
        >
          {/* ── Appearance ── */}
          <div>
            <SectionHeader>Appearance</SectionHeader>
            <Field label="Theme" description="Switch between dark and light mode. Default is dark.">
              <ThemeToggle
                value={local.theme}
                onChange={(t) => update('theme', t)}
              />
            </Field>
          </div>

          {/* ── Model Status ── */}
          <div>
            <SectionHeader>AI Model</SectionHeader>
            <div
              className="rounded-xl p-4 flex items-center justify-between gap-3"
              style={{ background: 'var(--surface-dark)', border: '1px solid rgba(201,168,76,0.12)' }}
            >
              <div className="flex items-center gap-3">
                <div
                  className="w-2 h-2 rounded-full shrink-0"
                  style={{ background: 'var(--success)', boxShadow: '0 0 6px rgba(63,185,80,0.5)' }}
                />
                <div>
                  <p className="text-[13px] font-semibold" style={{ color: 'var(--text)' }}>Saul-7B-Instruct</p>
                  <p className="text-[11px] mt-0.5" style={{ color: 'rgb(var(--ov) / 0.35)' }}>
                    BGE-small-en-v1.5 embeddings · 7 billion parameters · running on-device
                  </p>
                </div>
              </div>
              <span
                className="shrink-0 text-[10px] font-semibold tracking-wider uppercase px-2 py-1 rounded-md"
                style={{ background: 'rgba(63,185,80,0.08)', color: 'rgba(63,185,80,0.7)', border: '1px solid rgba(63,185,80,0.15)' }}
              >
                On-Device
              </span>
            </div>
          </div>

          {/* ── RAG Configuration ── */}
          <div>
            <SectionHeader>Search Configuration</SectionHeader>

            {/* Practice Area Presets */}
            <div className="mb-5">
              <label className="block text-[12px] font-medium mb-2" style={{ color: 'var(--text)' }}>
                Practice Area
              </label>
              <div className="grid grid-cols-3 gap-2">
                {PRESETS.map((p) => {
                  const isActive = activePreset === p.name
                  return (
                    <button
                      key={p.name}
                      onClick={() => applyPreset(p)}
                      aria-label={`Apply ${p.name} preset`}
                      className="rounded-lg px-3 py-2 text-[11px] text-center font-medium transition-all"
                      style={{
                        background: isActive ? 'rgba(201,168,76,0.12)' : 'var(--surface-dark)',
                        color: isActive ? 'var(--gold)' : 'rgb(var(--ov) / 0.4)',
                        border: isActive
                          ? '1px solid rgba(201,168,76,0.3)'
                          : '1px solid rgb(var(--ov) / 0.08)',
                      }}
                    >
                      {p.name}
                    </button>
                  )
                })}
              </div>
            </div>

            {/* Sliders */}
            <div className="flex flex-col gap-4">
              <Slider
                label="Chunk Size"
                value={local.chunkSize}
                onChange={(v) => update('chunkSize', v)}
                min={100}
                max={2000}
                step={50}
              />
              <Slider
                label="Chunk Overlap"
                value={local.chunkOverlap}
                onChange={(v) => update('chunkOverlap', v)}
                min={0}
                max={200}
                step={10}
              />
              <Slider
                label="Top-K Results"
                value={local.topK}
                onChange={(v) => update('topK', v)}
                min={1}
                max={20}
                step={1}
              />
            </div>
          </div>

          {/* ── Jurisdiction ── */}
          <div>
            <SectionHeader>Jurisdiction</SectionHeader>
            <p className="text-[11px] mb-3" style={{ color: 'rgb(var(--ov) / 0.45)' }}>
              Set a default jurisdiction so the AI applies the correct legal hierarchy. Leave on "Auto" to detect from documents.
            </p>
            <div className="flex flex-col gap-3">
              <Field label="Level">
                <StyledSelect
                  value={local.jurisdiction?.level ?? 'auto'}
                  options={[
                    { value: 'auto', label: 'Auto (from documents)' },
                    { value: 'federal', label: 'Federal' },
                    { value: 'state', label: 'State' },
                    { value: 'county', label: 'County' },
                  ]}
                  onChange={(val) => {
                    if (val === 'auto') {
                      setLocal((prev) => ({ ...prev, jurisdiction: undefined }))
                    } else {
                      setLocal((prev) => ({
                        ...prev,
                        jurisdiction: {
                          level: val as JurisdictionLevel,
                          state: val === 'federal' ? undefined : prev.jurisdiction?.state,
                          county: val === 'county' ? prev.jurisdiction?.county : undefined,
                        },
                      }))
                    }
                  }}
                />
              </Field>

              {(local.jurisdiction?.level === 'state' || local.jurisdiction?.level === 'county') && (
                <Field label="State">
                  <StyledSelect
                    value={local.jurisdiction?.state ?? ''}
                    options={[
                      { value: '', label: 'Select state...' },
                      ...US_STATES.map((s) => ({ value: s, label: s })),
                    ]}
                    onChange={(val) => {
                      setLocal((prev) => ({
                        ...prev,
                        jurisdiction: prev.jurisdiction
                          ? { ...prev.jurisdiction, state: val || undefined }
                          : { level: 'state' as JurisdictionLevel, state: val || undefined },
                      }))
                    }}
                  />
                </Field>
              )}

              {local.jurisdiction?.level === 'county' && (
                <Field label="County">
                  <input
                    type="text"
                    value={local.jurisdiction?.county ?? ''}
                    onChange={(e) => {
                      setLocal((prev) => ({
                        ...prev,
                        jurisdiction: prev.jurisdiction
                          ? { ...prev.jurisdiction, county: e.target.value || undefined }
                          : { level: 'county', county: e.target.value || undefined },
                      }))
                    }}
                    placeholder="e.g. Los Angeles County"
                    className="w-full rounded-lg px-3 py-2 text-[12px] outline-none"
                    style={{
                      background: 'var(--surface-dark)',
                      color: 'var(--text)',
                      border: '1px solid rgb(var(--ov) / 0.08)',
                    }}
                  />
                </Field>
              )}
            </div>
          </div>

          {/* ── Re-index Warning ── */}
          {showReindexWarning && (
            <div
              className="rounded-xl px-4 py-3 flex flex-col gap-3"
              style={{ background: 'rgba(201,168,76,0.06)', border: '1px solid rgba(201,168,76,0.2)' }}
            >
              <div className="flex items-start gap-3">
                <svg width="14" height="14" viewBox="0 0 16 16" fill="var(--gold)" className="shrink-0 mt-0.5">
                  <path d="M8 1.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13zM0 8a8 8 0 1 1 16 0A8 8 0 0 1 0 8zm6.5-.25A.75.75 0 0 1 7.25 7h1a.75.75 0 0 1 .75.75v2.75h.25a.75.75 0 0 1 0 1.5h-2a.75.75 0 0 1 0-1.5h.25v-2h-.25a.75.75 0 0 1-.75-.75zM8 6a1 1 0 1 1 0-2 1 1 0 0 1 0 2z" />
                </svg>
                <div>
                  <p className="text-[12px] font-semibold mb-1" style={{ color: 'var(--gold)' }}>
                    Chunk settings changed
                  </p>
                  <p className="text-[11px] leading-relaxed" style={{ color: 'rgb(var(--ov) / 0.45)' }}>
                    Existing documents were indexed with the previous settings. Re-indexing will re-process all documents with the new chunk size and overlap.
                  </p>
                </div>
              </div>
              <div className="flex items-center gap-2 ml-[26px]">
                <button
                  onClick={handleSaveAndReindex}
                  className="rounded-lg px-3 py-1.5 text-[11px] font-semibold transition-colors"
                  style={{ background: 'var(--gold)', color: 'var(--text-on-gold)' }}
                >
                  Re-index Documents
                </button>
                <button
                  onClick={handleSaveSkipReindex}
                  className="rounded-lg px-3 py-1.5 text-[11px] font-medium transition-colors"
                  style={{
                    background: 'transparent',
                    border: '1px solid rgb(var(--ov) / 0.1)',
                    color: 'rgb(var(--ov) / 0.4)',
                  }}
                >
                  Skip, apply to new docs only
                </button>
              </div>
            </div>
          )}

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
              <p className="text-[11px] font-semibold mb-0.5" style={{ color: 'var(--success)' }}>
                100% Private
              </p>
              <p
                className="text-[11px] leading-relaxed"
                style={{ color: 'rgb(var(--ov) / 0.5)' }}
              >
                Your documents and queries never leave this machine. Parsing, search, and AI inference all run locally — no accounts, no API keys, zero network traffic.
              </p>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div
          className="flex items-center justify-between gap-3 px-6 py-4"
          style={{ borderTop: '1px solid rgb(var(--ov) / 0.06)' }}
        >
          {validationError ? (
            <p className="text-[11px]" style={{ color: 'rgba(248,81,73,0.85)' }}>
              {validationError}
            </p>
          ) : buildInfo ? (
            <p className="text-[10px] font-mono" style={{ color: 'rgb(var(--ov) / 0.4)' }}>
              Build: {buildInfo}
            </p>
          ) : <span />}
          <div className="flex items-center gap-3">
          <button
            onClick={handleClose}
            aria-label="Cancel"
            className="rounded-lg px-4 py-2 text-[12px] font-medium transition-colors"
            style={{
              background: 'transparent',
              border: '1px solid rgb(var(--ov) / 0.08)',
              color: 'rgb(var(--ov) / 0.4)',
            }}
            onMouseEnter={(e) => {
              ;(e.currentTarget as HTMLButtonElement).style.color = 'rgb(var(--ov) / 0.7)'
            }}
            onMouseLeave={(e) => {
              ;(e.currentTarget as HTMLButtonElement).style.color = 'rgb(var(--ov) / 0.4)'
            }}
          >
            Cancel
          </button>
          <button
            onClick={handleSave}
            aria-label="Save settings"
            className="rounded-lg px-5 py-2 text-[12px] font-semibold transition-colors"
            style={{ background: 'var(--gold)', color: 'var(--text-on-gold)' }}
            onMouseEnter={(e) => {
              ;(e.currentTarget as HTMLButtonElement).style.background = 'var(--gold-hover)'
            }}
            onMouseLeave={(e) => {
              ;(e.currentTarget as HTMLButtonElement).style.background = 'var(--gold)'
            }}
          >
            Save
          </button>
          </div>
        </div>
      </div>
    </div>
  )
}

import { useEffect, useRef, useState } from 'react'

interface Props {
  upgradeAvailable?: boolean
  onComplete: () => void
}

type Phase = 'prompt' | 'downloading' | 'cleanup'

export default function ModelSetup({ upgradeAvailable, onComplete }: Props): JSX.Element {
  const [percent, setPercent] = useState(0)
  const [downloadedGb, setDownloadedGb] = useState(0)
  const [totalGb, setTotalGb] = useState(5.0)
  const [speed, setSpeed] = useState('')
  const [eta, setEta] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [isDownloading, setIsDownloading] = useState(false)
  // Incrementing this triggers a fresh download attempt via useEffect dependency.
  const [attempt, setAttempt] = useState(0)
  // Phase: upgrade prompt -> downloading -> cleanup (if upgrade)
  const [phase, setPhase] = useState<Phase>(upgradeAvailable ? 'prompt' : 'downloading')
  const [deletingOld, setDeletingOld] = useState(false)

  // High-water marks and throttle state (refs to avoid re-render churn)
  const highWaterPct = useRef(0)
  const highWaterBytes = useRef(0)
  const lastUiUpdate = useRef(0)
  const speedSamples = useRef<{ time: number; bytes: number }[]>([])
  // Track whether this was an upgrade for the cleanup phase
  const wasUpgrade = useRef(!!upgradeAvailable)

  useEffect(() => {
    if (phase !== 'downloading') return

    let isMounted = true
    let unlisten: (() => void) | null = null
    let rafId: number | null = null

    setError(null)
    setIsDownloading(true)
    setPercent(0)
    setDownloadedGb(0)
    setSpeed('')
    setEta('')
    highWaterPct.current = 0
    highWaterBytes.current = 0
    lastUiUpdate.current = 0
    speedSamples.current = []

    // Pending values that get flushed to state at throttled intervals
    let pendingPct = 0
    let pendingGb = 0
    let pendingTotalGb = 5.0
    let pendingSpeed = ''
    let pendingEta = ''

    function flushToUi(): void {
      if (!isMounted) return
      setPercent(pendingPct)
      setDownloadedGb(pendingGb)
      setTotalGb(pendingTotalGb)
      setSpeed(pendingSpeed)
      setEta(pendingEta)
    }

    function formatSpeed(bytesPerSec: number): string {
      if (bytesPerSec < 1e6) return `${(bytesPerSec / 1e3).toFixed(0)} KB/s`
      return `${(bytesPerSec / 1e6).toFixed(1)} MB/s`
    }

    function formatEta(seconds: number): string {
      if (seconds <= 0 || !isFinite(seconds)) return ''
      const m = Math.floor(seconds / 60)
      const s = Math.ceil(seconds % 60)
      if (m === 0) return `${s}s left`
      return `${m}m ${s}s left`
    }

    async function run(): Promise<void> {
      try {
        unlisten = await window.api.onDownloadProgress((progress) => {
          if (!isMounted) return

          if (progress.done) {
            // Immediately flush final state
            pendingPct = 100
            pendingGb = progress.downloadedBytes / 1e9
            pendingSpeed = ''
            pendingEta = ''
            flushToUi()
            if (unlisten) unlisten()
            // If this was an upgrade, show cleanup prompt; otherwise complete
            if (wasUpgrade.current) {
              setPhase('cleanup')
            } else {
              onComplete()
            }
            return
          }

          // Enforce monotonic progress — never let the bar go backwards
          const rawPct = progress.percent
          if (rawPct > highWaterPct.current) highWaterPct.current = rawPct
          pendingPct = highWaterPct.current

          const rawBytes = progress.downloadedBytes
          if (rawBytes > highWaterBytes.current) highWaterBytes.current = rawBytes
          pendingGb = highWaterBytes.current / 1e9

          if (progress.totalBytes > 0) pendingTotalGb = progress.totalBytes / 1e9

          // Speed calculation: rolling 5-second window
          const now = Date.now()
          speedSamples.current.push({ time: now, bytes: highWaterBytes.current })
          // Trim samples older than 5 seconds
          const cutoff = now - 5000
          speedSamples.current = speedSamples.current.filter((s) => s.time >= cutoff)
          if (speedSamples.current.length >= 2) {
            const oldest = speedSamples.current[0]
            const newest = speedSamples.current[speedSamples.current.length - 1]
            const elapsed = (newest.time - oldest.time) / 1000
            const bytesDelta = newest.bytes - oldest.bytes
            if (elapsed > 0.5) {
              const bps = bytesDelta / elapsed
              pendingSpeed = formatSpeed(bps)
              const remaining = pendingTotalGb * 1e9 - highWaterBytes.current
              if (bps > 0 && remaining > 0) {
                pendingEta = formatEta(remaining / bps)
              } else {
                pendingEta = ''
              }
            }
          }

          // Throttle UI updates to ~4 per second (250ms)
          if (now - lastUiUpdate.current >= 250) {
            lastUiUpdate.current = now
            if (rafId !== null) cancelAnimationFrame(rafId)
            rafId = requestAnimationFrame(flushToUi)
          }
        })
        await window.api.downloadModels()
      } catch (err) {
        if (unlisten) { unlisten(); unlisten = null }
        if (!isMounted) return
        setError(err instanceof Error ? err.message : 'Download failed. Check your connection and try again.')
        setIsDownloading(false)
      }
    }

    run()

    return () => {
      isMounted = false
      if (unlisten) unlisten()
      if (rafId !== null) cancelAnimationFrame(rafId)
    }
  }, [attempt, phase])

  const handleDeleteOldModel = async (): Promise<void> => {
    setDeletingOld(true)
    try {
      await window.api.deleteOldModel()
    } catch {
      // Non-critical — old model stays on disk
    }
    setDeletingOld(false)
    onComplete()
  }

  // Gold button style helper
  const goldBtnStyle = {
    background: '#c9a84c',
    color: 'var(--text-on-gold)',
    boxShadow: '0 4px 16px rgba(201,168,76,0.25)',
    transition: 'background 0.15s ease, box-shadow 0.15s ease',
  }
  const goldBtnHover = (e: React.MouseEvent<HTMLButtonElement>): void => {
    const el = e.currentTarget
    el.style.background = '#e8c97e'
    el.style.boxShadow = '0 6px 20px rgba(201,168,76,0.35)'
  }
  const goldBtnLeave = (e: React.MouseEvent<HTMLButtonElement>): void => {
    const el = e.currentTarget
    el.style.background = '#c9a84c'
    el.style.boxShadow = '0 4px 16px rgba(201,168,76,0.25)'
  }

  // ── Upgrade prompt phase ───────────────────────────────────────────────────
  if (phase === 'prompt') {
    return (
      <div
        className="fixed inset-0 z-50 flex flex-col items-center justify-center"
        style={{ background: 'var(--bg)' }}
      >
        <div
          className="w-full max-w-md px-8 flex flex-col items-center"
          style={{ animation: 'scaleIn 0.4s cubic-bezier(0.16, 1, 0.3, 1) both' }}
        >
          {/* Logo mark */}
          <div
            className="w-16 h-16 rounded-2xl flex items-center justify-center mb-8"
            style={{
              background: 'rgba(201,168,76,0.08)',
              border: '1px solid rgba(201,168,76,0.22)',
              boxShadow: '0 8px 32px rgba(201,168,76,0.1)',
            }}
          >
            <svg width="30" height="30" viewBox="0 0 28 28" fill="none">
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
          </div>

          <h1 className="text-[26px] font-bold text-white mb-2.5 text-center tracking-[-0.03em] leading-tight">
            A newer, more capable AI model is available
          </h1>
          <p className="text-[13.5px] text-center mb-4 leading-relaxed" style={{ color: 'rgb(var(--ov) / 0.5)' }}>
            Justice AI now uses Qwen3-8B, which provides significantly improved accuracy,
            better multi-document analysis, and fewer errors compared to the previous model.
          </p>
          <p className="text-[12.5px] text-center mb-8 leading-relaxed" style={{ color: 'rgb(var(--ov) / 0.35)' }}>
            This is a one-time ~5 GB download. Your documents and chat history are preserved.
          </p>

          <div className="flex items-center gap-3">
            <button
              onClick={() => setPhase('downloading')}
              className="rounded-xl px-8 py-3 text-[13.5px] font-semibold"
              style={goldBtnStyle}
              onMouseEnter={goldBtnHover}
              onMouseLeave={goldBtnLeave}
            >
              Upgrade Now
            </button>
            <button
              onClick={onComplete}
              className="rounded-xl px-6 py-3 text-[13.5px] font-medium"
              style={{
                background: 'transparent',
                color: 'rgb(var(--ov) / 0.45)',
                border: '1px solid rgb(var(--ov) / 0.12)',
                transition: 'border-color 0.15s ease',
              }}
              onMouseEnter={(e) => { e.currentTarget.style.borderColor = 'rgb(var(--ov) / 0.25)' }}
              onMouseLeave={(e) => { e.currentTarget.style.borderColor = 'rgb(var(--ov) / 0.12)' }}
            >
              Remind Me Later
            </button>
          </div>
        </div>
      </div>
    )
  }

  // ── Cleanup phase (after upgrade download) ─────────────────────────────────
  if (phase === 'cleanup') {
    return (
      <div
        className="fixed inset-0 z-50 flex flex-col items-center justify-center"
        style={{ background: 'var(--bg)' }}
      >
        <div
          className="w-full max-w-md px-8 flex flex-col items-center"
          style={{ animation: 'scaleIn 0.4s cubic-bezier(0.16, 1, 0.3, 1) both' }}
        >
          {/* Success check */}
          <div
            className="w-16 h-16 rounded-2xl flex items-center justify-center mb-8"
            style={{
              background: 'rgba(63,185,80,0.08)',
              border: '1px solid rgba(63,185,80,0.22)',
            }}
          >
            <svg width="28" height="28" viewBox="0 0 24 24" fill="none">
              <path d="M5 13l4 4L19 7" stroke="#3fb950" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          </div>

          <h1 className="text-[26px] font-bold text-white mb-2.5 text-center tracking-[-0.03em] leading-tight">
            Upgrade complete
          </h1>
          <p className="text-[13.5px] text-center mb-8 leading-relaxed" style={{ color: 'rgb(var(--ov) / 0.5)' }}>
            The previous model (saul.gguf) is no longer needed. Free up 4.5 GB?
          </p>

          <div className="flex items-center gap-3">
            <button
              onClick={handleDeleteOldModel}
              disabled={deletingOld}
              className="rounded-xl px-8 py-3 text-[13.5px] font-semibold"
              style={{
                ...goldBtnStyle,
                opacity: deletingOld ? 0.6 : 1,
                cursor: deletingOld ? 'wait' : 'pointer',
              }}
              onMouseEnter={deletingOld ? undefined : goldBtnHover}
              onMouseLeave={deletingOld ? undefined : goldBtnLeave}
            >
              {deletingOld ? 'Deleting...' : 'Delete'}
            </button>
            <button
              onClick={onComplete}
              disabled={deletingOld}
              className="rounded-xl px-6 py-3 text-[13.5px] font-medium"
              style={{
                background: 'transparent',
                color: 'rgb(var(--ov) / 0.45)',
                border: '1px solid rgb(var(--ov) / 0.12)',
                transition: 'border-color 0.15s ease',
              }}
              onMouseEnter={(e) => { e.currentTarget.style.borderColor = 'rgb(var(--ov) / 0.25)' }}
              onMouseLeave={(e) => { e.currentTarget.style.borderColor = 'rgb(var(--ov) / 0.12)' }}
            >
              Keep
            </button>
          </div>
        </div>
      </div>
    )
  }

  // ── Download phase ─────────────────────────────────────────────────────────
  return (
    <div
      className="fixed inset-0 z-50 flex flex-col items-center justify-center"
      style={{ background: 'var(--bg)' }}
    >
      <div
        className="w-full max-w-md px-8 flex flex-col items-center"
        style={{ animation: 'scaleIn 0.4s cubic-bezier(0.16, 1, 0.3, 1) both' }}
      >
        {/* Logo mark */}
        <div
          className="w-16 h-16 rounded-2xl flex items-center justify-center mb-8"
          style={{
            background: 'rgba(201,168,76,0.08)',
            border: '1px solid rgba(201,168,76,0.22)',
            boxShadow: '0 8px 32px rgba(201,168,76,0.1)',
            animation: 'pulseGlow 3s ease-in-out infinite',
          }}
        >
          <svg width="30" height="30" viewBox="0 0 28 28" fill="none">
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
        </div>

        <h1 className="text-[26px] font-bold text-white mb-2.5 text-center tracking-[-0.03em] leading-tight">
          {wasUpgrade.current ? 'Upgrading AI Model' : 'Setting up Justice AI'}
        </h1>
        <p className="text-[13.5px] text-center mb-10 leading-relaxed" style={{ color: 'rgb(var(--ov) / 0.35)' }}>
          Downloading the Qwen3-8B language model (~5 GB).
          <br />
          <span style={{ color: 'rgb(var(--ov) / 0.45)' }}>This happens once — after this, everything runs locally on your device.</span>
        </p>

        {error ? (
          <div className="w-full flex flex-col items-center gap-5">
            <div
              role="alert"
              className="w-full rounded-xl px-5 py-4"
              style={{
                background: 'rgba(248,81,73,0.06)',
                border: '1px solid rgba(248,81,73,0.2)',
              }}
            >
              <div className="flex items-start gap-3">
                <svg width="14" height="14" viewBox="0 0 16 16" fill="none" className="shrink-0 mt-0.5">
                  <path d="M8 1a7 7 0 1 0 0 14A7 7 0 0 0 8 1zm0 3.5a.75.75 0 0 1 .75.75v3a.75.75 0 0 1-1.5 0v-3A.75.75 0 0 1 8 4.5zm0 6.5a.75.75 0 1 1 0-1.5.75.75 0 0 1 0 1.5z" fill="rgba(248,81,73,0.8)" />
                </svg>
                <p className="text-[13px] leading-relaxed" style={{ color: 'rgba(248,81,73,0.85)' }}>
                  {error}
                </p>
              </div>
            </div>
            <button
              onClick={() => setAttempt((n) => n + 1)}
              aria-label="Retry download"
              className="rounded-xl px-8 py-3 text-[13.5px] font-semibold"
              style={goldBtnStyle}
              onMouseEnter={goldBtnHover}
              onMouseLeave={goldBtnLeave}
            >
              Try Again
            </button>
          </div>
        ) : (
          <div className="w-full flex flex-col gap-4">
            {/* Progress bar */}
            <div
              className="w-full rounded-full overflow-hidden relative"
              style={{ height: '6px', background: 'rgb(var(--ov) / 0.06)' }}
            >
              <div
                className="h-full rounded-full relative overflow-hidden"
                style={{
                  width: `${percent}%`,
                  background: 'linear-gradient(90deg, #b8923e, #c9a84c, #e8c97e)',
                  transition: 'width 0.6s cubic-bezier(0.25, 0.1, 0.25, 1)',
                  willChange: 'width',
                }}
              >
                {/* Shimmer overlay */}
                {isDownloading && percent < 100 && (
                  <div
                    style={{
                      position: 'absolute',
                      inset: 0,
                      background: 'linear-gradient(90deg, transparent 0%, rgb(var(--ov) / 0.35) 50%, transparent 100%)',
                      animation: 'shimmer 1.8s ease-in-out infinite',
                    }}
                  />
                )}
              </div>
            </div>

            {/* Stats */}
            <div className="flex items-center justify-between">
              <span className="text-[13px] font-medium tabular-nums" style={{ color: 'rgb(var(--ov) / 0.45)' }}>
                {isDownloading ? (
                  percent < 100 ? (
                    <>
                      {downloadedGb.toFixed(2)} GB / {totalGb.toFixed(1)} GB
                      {speed && (
                        <span style={{ color: 'rgb(var(--ov) / 0.45)', marginLeft: '8px' }}>
                          {speed}
                        </span>
                      )}
                    </>
                  ) : (
                    'Finalizing...'
                  )
                ) : (
                  'Starting...'
                )}
              </span>
              <span className="flex items-center gap-2">
                {eta && percent < 100 && (
                  <span className="text-[11px] tabular-nums" style={{ color: 'rgb(var(--ov) / 0.45)' }}>
                    {eta}
                  </span>
                )}
                <span className="text-[13px] font-bold tabular-nums" style={{ color: '#c9a84c' }}>
                  {percent}%
                </span>
              </span>
            </div>

            {/* Privacy note */}
            <div
              className="mt-5 rounded-xl px-4 py-3.5 flex items-start gap-3"
              style={{ background: 'rgba(63,185,80,0.04)', border: '1px solid rgba(63,185,80,0.14)' }}
            >
              <svg
                width="14"
                height="14"
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
              <p className="text-[12.5px] leading-relaxed" style={{ color: 'rgb(var(--ov) / 0.5)' }}>
                Qwen3-8B runs entirely on your device. No accounts, no API keys, no data sent to the cloud.
              </p>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}

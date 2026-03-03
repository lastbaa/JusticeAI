import { useEffect, useState } from 'react'

interface Props {
  onComplete: () => void
}

export default function ModelSetup({ onComplete }: Props): JSX.Element {
  const [percent, setPercent] = useState(0)
  const [downloadedGb, setDownloadedGb] = useState(0)
  const [totalGb, setTotalGb] = useState(4.5)
  const [error, setError] = useState<string | null>(null)
  const [isDownloading, setIsDownloading] = useState(false)
  // Incrementing this triggers a fresh download attempt via useEffect dependency.
  const [attempt, setAttempt] = useState(0)

  useEffect(() => {
    let isMounted = true
    let unlisten: (() => void) | null = null

    setError(null)
    setIsDownloading(true)
    setPercent(0)
    setDownloadedGb(0)

    async function run(): Promise<void> {
      try {
        unlisten = await window.api.onDownloadProgress((progress) => {
          if (!isMounted) return  // component unmounted — ignore stale events
          setPercent(progress.percent)
          setDownloadedGb(progress.downloadedBytes / 1e9)
          if (progress.totalBytes > 0) setTotalGb(progress.totalBytes / 1e9)
          if (progress.done) {
            if (unlisten) unlisten()
            onComplete()
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
      if (unlisten) unlisten()  // always unregister progress listener on unmount
    }
  }, [attempt])

  return (
    <div
      className="fixed inset-0 z-50 flex flex-col items-center justify-center"
      style={{ background: '#080808' }}
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
          <svg width="30" height="30" viewBox="0 0 20 20" fill="none">
            <rect x="1" y="3" width="11" height="4" rx="1.25" fill="#c9a84c" transform="rotate(45 6.5 5)" />
            <line x1="10.5" y1="10.5" x2="18.5" y2="18.5" stroke="#c9a84c" strokeWidth="2.5" strokeLinecap="round" />
            <rect x="0.5" y="16.5" width="8.5" height="2.5" rx="0.75" fill="#c9a84c" opacity="0.38" />
          </svg>
        </div>

        <h1 className="text-[26px] font-bold text-white mb-2.5 text-center tracking-[-0.03em] leading-tight">
          Setting up Justice AI
        </h1>
        <p className="text-[13.5px] text-center mb-10 leading-relaxed" style={{ color: 'rgba(255,255,255,0.35)' }}>
          Downloading the Saul legal AI model.
          <br />
          <span style={{ color: 'rgba(255,255,255,0.22)' }}>This only happens once.</span>
        </p>

        {error ? (
          <div className="w-full flex flex-col items-center gap-5">
            <div
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
              className="rounded-xl px-8 py-3 text-[13.5px] font-semibold"
              style={{
                background: '#c9a84c',
                color: '#080808',
                boxShadow: '0 4px 16px rgba(201,168,76,0.25)',
                transition: 'background 0.15s ease, box-shadow 0.15s ease',
              }}
              onMouseEnter={(e) => {
                const el = e.currentTarget as HTMLButtonElement
                el.style.background = '#e8c97e'
                el.style.boxShadow = '0 6px 20px rgba(201,168,76,0.35)'
              }}
              onMouseLeave={(e) => {
                const el = e.currentTarget as HTMLButtonElement
                el.style.background = '#c9a84c'
                el.style.boxShadow = '0 4px 16px rgba(201,168,76,0.25)'
              }}
            >
              Try Again
            </button>
          </div>
        ) : (
          <div className="w-full flex flex-col gap-4">
            {/* Progress bar */}
            <div
              className="w-full rounded-full overflow-hidden relative"
              style={{ height: '6px', background: 'rgba(255,255,255,0.06)' }}
            >
              <div
                className="h-full rounded-full transition-all duration-500 relative overflow-hidden"
                style={{ width: `${percent}%`, background: 'linear-gradient(90deg, #b8923e, #c9a84c, #e8c97e)' }}
              >
                {/* Shimmer overlay */}
                {isDownloading && percent < 100 && (
                  <div
                    style={{
                      position: 'absolute',
                      inset: 0,
                      background: 'linear-gradient(90deg, transparent 0%, rgba(255,255,255,0.35) 50%, transparent 100%)',
                      animation: 'shimmer 1.8s ease-in-out infinite',
                    }}
                  />
                )}
              </div>
            </div>

            {/* Stats */}
            <div className="flex items-center justify-between">
              <span className="text-[13px] font-medium" style={{ color: 'rgba(255,255,255,0.45)' }}>
                {isDownloading ? (
                  percent < 100 ? (
                    <>
                      {downloadedGb.toFixed(2)} GB / {totalGb.toFixed(1)} GB
                    </>
                  ) : (
                    'Finalizing…'
                  )
                ) : (
                  'Starting…'
                )}
              </span>
              <span className="text-[13px] font-bold tabular-nums" style={{ color: '#c9a84c' }}>
                {percent}%
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
              <p className="text-[12.5px] leading-relaxed" style={{ color: 'rgba(255,255,255,0.38)' }}>
                Saul runs entirely on your device. No accounts, no API keys, no data sent to the cloud.
              </p>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}

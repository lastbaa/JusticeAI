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

  async function startDownload(): Promise<void> {
    setError(null)
    setIsDownloading(true)

    let unlisten: (() => void) | null = null
    try {
      unlisten = await window.api.onDownloadProgress((progress) => {
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
      if (unlisten) unlisten()
      setError(err instanceof Error ? err.message : 'Download failed. Check your connection and try again.')
      setIsDownloading(false)
    }
  }

  useEffect(() => {
    startDownload()
  }, [])

  return (
    <div
      className="fixed inset-0 z-50 flex flex-col items-center justify-center"
      style={{ background: '#080808' }}
    >
      <div className="w-full max-w-md px-8 flex flex-col items-center">
        {/* Logo mark */}
        <div
          className="w-14 h-14 rounded-2xl flex items-center justify-center mb-8"
          style={{ background: 'rgba(201,168,76,0.08)', border: '1px solid rgba(201,168,76,0.2)' }}
        >
          <svg width="26" height="26" viewBox="0 0 24 24" fill="none">
            <path
              d="M12 2L3 7v5c0 5.25 3.75 10.15 9 11.35C17.25 22.15 21 17.25 21 12V7L12 2z"
              stroke="#c9a84c"
              strokeWidth="1.8"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
            <path
              d="M9 12l2 2 4-4"
              stroke="#c9a84c"
              strokeWidth="1.8"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </div>

        <h1 className="text-xl font-bold text-white mb-2 text-center">
          Setting up Justice AI
        </h1>
        <p className="text-sm text-center mb-10" style={{ color: 'rgba(255,255,255,0.38)' }}>
          Downloading the Saul legal AI model. This only happens once.
        </p>

        {error ? (
          <div className="w-full flex flex-col items-center gap-5">
            <div
              className="w-full rounded-xl px-5 py-4 text-sm"
              style={{
                background: 'rgba(248,81,73,0.08)',
                border: '1px solid rgba(248,81,73,0.25)',
                color: '#f85149',
              }}
            >
              {error}
            </div>
            <button
              onClick={() => startDownload()}
              className="rounded-xl px-8 py-3 text-sm font-semibold transition-colors"
              style={{ background: '#c9a84c', color: '#080808' }}
              onMouseEnter={(e) => {
                ;(e.currentTarget as HTMLButtonElement).style.background = '#e8c97e'
              }}
              onMouseLeave={(e) => {
                ;(e.currentTarget as HTMLButtonElement).style.background = '#c9a84c'
              }}
            >
              Retry Download
            </button>
          </div>
        ) : (
          <div className="w-full flex flex-col gap-4">
            {/* Progress bar */}
            <div
              className="w-full rounded-full overflow-hidden"
              style={{ height: '6px', background: 'rgba(255,255,255,0.06)' }}
            >
              <div
                className="h-full rounded-full transition-all duration-300"
                style={{ width: `${percent}%`, background: '#c9a84c' }}
              />
            </div>

            {/* Stats */}
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium" style={{ color: 'rgba(255,255,255,0.5)' }}>
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
              <span className="text-sm font-semibold" style={{ color: '#c9a84c' }}>
                {percent}%
              </span>
            </div>

            {/* Privacy note */}
            <div
              className="mt-4 rounded-xl px-4 py-3 flex items-start gap-3"
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
              <p className="text-xs leading-relaxed" style={{ color: 'rgba(255,255,255,0.35)' }}>
                Saul runs entirely on your device. No accounts, no API keys, no data sent to the cloud.
              </p>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}

import Navbar from '../components/Navbar'
import Footer from '../components/Footer'

const releases = [
  {
    version: 'v1.2.0',
    date: 'March 2026',
    tag: 'Latest',
    changes: [
      { type: 'new', text: 'Fully local AI — Saul-7B-Instruct GGUF model runs on-device via llama.cpp. No API keys, no internet required after the one-time model download.' },
      { type: 'new', text: 'Multi-turn conversation — the AI now remembers prior exchanges in a session, allowing natural follow-up questions.' },
      { type: 'new', text: 'Session search — filter your session history by name instantly in the sidebar.' },
      { type: 'new', text: 'Copy to clipboard — one-click copy on all messages, code blocks, and source excerpts.' },
      { type: 'new', text: 'Session rename — click the pencil icon on any session to give it a custom name.' },
      { type: 'new', text: 'Export chat — save any conversation as a Markdown file for your records.' },
      { type: 'new', text: 'Export citations — save the retrieved sources for a session as a CSV file.' },
      { type: 'improved', text: 'Confirmation dialogs before destructive actions (clear files, clear sessions, delete session).' },
      { type: 'improved', text: 'Toast notifications for success and error feedback across all operations.' },
      { type: 'improved', text: 'Migrated from Electron to Tauri 2 — smaller bundle, lower memory use, native macOS feel.' },
    ],
  },
  {
    version: 'v1.1.0',
    date: 'February 2026',
    tag: null,
    changes: [
      { type: 'new', text: 'Document viewer — open PDFs and DOCX files directly inside the app with page-level citation highlighting.' },
      { type: 'new', text: 'Folder loading — drag in an entire folder and Justice AI will find and index all supported files automatically.' },
      { type: 'new', text: 'Chat sessions — conversations are now saved and grouped by date in the sidebar.' },
      { type: 'improved', text: 'Embedding model upgraded to AllMiniLML6V2 via fastembed — faster indexing, better retrieval.' },
      { type: 'improved', text: 'PDF file server — PDFs now render reliably in the in-app viewer on all macOS versions.' },
      { type: 'fixed', text: 'DOCX parsing no longer drops tables or multi-column layouts.' },
    ],
  },
  {
    version: 'v1.0.0',
    date: 'January 2026',
    tag: 'Initial Release',
    changes: [
      { type: 'new', text: 'Initial public release of Justice AI for macOS.' },
      { type: 'new', text: 'Load PDF and DOCX files, index them locally with vector embeddings, and ask natural-language questions.' },
      { type: 'new', text: 'Streaming answers with inline source citations (filename, page, excerpt).' },
      { type: 'new', text: 'Settings panel — configure embedding model, chunk size, and retrieval count.' },
      { type: 'new', text: 'Apple Silicon native build with Metal GPU acceleration.' },
    ],
  },
]

const typeLabel: Record<string, { label: string; color: string; bg: string }> = {
  new: { label: 'New', color: '#86efac', bg: 'rgba(134,239,172,0.08)' },
  improved: { label: 'Improved', color: '#93c5fd', bg: 'rgba(147,197,253,0.08)' },
  fixed: { label: 'Fixed', color: '#fca5a5', bg: 'rgba(252,165,165,0.08)' },
}

export default function Changelog() {
  return (
    <main className="min-h-screen" style={{ background: '#080808' }}>
      <Navbar />

      <section className="pt-36 pb-24 px-6">
        <div className="max-w-2xl mx-auto">

          {/* Header */}
          <div className="mb-16">
            <span
              className="text-xs font-medium tracking-[0.2em] uppercase mb-4 block"
              style={{ color: 'rgba(201,168,76,0.55)' }}
            >
              Release History
            </span>
            <h1
              className="text-4xl sm:text-5xl font-bold text-white mb-4"
              style={{ letterSpacing: '-0.03em' }}
            >
              Changelog
            </h1>
            <p className="text-base" style={{ color: 'rgba(255,255,255,0.4)' }}>
              What&apos;s new, improved, and fixed in Justice AI.
            </p>
          </div>

          {/* Releases */}
          <div className="space-y-16">
            {releases.map((release) => (
              <div key={release.version}>
                {/* Version header */}
                <div className="flex items-center gap-3 mb-6">
                  <span
                    className="text-xl font-bold"
                    style={{ color: '#fff', letterSpacing: '-0.02em' }}
                  >
                    {release.version}
                  </span>
                  {release.tag && (
                    <span
                      className="text-xs font-medium px-2.5 py-0.5 rounded-full"
                      style={{
                        background: 'rgba(201,168,76,0.1)',
                        border: '1px solid rgba(201,168,76,0.25)',
                        color: '#c9a84c',
                      }}
                    >
                      {release.tag}
                    </span>
                  )}
                  <span
                    className="text-sm ml-auto"
                    style={{ color: 'rgba(255,255,255,0.3)' }}
                  >
                    {release.date}
                  </span>
                </div>

                {/* Changes */}
                <div
                  className="rounded-xl px-6 py-4 space-y-3"
                  style={{
                    background: '#0d0d0d',
                    border: '1px solid rgba(255,255,255,0.07)',
                  }}
                >
                  {release.changes.map((change, i) => {
                    const meta = typeLabel[change.type]
                    return (
                      <div key={i} className="flex items-start gap-3 py-1">
                        <span
                          className="shrink-0 text-xs font-semibold px-2 py-0.5 rounded mt-0.5"
                          style={{
                            background: meta.bg,
                            color: meta.color,
                            minWidth: '60px',
                            textAlign: 'center',
                          }}
                        >
                          {meta.label}
                        </span>
                        <p
                          className="text-sm leading-relaxed"
                          style={{ color: 'rgba(255,255,255,0.6)' }}
                        >
                          {change.text}
                        </p>
                      </div>
                    )
                  })}
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      <Footer />
    </main>
  )
}

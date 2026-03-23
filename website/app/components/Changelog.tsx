'use client'

const releases = [
  {
    version: '1.4.0',
    date: 'March 2026',
    items: [
      'Case deletion confirmation dialog with keep/delete options',
      'Windows and Linux cross-platform compatibility',
      'DEB package support for Linux',
      'Legal jurisdiction detection and UI selector',
      'Citation confidence scores and key facts extraction',
    ],
  },
  {
    version: '1.3.0',
    date: 'March 2026',
    items: [
      'Case folders with scoped retrieval',
      'PDF highlighting in document viewer',
      'Key Sources feature',
      'LLM response quality improvements',
      'Smooth download progress bar with speed and ETA',
    ],
  },
  {
    version: '1.2.0',
    date: 'February 2026',
    items: [
      'Streaming token output with pipeline status indicators',
      'Evaluation harness with 77 test cases across 8 fixtures',
      'Multi-format ingestion (DOCX, CSV, HTML, EML, XLSX) and OCR',
      'Pluggable retrieval backend trait',
      'Paragraph-aware chunking with BGE embeddings',
    ],
  },
  {
    version: '1.1.0',
    date: 'January 2026',
    items: [
      'MMR diversity reranking for retrieval results',
      'Abbreviation-aware chunking',
      'Fixed garbled LLM output and PDF encoding issues',
      'Metal GPU offload and context reduction for stability',
    ],
  },
  {
    version: '1.0.0',
    date: 'December 2025',
    items: [
      'Tauri 2 desktop app with Rust backend',
      'Fully local LLM inference via Saul-7B-Instruct',
      'Local embeddings via fastembed (BGE-small-en-v1.5)',
      'Hybrid BM25 + cosine similarity retrieval with RRF',
      'Zero-config setup with automatic model download',
    ],
  },
]

export default function Changelog() {
  return (
    <section className="py-20 px-6" style={{ background: '#080808' }}>
      <div className="max-w-4xl mx-auto">
        <h2 className="text-3xl md:text-4xl font-bold text-white text-center mb-4">
          Changelog
        </h2>
        <p className="text-center text-gray-400 mb-12">
          What&apos;s new in Justice AI
        </p>

        <div className="relative">
          {/* Timeline line */}
          <div className="absolute left-4 md:left-6 top-0 bottom-0 w-px bg-gray-800" />

          <div className="space-y-10">
            {releases.map((release) => (
              <div key={release.version} className="relative pl-12 md:pl-16">
                {/* Timeline dot */}
                <div
                  className="absolute left-2.5 md:left-4.5 top-1.5 w-3 h-3 rounded-full border-2"
                  style={{ borderColor: '#c9a84c', background: '#080808' }}
                />

                <div className="flex items-baseline gap-3 mb-3">
                  <span
                    className="text-sm font-mono font-semibold px-2 py-0.5 rounded"
                    style={{ background: 'rgba(201,168,76,0.15)', color: '#c9a84c' }}
                  >
                    v{release.version}
                  </span>
                  <span className="text-sm text-gray-500">{release.date}</span>
                </div>

                <ul className="space-y-1.5">
                  {release.items.map((item, i) => (
                    <li key={i} className="text-sm text-gray-300 flex items-start gap-2">
                      <span className="mt-1.5 w-1 h-1 rounded-full bg-gray-600 flex-shrink-0" />
                      {item}
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  )
}

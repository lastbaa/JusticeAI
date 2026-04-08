'use client'

import { useState } from 'react'
import { Reveal } from './Reveal'
import { WordReveal } from './WordReveal'

const faqs = [
  {
    q: 'Does Justice AI work completely offline?',
    a: 'Yes — after the one-time model download (~4.5 GB on first launch), everything runs 100% offline. Document parsing, vector search, and AI answer generation all happen on your machine. No internet connection required.',
  },
  {
    q: 'What file types are supported?',
    a: 'Justice AI supports 17 file types: PDF, DOCX, TXT, Markdown, CSV, XLSX, HTML, MHTML, XML, EML (email), and images (PNG, JPG, JPEG, TIFF) via built-in OCR. You can load individual files or entire folders. The app will automatically find and index all supported files in a folder.',
  },
  {
    q: 'How big is the model download?',
    a: 'The Saul-7B legal AI model is approximately 4.5 GB and downloads once on first launch. The embedding model (used for search) is an additional ~22 MB and downloads automatically. After that, no further downloads are needed.',
  },
  {
    q: 'Do my documents ever leave my device?',
    a: 'Never. Your documents are parsed, embedded, and searched entirely on-device. Nothing is uploaded to any server. The only network activity is the initial model download on first launch.',
  },
  {
    q: 'Is this legal advice?',
    a: "No. Justice AI is a research tool for attorneys and legal professionals. It surfaces and cites passages from your documents but does not provide legal advice. All outputs should be reviewed by a licensed attorney.",
  },
  {
    q: 'What hardware does Justice AI require?',
    a: 'macOS 12+ (Apple Silicon native, Intel via Rosetta 2), Windows 10+ (x64, Vulkan GPU recommended), or Linux (Ubuntu 22.04+ / Fedora 38+, x64 AppImage). 8 GB RAM minimum, 16 GB recommended. Apple Silicon Macs get the best performance via Metal GPU acceleration.',
  },
]

function FAQItem({ item, isLast }: { item: typeof faqs[0]; isLast: boolean }): JSX.Element {
  const [open, setOpen] = useState(false)

  return (
    <div
      className={`py-5 ${!isLast ? 'border-b' : ''}`}
      style={{ borderColor: 'rgba(255,255,255,0.06)' }}
    >
      <button
        onClick={() => setOpen((v) => !v)}
        className="flex w-full items-start justify-between gap-6 text-left group"
      >
        <span
          className="text-sm font-medium leading-snug transition-colors"
          style={{ color: open ? '#fff' : 'rgba(255,255,255,0.65)' }}
        >
          {item.q}
        </span>
        <span
          className="shrink-0 w-5 h-5 rounded-full flex items-center justify-center mt-px transition-all"
          style={{
            background: open ? 'rgba(201,168,76,0.12)' : 'rgba(255,255,255,0.05)',
            border: `1px solid ${open ? 'rgba(201,168,76,0.3)' : 'rgba(255,255,255,0.1)'}`,
            color: open ? '#c9a84c' : 'rgba(255,255,255,0.5)',
            transform: open ? 'rotate(45deg)' : 'rotate(0deg)',
          }}
        >
          <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor">
            <path d="M7.75 2a.75.75 0 0 1 .75.75V7h4.25a.75.75 0 0 1 0 1.5H8.5v4.25a.75.75 0 0 1-1.5 0V8.5H2.75a.75.75 0 0 1 0-1.5H7V2.75A.75.75 0 0 1 7.75 2z" />
          </svg>
        </span>
      </button>
      {open && (
        <p
          className="mt-3 text-sm leading-relaxed"
          style={{ color: 'rgba(255,255,255,0.45)', animation: 'fadeUp 0.18s ease both' }}
        >
          {item.a}
        </p>
      )}
    </div>
  )
}

export default function FAQ(): JSX.Element {
  return (
    <section className="py-24 px-6" style={{ background: '#080808' }}>
      <div className="max-w-6xl mx-auto">
        <div className="border-t mb-24" style={{ borderColor: 'rgba(255,255,255,0.05)' }} />
      </div>

      <div className="max-w-2xl mx-auto">
        <Reveal className="flex justify-center mb-6">
          <span className="text-xs font-medium tracking-[0.2em] uppercase" style={{ color: 'rgba(201,168,76,0.55)' }}>
            FAQ
          </span>
        </Reveal>

        <div className="text-center mb-12">
          <WordReveal
            text="Common Questions"
            as="h2"
            stagger={85}
            className="text-3xl sm:text-4xl font-bold text-white"
            style={{ letterSpacing: '-0.02em' }}
          />
        </div>

        <Reveal>
          <div
            className="rounded-2xl px-8 py-2"
            style={{ background: '#0d0d0d', border: '1px solid rgba(255,255,255,0.07)' }}
          >
            {faqs.map((item, i) => (
              <FAQItem key={i} item={item} isLast={i === faqs.length - 1} />
            ))}
          </div>
        </Reveal>
      </div>
    </section>
  )
}

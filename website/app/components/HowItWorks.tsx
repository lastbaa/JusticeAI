'use client'

import { Reveal } from './Reveal'
import { WordReveal } from './WordReveal'

const steps = [
  {
    number: '01',
    title: 'Load Your Documents',
    body: 'Drag in a folder of case files, contracts, or briefs. Justice AI accepts PDF and Word documents. All parsing happens locally — nothing is transmitted.',
    icon: (
      <svg width="20" height="20" viewBox="0 0 28 28" fill="none">
        <path d="M3 8a2 2 0 012-2h5l2 2h11a2 2 0 012 2v10a2 2 0 01-2 2H5a2 2 0 01-2-2V8z"
          stroke="rgba(201,168,76,0.7)" strokeWidth="1.6" fill="none" strokeLinejoin="round" />
        <path d="M14 21v-7m-3 3l3-3 3 3" stroke="rgba(201,168,76,0.7)" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    ),
    variant: 'left' as const,
  },
  {
    number: '02',
    title: 'Ask Your Question',
    body: "Type any legal question in plain English. 'What does clause 7 say about termination?' or 'Find all references to the indemnification period.'",
    icon: (
      <svg width="20" height="20" viewBox="0 0 28 28" fill="none">
        <path d="M5 6a2 2 0 012-2h14a2 2 0 012 2v11a2 2 0 01-2 2H9l-4 4V6z"
          stroke="rgba(201,168,76,0.7)" strokeWidth="1.6" fill="none" strokeLinejoin="round" />
        <line x1="10" y1="10" x2="18" y2="10" stroke="rgba(201,168,76,0.7)" strokeWidth="1.4" strokeLinecap="round" />
        <line x1="10" y1="14" x2="15" y2="14" stroke="rgba(201,168,76,0.7)" strokeWidth="1.4" strokeLinecap="round" />
      </svg>
    ),
    variant: 'up' as const,
  },
  {
    number: '03',
    title: 'Get Cited Answers',
    body: 'Receive a direct answer with source citations — filename, page number, and exact quoted text. Verify every answer instantly.',
    icon: (
      <svg width="20" height="20" viewBox="0 0 28 28" fill="none">
        <path d="M7 4h10l4 4v16H7V4z" stroke="rgba(201,168,76,0.7)" strokeWidth="1.6" fill="none" strokeLinejoin="round" />
        <path d="M17 4v4h4" stroke="rgba(201,168,76,0.7)" strokeWidth="1.6" strokeLinejoin="round" />
        <path d="M11 14.5l2.5 2.5 5-5" stroke="rgba(201,168,76,0.7)" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    ),
    variant: 'right' as const,
  },
]

export default function HowItWorks() {
  return (
    <section id="how-it-works" className="py-32 px-6" style={{ background: '#080808' }}>
      <div className="max-w-6xl mx-auto">
        <div className="border-t mb-32" style={{ borderColor: 'rgba(255,255,255,0.05)' }} />
      </div>

      <div className="max-w-6xl mx-auto">

        {/* Section label */}
        <Reveal className="flex justify-center mb-6">
          <span className="text-xs font-medium tracking-[0.2em] uppercase" style={{ color: 'rgba(201,168,76,0.55)' }}>04 — The Process</span>
        </Reveal>

        {/* Animated heading */}
        <div className="text-center mb-5">
          <WordReveal
            text="How It Works"
            as="h2"
            stagger={100}
            className="text-3xl sm:text-4xl font-bold text-white"
            style={{ letterSpacing: '-0.02em' }}
          />
        </div>
        <Reveal className="text-center mb-24">
          <p className="text-lg max-w-xl mx-auto" style={{ color: 'rgba(255,255,255,0.45)' }}>
            Three steps from document to verified, cited answer backed by your own files.
          </p>
        </Reveal>

        <div className="relative">
          <div
            className="hidden md:block absolute top-[2.6rem] left-[calc(16.67%+2rem)] right-[calc(16.67%+2rem)] h-px"
            style={{ background: 'rgba(201,168,76,0.12)' }}
          />

          <div className="grid grid-cols-1 md:grid-cols-3 gap-16 md:gap-8">
            {steps.map((step, index) => (
              <Reveal key={step.number} variant={step.variant} delay={index * 130}>
                <div className="relative flex flex-col items-center text-center">
                  <div
                    className="relative z-10 w-[3.25rem] h-[3.25rem] rounded-full flex items-center justify-center mb-8"
                    style={{ border: '1px solid rgba(201,168,76,0.22)', background: '#0f0f0f' }}
                  >
                    <span className="text-xs font-bold tracking-widest" style={{ color: 'rgba(201,168,76,0.7)' }}>
                      {step.number}
                    </span>
                  </div>

                  <div className="mb-5 w-8 h-8 flex items-center justify-center rounded-lg" style={{ background: 'rgba(201,168,76,0.06)', border: '1px solid rgba(201,168,76,0.16)' }}>
                    {step.icon}
                  </div>

                  <h3 className="text-sm font-semibold text-white mb-3 tracking-tight">{step.title}</h3>
                  <p className="text-sm leading-relaxed max-w-xs mx-auto" style={{ color: 'rgba(255,255,255,0.45)' }}>
                    {step.body}
                  </p>

                  {index < steps.length - 1 && (
                    <div className="md:hidden mt-12 w-px h-10" style={{ background: 'rgba(255,255,255,0.07)' }} />
                  )}
                </div>
              </Reveal>
            ))}
          </div>
        </div>

        <Reveal variant="up" delay={420} className="mt-24">
          <div
            className="card-lift rounded-xl p-7 flex flex-col sm:flex-row items-start sm:items-center gap-4"
            style={{ background: '#0f0f0f', border: '1px solid rgba(63,185,80,0.12)', boxShadow: '0 0 30px rgba(63,185,80,0.04)' }}
          >
            <div className="flex-shrink-0 w-7 h-7 flex items-center justify-center rounded-lg" style={{ background: 'rgba(63,185,80,0.08)', border: '1px solid rgba(63,185,80,0.2)' }}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
                <circle cx="12" cy="12" r="10" stroke="rgba(63,185,80,0.7)" strokeWidth="1.6" />
                <path d="M12 8v4m0 4h.01" stroke="rgba(63,185,80,0.7)" strokeWidth="1.8" strokeLinecap="round" />
              </svg>
            </div>
            <p className="text-sm leading-relaxed" style={{ color: 'rgba(255,255,255,0.45)' }}>
              <span className="text-white font-medium">Everything runs on your machine — nothing is ever sent anywhere.</span>{' '}
              Document parsing, vector search, and AI answer generation all happen locally using the Saul-7B legal model. No accounts, no API keys, no network required after the one-time model download.
            </p>
          </div>
        </Reveal>
      </div>
    </section>
  )
}

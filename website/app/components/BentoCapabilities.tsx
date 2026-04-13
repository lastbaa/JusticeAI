'use client'

import { Shield, Zap, BookOpenCheck, Cpu, Layers, FolderOpen, FileSearch, SlidersHorizontal } from 'lucide-react'
import { GlowingEffect } from '@/components/ui/glowing-effect'
import { cn } from '@/lib/utils'
import { Reveal } from './Reveal'
import { WordReveal } from './WordReveal'

interface BentoCardProps {
  className: string
  icon: React.ReactNode
  name: string
  description: string
  cta: string
  background?: React.ReactNode
}

function BentoCard({ className, icon, name, description, cta, background }: BentoCardProps) {
  return (
    <div
      className={cn('group relative overflow-hidden rounded-2xl', className)}
      style={{
        background: '#0d0d0d',
        border: '1px solid rgba(255,255,255,0.07)',
      }}
    >
      <GlowingEffect spread={50} glow proximity={80} inactiveZone={0.01} borderWidth={1.5} />

      {/* Decorative background — clipped by overflow-hidden */}
      {background && (
        <div className="pointer-events-none select-none absolute inset-0">
          {background}
        </div>
      )}

      {/* Content slides up on hover to make room for the CTA */}
      <div className="relative z-10 flex flex-col justify-end h-full p-6 md:p-7 transition-transform duration-300 ease-out group-hover:-translate-y-10">
        <div
          className="w-10 h-10 rounded-xl flex items-center justify-center mb-4 shrink-0"
          style={{ background: 'rgba(201,168,76,0.1)', border: '1px solid rgba(201,168,76,0.22)' }}
        >
          {icon}
        </div>
        <h3
          className="text-[17px] md:text-[19px] font-semibold text-white mb-2 leading-snug"
          style={{ letterSpacing: '-0.025em' }}
        >
          {name}
        </h3>
        <p className="text-[13px] md:text-sm leading-relaxed" style={{ color: 'rgba(255,255,255,0.4)' }}>
          {description}
        </p>
      </div>

      {/* CTA slides up from below the card edge on hover */}
      <div className="absolute bottom-0 left-0 right-0 z-20 px-6 md:px-7 pb-5 translate-y-8 opacity-0 transition-all duration-300 ease-out group-hover:translate-y-0 group-hover:opacity-100">
        <span
          className="inline-flex items-center gap-1.5 text-[12px] font-semibold"
          style={{ color: '#c9a84c' }}
        >
          {cta}
          <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
            <path d="M6.22 3.22a.75.75 0 0 1 1.06 0l4.25 4.25a.75.75 0 0 1 0 1.06l-4.25 4.25a.75.75 0 0 1-1.06-1.06L9.94 8 6.22 4.28a.75.75 0 0 1 0-1.06z" />
          </svg>
        </span>
      </div>

      {/* Subtle hover tint */}
      <div className="pointer-events-none absolute inset-0 transition-all duration-300 group-hover:bg-white/[.012]" />
    </div>
  )
}

export default function BentoCapabilities() {
  return (
    <section className="py-32 px-6" style={{ background: '#080808' }}>
      <div className="max-w-6xl mx-auto">

        <Reveal className="flex justify-center mb-6">
          <span className="text-xs font-medium tracking-[0.2em] uppercase" style={{ color: 'rgba(201,168,76,0.55)' }}>
            02 — Capabilities
          </span>
        </Reveal>

        <div className="text-center mb-5">
          <WordReveal
            text="Purpose-Built for Legal Research"
            as="h2"
            stagger={75}
            className="text-3xl sm:text-4xl font-bold text-white"
            style={{ letterSpacing: '-0.02em' }}
          />
        </div>

        <Reveal className="text-center mb-16">
          <p className="text-lg max-w-xl mx-auto" style={{ color: 'rgba(255,255,255,0.4)' }}>
            Every feature was designed around the specific constraints of legal practice — not ported from a generic AI tool.
          </p>
        </Reveal>

        <Reveal variant="scale">
          {/* Grid: 12 cols, auto-rows of 20rem. Cards use row-span for height. */}
          <div
            className="grid grid-cols-1 md:grid-cols-12 gap-4"
            style={{ gridAutoRows: '20rem' }}
          >
            {/* ── Privilege (large, 5 cols × 2 rows) ── */}
            <BentoCard
              className="md:col-span-5 md:row-span-2"
              icon={<Shield className="h-5 w-5" style={{ color: '#c9a84c' }} />}
              name="Attorney-Client Privilege, Intact"
              description="The moment files touch a cloud AI, privilege may be at risk. Justice AI never makes that request — every document stays on your machine, in every query, forever."
              cta="Why this matters"
              background={
                <svg className="absolute -bottom-4 -right-4 w-52 h-52 opacity-[0.055]" viewBox="0 0 100 100" fill="none">
                  <path d="M50 8 L88 22 L88 54 C88 74 70 89 50 95 C30 89 12 74 12 54 L12 22 Z" stroke="#c9a84c" strokeWidth="2.5" fill="rgba(201,168,76,0.08)" />
                  <path d="M35 50 L45 60 L65 38" stroke="#c9a84c" strokeWidth="3.5" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
              }
            />

            {/* ── Speed (wide, 7 cols × 1 row) ── */}
            <BentoCard
              className="md:col-span-7"
              icon={<Zap className="h-5 w-5" style={{ color: '#c9a84c' }} />}
              name="Hours of Research in Seconds"
              description="A 500-page deposition that takes half a day to search manually returns a cited excerpt in seconds — and answers stream word-by-word so you see results immediately, not after a minute of waiting."
              cta="See how it works"
              background={
                <div className="absolute top-5 right-8 flex items-baseline gap-1 select-none pointer-events-none" style={{ opacity: 0.055 }}>
                  <span className="font-bold tabular-nums" style={{ color: '#c9a84c', fontSize: 80, letterSpacing: '-0.05em', lineHeight: 1 }}>10</span>
                  <span className="font-semibold" style={{ color: '#c9a84c', fontSize: 28, lineHeight: 1 }}>sec</span>
                </div>
              }
            />

            {/* ── Citations (mid, 4 cols × 1 row) ── */}
            <BentoCard
              className="md:col-span-4"
              icon={<BookOpenCheck className="h-5 w-5" style={{ color: '#c9a84c' }} />}
              name="Cited, Never Assumed"
              description="Every source shows its exact filename, page number, and a relevance score — Strong, Good, or Weak — so you know at a glance how confident the match is."
              cta="Explore citation format"
              background={
                <div className="absolute top-5 right-5 flex flex-col gap-2 opacity-[0.07] w-24 pointer-events-none">
                  {[100, 72, 88, 55, 92].map((w, i) => (
                    <div
                      key={i}
                      className="h-2 rounded-full"
                      style={{ width: `${w}%`, background: i === 2 ? '#c9a84c' : 'rgba(255,255,255,0.5)' }}
                    />
                  ))}
                </div>
              }
            />

            {/* ── On-Device AI (small, 3 cols × 1 row) ── */}
            <BentoCard
              className="md:col-span-3"
              icon={<Cpu className="h-5 w-5" style={{ color: '#c9a84c' }} />}
              name="On-Device AI"
              description="Qwen3-8B and all embeddings run on your hardware. No APIs, no subscriptions."
              cta="Technical details"
              background={
                <svg className="absolute bottom-3 right-3 w-20 h-20 opacity-[0.07]" viewBox="0 0 80 80" fill="none">
                  <rect x="18" y="18" width="44" height="44" rx="7" stroke="#c9a84c" strokeWidth="2.5" />
                  <rect x="27" y="27" width="26" height="26" rx="4" fill="rgba(201,168,76,0.25)" />
                  {[18, 32, 46].map((y) => (
                    <g key={y}>
                      <line x1="18" y1={y} x2="9" y2={y} stroke="#c9a84c" strokeWidth="2" />
                      <line x1="62" y1={y} x2="71" y2={y} stroke="#c9a84c" strokeWidth="2" />
                    </g>
                  ))}
                  {[18, 32, 46].map((x) => (
                    <g key={x}>
                      <line x1={x} y1="18" x2={x} y2="9" stroke="#c9a84c" strokeWidth="2" />
                      <line x1={x} y1="62" x2={x} y2="71" stroke="#c9a84c" strokeWidth="2" />
                    </g>
                  ))}
                </svg>
              }
            />

            {/* ── Case Management (5 cols × 1 row) ── */}
            <BentoCard
              className="md:col-span-5"
              icon={<FolderOpen className="h-5 w-5" style={{ color: '#c9a84c' }} />}
              name="Case Management"
              description="Organize documents, chat sessions, and notes by legal matter. Set case context that automatically injects into every query. Cross-session summaries carry knowledge across conversations within the same case."
              cta="Organize your matters"
              background={
                <div className="absolute top-5 right-5 flex flex-col gap-1.5 opacity-[0.06] pointer-events-none">
                  {['Smith v. Jones', 'Contract Review', 'Due Diligence'].map((label, i) => (
                    <div key={i} className="flex items-center gap-2">
                      <div className="w-3 h-3 rounded" style={{ background: '#c9a84c' }} />
                      <span className="text-xs font-medium" style={{ color: '#c9a84c' }}>{label}</span>
                    </div>
                  ))}
                </div>
              }
            />

            {/* ── Document Intelligence (7 cols × 1 row) ── */}
            <BentoCard
              className="md:col-span-7"
              icon={<FileSearch className="h-5 w-5" style={{ color: '#c9a84c' }} />}
              name="Auto-Generated Fact Sheets"
              description="Every document is automatically analyzed on load — extracting parties, dates, dollar amounts, and key clauses. Tag documents as Client Documents, Legal Authority, Evidence, or Reference to shape how the AI weighs each source."
              cta="See document intelligence"
              background={
                <div className="absolute top-5 right-8 flex flex-col gap-2 opacity-[0.06] pointer-events-none">
                  {['$125,000', 'Jan 15, 2024', 'Sec. 4.2'].map((val, i) => (
                    <div key={i} className="flex items-center gap-2 text-xs font-mono" style={{ color: '#c9a84c' }}>
                      <div className="w-1.5 h-1.5 rounded-full" style={{ background: '#c9a84c' }} />
                      {val}
                    </div>
                  ))}
                </div>
              }
            />

            {/* ── Multi-document (full width, 12 cols × 1 row) ── */}
            <BentoCard
              className="md:col-span-5"
              icon={<Layers className="h-5 w-5" style={{ color: '#c9a84c' }} />}
              name="Search Your Entire Case at Once"
              description="Load a whole matter folder — contracts, depositions, briefs, correspondence — and ask one question. Justice AI retrieves the most relevant passages from every document simultaneously, ranked by semantic relevance, with full page citations."
              cta="Start a search"
              background={
                <div className="absolute top-1/2 -translate-y-1/2 right-8 flex items-end gap-2 pointer-events-none" style={{ opacity: 0.055 }}>
                  {[72, 88, 64, 80].map((h, i) => (
                    <div
                      key={i}
                      className="w-8 rounded-lg shrink-0"
                      style={{
                        height: h,
                        background: 'rgba(201,168,76,0.9)',
                        transform: `translateY(${i % 2 === 0 ? -6 : 6}px)`,
                      }}
                    />
                  ))}
                </div>
              }
            />

            {/* ── Inference Modes (7 cols × 1 row) ── */}
            <BentoCard
              className="md:col-span-7"
              icon={<SlidersHorizontal className="h-5 w-5" style={{ color: '#c9a84c' }} />}
              name="Three Inference Modes"
              description="Brief for quick lookups, Standard for balanced analysis, and Discovery for comprehensive deep-dive research. Each mode adjusts token limits, temperature, and retrieval depth — so you control the speed-vs-thoroughness tradeoff per query."
              cta="Choose your depth"
              background={
                <div className="absolute top-5 right-8 flex gap-3 opacity-[0.06] pointer-events-none">
                  {['Brief', 'Standard', 'Discovery'].map((mode, i) => (
                    <div key={i} className="flex flex-col items-center gap-1">
                      <div className="rounded-full" style={{ width: 12 + i * 8, height: 12 + i * 8, background: '#c9a84c' }} />
                      <span className="text-[10px] font-medium" style={{ color: '#c9a84c' }}>{mode}</span>
                    </div>
                  ))}
                </div>
              }
            />
          </div>
        </Reveal>

      </div>
    </section>
  )
}

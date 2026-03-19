'use client'

import { Typewriter } from './Typewriter'

const stats = [
  { value: '100%', label: 'On-Device Processing' },
  { value: '0', label: 'Cloud Uploads Required' },
  { value: '7B', label: 'Parameter Legal AI Model' },
]

export default function Hero() {
  return (
    <section
      className="relative min-h-[100vh] flex flex-col items-center justify-center text-center px-6 pt-16 overflow-hidden"
      style={{ background: '#080808' }}
    >
      {/* Grid */}
      <div
        className="absolute inset-0"
        style={{
          backgroundImage:
            'linear-gradient(rgba(255,255,255,0.013) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,0.013) 1px, transparent 1px)',
          backgroundSize: '72px 72px',
        }}
      />

      {/* Large gold atmospheric glow — centered on icon */}
      <div
        className="absolute pointer-events-none"
        style={{
          width: '700px',
          height: '600px',
          background:
            'radial-gradient(ellipse at center, rgba(201,168,76,0.11) 0%, rgba(201,168,76,0.04) 40%, transparent 68%)',
          top: '0%',
          left: '50%',
          transform: 'translateX(-50%)',
        }}
      />

      {/* Radial vignette — white */}
      <div
        className="absolute top-0 left-0 right-0 h-[65vh] pointer-events-none"
        style={{
          background:
            'radial-gradient(ellipse 60% 45% at 50% 0%, rgba(255,255,255,0.018) 0%, transparent 70%)',
        }}
      />

      {/* Bottom fade */}
      <div
        className="absolute bottom-0 left-0 right-0 h-32"
        style={{ background: 'linear-gradient(to top, #080808, transparent)' }}
      />

      <div className="relative z-20 max-w-4xl mx-auto flex flex-col items-center pb-12">

        {/* Floating scales icon with multi-ring treatment */}
        <div className="hero-icon mb-10 relative inline-flex items-center justify-center">
          {/* Outermost pulse ring */}
          <div
            className="absolute rounded-full ring-pulse"
            style={{
              inset: '-22px',
              border: '1px solid rgba(201,168,76,0.1)',
            }}
          />
          {/* Inner ring */}
          <div
            className="absolute rounded-full"
            style={{
              inset: '-10px',
              border: '1px solid rgba(201,168,76,0.2)',
            }}
          />
          {/* Icon frame */}
          <div
            className="flex items-center justify-center"
            style={{
              width: 84,
              height: 84,
              borderRadius: '50%',
              background: 'rgba(201,168,76,0.07)',
              border: '1px solid rgba(201,168,76,0.28)',
              boxShadow:
                '0 0 80px rgba(201,168,76,0.2), 0 0 30px rgba(201,168,76,0.08), inset 0 0 20px rgba(201,168,76,0.04)',
            }}
          >
            <svg width="46" height="46" viewBox="0 0 96 96" fill="none" xmlns="http://www.w3.org/2000/svg">
              <circle cx="48" cy="48" r="44" fill="rgba(201,168,76,0.03)" />
              <circle cx="48" cy="17" r="3.5" fill="#c9a84c" />
              <rect x="46.25" y="17" width="3.5" height="54" fill="#c9a84c" rx="1" />
              <rect x="32" y="71" width="32" height="4" rx="2" fill="#c9a84c" />
              <rect x="40" y="75" width="16" height="4" rx="2" fill="#c9a84c" />
              <rect x="16" y="28" width="64" height="3.5" rx="1.75" fill="#c9a84c" />
              <line x1="23" y1="31.5" x2="18" y2="56" stroke="#c9a84c" strokeWidth="2.5" strokeLinecap="round" />
              <line x1="73" y1="31.5" x2="78" y2="56" stroke="#c9a84c" strokeWidth="2.5" strokeLinecap="round" />
              <path d="M8 56 Q18 68 28 56" stroke="#c9a84c" strokeWidth="3" fill="none" strokeLinecap="round" />
              <path d="M68 56 Q78 68 88 56" stroke="#c9a84c" strokeWidth="3" fill="none" strokeLinecap="round" />
            </svg>
          </div>
        </div>

        {/* Badge */}
        <div className="hero-badge mb-7">
          <span
            className="text-xs font-medium tracking-[0.2em] uppercase px-4 py-1.5 rounded-full"
            style={{
              border: '1px solid rgba(201,168,76,0.22)',
              color: 'rgba(201,168,76,0.7)',
              background: 'rgba(201,168,76,0.05)',
            }}
          >
            Private · Local · Open Source
          </span>
        </div>

        {/* Title */}
        <h1
          className="hero-title font-bold tracking-tight leading-none mb-7"
          style={{ fontSize: 'clamp(4rem, 12vw, 9rem)', letterSpacing: '-0.035em' }}
        >
          Justice <span style={{ color: '#c9a84c' }}>AI</span>
        </h1>

        {/* Typewriter subheading */}
        <div className="hero-sub mb-5" style={{ minHeight: '2rem' }}>
          <p
            className="text-lg sm:text-xl font-light"
            style={{ color: 'rgba(255,255,255,0.55)', letterSpacing: '-0.01em' }}
          >
            <Typewriter
              text="Search every document you own. In seconds. On your machine."
              startDelay={850}
              speed={32}
            />
          </p>
        </div>

        {/* Body */}
        <p
          className="hero-body text-base sm:text-lg leading-relaxed max-w-xl mb-6"
          style={{ color: 'rgba(255,255,255,0.45)' }}
        >
          Attorney-client privilege demands that your case files never touch a cloud server.
          Justice AI keeps everything on-device — document parsing, semantic search, and AI
          answer generation all run locally on your machine. No cloud. No API keys. No
          data ever leaves your computer.
        </p>

        {/* Disclaimer */}
        <p
          className="hero-disclaimer text-xs italic mb-10 px-5 py-2.5 rounded-lg"
          style={{
            color: 'rgba(255,255,255,0.32)',
            border: '1px solid rgba(255,255,255,0.08)',
            background: 'rgba(255,255,255,0.02)',
          }}
        >
          Not legal advice — a research tool for the attorneys who give it.
        </p>

        {/* CTAs */}
        <div className="hero-ctas flex flex-col sm:flex-row items-center justify-center gap-3 mb-16">
          <a
            href="#download"
            className="gold-solid-btn inline-flex items-center gap-2.5 font-semibold text-sm px-8 py-3.5 rounded-lg"
          >
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
              <path d="M8 2v9M4 8l4 4 4-4" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
              <path d="M2 14h12" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
            </svg>
            Download Free
          </a>
          <a
            href="#product"
            className="gold-outline-btn inline-flex items-center gap-2 text-sm font-medium px-7 py-3.5 rounded-lg"
          >
            See It In Action
            <svg width="13" height="13" viewBox="0 0 16 16" fill="none">
              <path d="M8 3l5 5-5 5M3 8h10" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          </a>
        </div>

        {/* Stats row */}
        <div className="hero-stats flex flex-col sm:flex-row items-center justify-center gap-8 sm:gap-12">
          {stats.map((s, i) => (
            <div key={i} className="flex flex-col items-center gap-1">
              <span
                className="text-2xl font-bold"
                style={{ color: '#ffffff', letterSpacing: '-0.02em' }}
              >
                {s.value}
              </span>
              <span className="text-xs tracking-[0.12em] uppercase" style={{ color: 'rgba(255,255,255,0.5)' }}>
                {s.label}
              </span>
            </div>
          ))}
        </div>

      </div>
    </section>
  )
}

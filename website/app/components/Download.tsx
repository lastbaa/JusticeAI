'use client'

import { useState, useEffect } from 'react'
import { Reveal } from './Reveal'
import { WordReveal } from './WordReveal'

type PlatformKey = 'mac' | 'windows' | 'linux'

const platforms: {
  key: PlatformKey
  label: string
  sub: string
  file: string
  available: boolean
  installSteps: string[]
  icon: React.ReactNode
}[] = [
  {
    key: 'mac',
    label: 'macOS',
    sub: 'Apple Silicon (native) · Intel via Rosetta 2 · macOS 12+',
    file: 'https://github.com/lastbaa/JusticeAI/releases/download/v1.4.0/Justice.AI-1.4.0-arm64.dmg',
    available: true,
    installSteps: [
      'Open the .dmg and drag Justice AI into your Applications folder, then eject the disk image.',
      'macOS blocks apps not from the App Store on first launch. Double-click to open — if blocked, go to System Settings → Privacy & Security → scroll down → click "Open Anyway". On macOS 12–13 you can also right-click the icon and choose Open.',
      'Justice AI automatically downloads the Saul legal AI model (~4.5 GB) on first open. This is a one-time setup — no accounts or API keys needed. After that, everything runs fully offline.',
      'Drag in your PDFs or Word documents, ask any legal question in plain English, and get cited answers grounded in your own files.',
    ],
    icon: (
      <svg width="17" height="21" viewBox="0 0 18 22" fill="currentColor">
        <path d="M14.94 11.44c-.02-2.53 2.06-3.75 2.16-3.81-1.18-1.72-3.01-1.96-3.66-1.98-1.56-.16-3.05.92-3.84.92-.79 0-2.01-.9-3.31-.88-1.7.03-3.27 1-4.14 2.52-1.77 3.07-.45 7.61 1.27 10.1.84 1.22 1.85 2.59 3.17 2.54 1.28-.05 1.76-.82 3.31-.82 1.54 0 1.98.82 3.33.8 1.37-.03 2.24-1.24 3.07-2.47.97-1.41 1.37-2.78 1.39-2.85-.03-.01-2.67-1.02-2.69-4.06zM12.47 3.8c.7-.85 1.17-2.02 1.04-3.2-1.01.04-2.22.67-2.94 1.52-.65.75-1.21 1.95-1.06 3.1 1.12.09 2.27-.57 2.96-1.42z" />
      </svg>
    ),
  },
  {
    key: 'windows',
    label: 'Windows',
    sub: 'Windows 10+ · x64 · Vulkan GPU recommended',
    file: 'https://github.com/lastbaa/JusticeAI/releases/download/v1.4.0/Justice.AI_1.4.0_x64-setup.exe',
    available: true,
    installSteps: [
      'Run the installer (.exe). Windows SmartScreen may warn about an unrecognized publisher — click "More info" → "Run anyway".',
      'Justice AI installs to your Program Files folder. A desktop shortcut is created automatically.',
      'On first launch the app downloads the Saul legal AI model (~4.5 GB). This is a one-time setup — no accounts or API keys needed. After that, everything runs fully offline.',
      'Drag in your PDFs or Word documents, ask any legal question in plain English, and get cited answers grounded in your own files.',
    ],
    icon: (
      <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
        <path d="M0 3.449L9.75 2.1v9.451H0m10.949-9.602L24 0v11.4H10.949M0 12.6h9.75v9.451L0 20.699M10.949 12.6H24V24l-13.051-1.8" />
      </svg>
    ),
  },
  {
    key: 'linux',
    label: 'Linux',
    sub: 'Ubuntu 22.04+ / Fedora 38+ · x64 · AppImage',
    file: 'https://github.com/lastbaa/JusticeAI/releases/download/v1.4.0/Justice.AI_1.4.0_amd64.AppImage',
    available: true,
    installSteps: [
      'Download the .AppImage file. Make it executable: chmod +x Justice.AI_1.4.0_amd64.AppImage, then double-click or run it from a terminal.',
      'If your desktop environment supports it, you can right-click the AppImage → Properties → Permissions → "Allow executing file as program".',
      'On first launch the app downloads the Saul legal AI model (~4.5 GB). This is a one-time setup — no accounts or API keys needed. After that, everything runs fully offline.',
      'Drag in your PDFs or Word documents, ask any legal question in plain English, and get cited answers grounded in your own files.',
    ],
    icon: (
      <svg width="17" height="17" viewBox="0 0 24 24" fill="currentColor">
        <path d="M12.504 0c-.155 0-.315.008-.48.021C7.309.191 4.693 2.688 4.693 6.036c0 1.715.565 3.138 1.489 4.206-.226 1.09-.697 2.457-.697 3.768 0 2.316.792 4.142 2.305 5.354 1.226 1.205 2.978 1.862 5.042 1.862 2.163 0 3.985-.78 5.33-2.199.966-1.133 1.518-2.681 1.518-4.381 0-1.232-.308-2.46-.791-3.509l-.106-.222c.888-1.032 1.394-2.374 1.394-3.879 0-3.382-2.636-5.993-7.673-6.036zm.087 1.485c4.315.039 6.405 2.241 6.405 5.016 0 1.316-.458 2.5-1.244 3.328l-.344.363.219.451c.486 1.002.79 2.167.79 3.274 0 1.434-.473 2.724-1.285 3.681-1.111 1.217-2.659 1.856-4.519 1.856-1.787 0-3.271-.549-4.296-1.588-1.253-1.012-1.9-2.573-1.9-4.554 0-1.175.447-2.477.67-3.438l.115-.495-.355-.373C5.897 9.169 5.367 7.866 5.367 6.29c0-2.809 2.122-4.739 6.224-4.8l.085-.005h.915zm.404 6.435c-1.085 0-1.965.88-1.965 1.965s.88 1.965 1.965 1.965 1.965-.88 1.965-1.965-.88-1.965-1.965-1.965zm-4.929 0c-1.085 0-1.965.88-1.965 1.965s.88 1.965 1.965 1.965 1.965-.88 1.965-1.965-.88-1.965-1.965-1.965z" />
      </svg>
    ),
  },
]

const requirements = [
  { label: 'macOS', value: 'macOS 12 Monterey+ · Apple Silicon or Intel', highlight: false },
  { label: 'Windows', value: 'Windows 10+ · x64 · Vulkan-compatible GPU recommended', highlight: false },
  { label: 'Linux', value: 'Ubuntu 22.04+ / Fedora 38+ · x64 · AppImage', highlight: false },
  { label: 'RAM', value: '8 GB minimum · 16 GB recommended for best performance', highlight: false },
  { label: 'Storage', value: '~5 GB total (app + Saul model, downloaded once)', highlight: false },
  { label: 'Network', value: 'Only on first launch to download the model — all AI runs fully offline after that', highlight: true },
  { label: 'Accounts', value: 'None required — no sign-up, no API keys, no subscriptions', highlight: false },
]

const setupSteps = [
  {
    number: '01',
    title: 'Download & Install',
    body: 'Choose your platform above. macOS: drag to Applications. Windows: run the installer. Linux: make the AppImage executable and run it. No terminal, no dependencies.',
  },
  {
    number: '02',
    title: 'One-Time Model Download',
    body: 'First launch downloads Saul-7B — the legal AI model (~4.5 GB). This happens once. After that, Justice AI runs fully offline, forever, with no internet required.',
  },
  {
    number: '03',
    title: 'Cited Answers From Your Documents',
    body: 'Drag in your PDFs and Word files. Ask any legal question in plain English. Get answers grounded in your own documents — with exact filename and page citations.',
  },
]

export default function Download() {
  const [detected, setDetected] = useState<PlatformKey>('mac')
  const [activePlatform, setActivePlatform] = useState<PlatformKey | null>(null)

  useEffect(() => {
    const ua = navigator.userAgent.toLowerCase()
    if (ua.includes('win')) setDetected('windows')
    else if (ua.includes('linux')) setDetected('linux')
    else setDetected('mac')
  }, [])

  function handleDownload(platform: typeof platforms[0]) {
    setActivePlatform(platform.key)
    window.open(platform.file, '_blank', 'noopener')
  }

  const shownSteps = activePlatform
    ? platforms.find((p) => p.key === activePlatform)!.installSteps
    : platforms.find((p) => p.key === detected)!.installSteps

  return (
    <section className="py-32 px-6" style={{ background: '#080808' }}>
      <div className="max-w-6xl mx-auto">
        <div className="border-t mb-16" style={{ borderColor: 'rgba(255,255,255,0.05)' }} />
      </div>

      <div className="max-w-3xl mx-auto">
        <Reveal className="flex justify-center mb-6">
          <span className="text-xs font-medium tracking-[0.2em] uppercase" style={{ color: 'rgba(201,168,76,0.55)' }}>
            05 — Get Started
          </span>
        </Reveal>

        <div className="text-center mb-4">
          <WordReveal
            text="Download Justice AI"
            as="h2"
            stagger={90}
            className="text-3xl sm:text-4xl font-bold text-white"
            style={{ letterSpacing: '-0.02em' }}
          />
        </div>
        <Reveal className="text-center mb-14">
          <p className="text-lg" style={{ color: 'rgba(255,255,255,0.38)' }}>
            Free and open source. Three steps to your first search.
          </p>
        </Reveal>

        {/* Setup steps */}
        <Reveal className="mb-12">
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
            {setupSteps.map((step) => (
              <div
                key={step.number}
                className="rounded-2xl px-5 py-5"
                style={{ background: '#0d0d0d', border: '1px solid rgba(255,255,255,0.06)' }}
              >
                <span
                  className="block text-[11px] font-bold tracking-[0.18em] uppercase mb-3"
                  style={{ color: 'rgba(201,168,76,0.5)' }}
                >
                  {step.number}
                </span>
                <p className="text-sm font-semibold text-white mb-2">{step.title}</p>
                <p className="text-xs leading-relaxed" style={{ color: 'rgba(255,255,255,0.5)' }}>
                  {step.body}
                </p>
              </div>
            ))}
          </div>
        </Reveal>

        <Reveal variant="scale" delay={120}>
          <div
            className="rounded-2xl overflow-hidden"
            style={{ background: '#0f0f0f', border: '1px solid rgba(255,255,255,0.07)' }}
          >
            <div className="h-px" style={{ background: 'rgba(255,255,255,0.12)' }} />

            <div className="p-10 sm:p-12">
              <div className="flex justify-center mb-8">
                <span
                  className="text-xs font-medium tracking-widest uppercase px-3 py-1 rounded-full"
                  style={{
                    border: '1px solid rgba(255,255,255,0.08)',
                    color: 'rgba(255,255,255,0.3)',
                    background: 'rgba(255,255,255,0.02)',
                  }}
                >
                  v1.5.0 · Open Source
                </span>
              </div>

              {/* Platform buttons */}
              <div className="flex flex-col sm:flex-row items-stretch gap-3 mb-8">
                {platforms.map((p) => {
                  const isDetected = p.key === detected
                  const isActive = p.key === activePlatform
                  const isPrimary = isDetected && p.available

                  return (
                    <button
                      key={p.key}
                      onClick={() => handleDownload(p)}
                      className={`group relative overflow-hidden rounded-2xl text-left transition-all duration-200 cursor-pointer ${isPrimary ? 'flex-[2]' : 'flex-1'}`}
                      style={{
                        background: isActive
                          ? 'rgba(201,168,76,0.07)'
                          : isPrimary
                          ? 'rgba(255,255,255,0.05)'
                          : 'rgba(255,255,255,0.025)',
                        border: `1px solid ${
                          isActive
                            ? 'rgba(201,168,76,0.3)'
                            : isPrimary
                            ? 'rgba(255,255,255,0.16)'
                            : 'rgba(255,255,255,0.06)'
                        }`,
                      }}
                      onMouseEnter={(e) => {
                        const el = e.currentTarget as HTMLButtonElement
                        el.style.transform = 'translateY(-2px)'
                        el.style.boxShadow = '0 12px 40px rgba(0,0,0,0.5)'
                        if (!isActive && !isPrimary) {
                          el.style.background = 'rgba(255,255,255,0.05)'
                          el.style.borderColor = 'rgba(255,255,255,0.14)'
                        }
                      }}
                      onMouseLeave={(e) => {
                        const el = e.currentTarget as HTMLButtonElement
                        el.style.transform = 'translateY(0)'
                        el.style.boxShadow = 'none'
                        if (!isActive && !isPrimary) {
                          el.style.background = 'rgba(255,255,255,0.025)'
                          el.style.borderColor = 'rgba(255,255,255,0.06)'
                        }
                      }}
                    >
                      <div className="px-5 py-5">
                        <div className="flex items-start justify-between mb-4">
                          <div
                            className="w-10 h-10 rounded-xl flex items-center justify-center"
                            style={{
                              background: isActive
                                ? 'rgba(201,168,76,0.12)'
                                : isPrimary
                                ? 'rgba(201,168,76,0.07)'
                                : 'rgba(255,255,255,0.04)',
                              border: `1px solid ${isActive ? 'rgba(201,168,76,0.25)' : isPrimary ? 'rgba(201,168,76,0.18)' : 'rgba(255,255,255,0.07)'}`,
                              color: isActive || isPrimary ? '#c9a84c' : 'rgba(255,255,255,0.4)',
                            }}
                          >
                            {isActive ? (
                              <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
                                <path d="M2.5 8.5l3.5 3.5 7-7" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
                              </svg>
                            ) : (
                              p.icon
                            )}
                          </div>
                          <div
                            className="w-7 h-7 rounded-lg flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity"
                            style={{ background: 'rgba(255,255,255,0.06)', border: '1px solid rgba(255,255,255,0.08)' }}
                          >
                            <svg width="12" height="12" viewBox="0 0 16 16" fill="none">
                              <path d="M8 2v9M4.5 7.5L8 11l3.5-3.5" stroke="rgba(255,255,255,0.7)" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
                            </svg>
                          </div>
                        </div>

                        <p className="text-sm font-semibold mb-0.5" style={{ color: 'white' }}>
                          {isActive ? 'Downloading…' : `Download for ${p.label}`}
                        </p>
                        <p className="text-xs" style={{ color: 'rgba(255,255,255,0.3)' }}>
                          {isActive ? 'Check your Downloads folder' : isPrimary ? '✦ Recommended for your device' : p.sub}
                        </p>
                        {isPrimary && !isActive && (
                          <span
                            className="inline-block mt-2 text-[10px] font-semibold tracking-wider uppercase px-2 py-0.5 rounded"
                            style={{ background: 'rgba(201,168,76,0.1)', color: 'rgba(201,168,76,0.7)', border: '1px solid rgba(201,168,76,0.2)' }}
                          >
                            Recommended
                          </span>
                        )}
                      </div>
                    </button>
                  )
                })}
              </div>

              {/* Install steps for selected/detected platform */}
              <div
                className="rounded-xl p-5 mb-8"
                style={{ background: 'rgba(255,255,255,0.025)', border: '1px solid rgba(255,255,255,0.06)' }}
              >
                <p className="text-xs font-semibold uppercase tracking-[0.15em] mb-4" style={{ color: 'rgba(255,255,255,0.25)' }}>
                  Setup · {platforms.find((p) => p.key === (activePlatform ?? detected))?.label}
                </p>
                <ol className="flex flex-col gap-2.5">
                  {shownSteps.map((step, i) => (
                    <li key={i} className="flex items-start gap-3">
                      <span
                        className="shrink-0 w-5 h-5 rounded-full flex items-center justify-center text-xs font-bold mt-px"
                        style={{ background: 'rgba(201,168,76,0.1)', color: 'rgba(201,168,76,0.6)' }}
                      >
                        {i + 1}
                      </span>
                      <span className="text-sm leading-relaxed" style={{ color: 'rgba(255,255,255,0.5)' }}>
                        {step}
                      </span>
                    </li>
                  ))}
                </ol>
              </div>

              <div className="border-t mb-8" style={{ borderColor: 'rgba(255,255,255,0.06)' }} />

              <h3
                className="text-xs font-semibold uppercase tracking-[0.15em] mb-5 text-center"
                style={{ color: 'rgba(255,255,255,0.2)' }}
              >
                System Requirements
              </h3>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
                {requirements.map((req) => (
                  <div
                    key={req.label}
                    className="flex items-start gap-3 rounded-lg px-4 py-3"
                    style={{
                      background: req.highlight ? 'rgba(63,185,80,0.04)' : 'rgba(255,255,255,0.025)',
                      border: `1px solid ${req.highlight ? 'rgba(63,185,80,0.15)' : 'rgba(255,255,255,0.05)'}`,
                    }}
                  >
                    <div>
                      <span
                        className="block text-xs uppercase tracking-wider mb-0.5 font-medium"
                        style={{ color: req.highlight ? 'rgba(63,185,80,0.6)' : 'rgba(255,255,255,0.25)' }}
                      >
                        {req.label}
                      </span>
                      <span className="text-sm" style={{ color: req.highlight ? 'rgba(255,255,255,0.75)' : 'rgba(255,255,255,0.6)' }}>
                        {req.value}
                      </span>
                    </div>
                  </div>
                ))}
              </div>

              <div className="border-t mt-8 mb-5" style={{ borderColor: 'rgba(255,255,255,0.06)' }} />

              {/* Sample document for demo */}
              <div
                className="rounded-xl p-5 mb-6"
                style={{ background: 'rgba(201,168,76,0.04)', border: '1px solid rgba(201,168,76,0.15)' }}
              >
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-xs font-semibold uppercase tracking-[0.15em] mb-1" style={{ color: 'rgba(201,168,76,0.6)' }}>
                      Sample Document
                    </p>
                    <p className="text-sm" style={{ color: 'rgba(255,255,255,0.6)' }}>
                      IRS W-9 (filled example) — try loading this into Justice AI to see cited answers in action.
                    </p>
                  </div>
                  <a
                    href="/irs_w9_filled.pdf"
                    download="IRS_W9_Filled_Example.pdf"
                    className="shrink-0 ml-4 flex items-center gap-2 rounded-lg px-4 py-2.5 text-sm font-semibold transition-all duration-200"
                    style={{
                      background: 'rgba(201,168,76,0.12)',
                      border: '1px solid rgba(201,168,76,0.3)',
                      color: '#c9a84c',
                    }}
                    onMouseEnter={(e) => {
                      const el = e.currentTarget as HTMLAnchorElement
                      el.style.background = 'rgba(201,168,76,0.2)'
                      el.style.borderColor = 'rgba(201,168,76,0.45)'
                    }}
                    onMouseLeave={(e) => {
                      const el = e.currentTarget as HTMLAnchorElement
                      el.style.background = 'rgba(201,168,76,0.12)'
                      el.style.borderColor = 'rgba(201,168,76,0.3)'
                    }}
                  >
                    <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
                      <path d="M8 2v9M4.5 7.5L8 11l3.5-3.5" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
                      <path d="M2 13h12" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
                    </svg>
                    Download PDF
                  </a>
                </div>
              </div>

              <p className="text-sm leading-relaxed" style={{ color: 'rgba(255,255,255,0.3)' }}>
                <span className="text-white font-medium">Everything runs on your machine.</span>{' '}
                The app installer is under 20 MB. First launch downloads Saul-7B once (~4.5 GB) — after that, document parsing, vector search, and AI answers run entirely offline. No accounts, no API keys, no data ever touches the cloud.
              </p>
            </div>
          </div>
        </Reveal>
      </div>
    </section>
  )
}

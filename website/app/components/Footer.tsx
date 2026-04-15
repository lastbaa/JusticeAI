'use client'

export default function Footer() {
  return (
    <footer className="py-14 px-6" style={{ background: '#080808', borderTop: '1px solid rgba(255,255,255,0.05)' }}>
      <div className="max-w-6xl mx-auto">
        <div className="flex flex-col md:flex-row items-start md:items-center justify-between gap-10">

          {/* Brand */}
          <div className="flex flex-col gap-3">
            <div className="flex items-center gap-2.5">
              <svg width="18" height="18" viewBox="0 0 28 28" fill="none" xmlns="http://www.w3.org/2000/svg">
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
              <span className="font-semibold text-sm text-white" style={{ letterSpacing: '-0.01em' }}>Justice AI</span>
            </div>
            <p className="text-xs max-w-[220px] leading-relaxed" style={{ color: 'rgba(255,255,255,0.45)' }}>
              Privacy-first legal research. Everything on your machine.
            </p>
          </div>

          {/* Links + copyright */}
          <div className="flex flex-col items-start md:items-end gap-4">
            <div className="flex items-center gap-5">
              <a
                href="https://github.com/lastbaa/JusticeAI"
                target="_blank"
                rel="noopener noreferrer"
                className="footer-link inline-flex items-center gap-1.5 text-xs"
              >
                <svg width="13" height="13" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" />
                </svg>
                GitHub
              </a>
              <a
                href="https://github.com/lastbaa/JusticeAI/blob/main/LICENSE"
                target="_blank"
                rel="noopener noreferrer"
                className="footer-link text-xs"
              >
                License
              </a>
              <a
                href="#how-it-works"
                className="footer-link text-xs"
              >
                How It Works
              </a>
            </div>
            <p className="text-xs" style={{ color: 'rgba(255,255,255,0.25)' }}>
              &copy; {new Date().getFullYear()} Justice AI
            </p>
          </div>
        </div>

        <div className="mt-12 pt-6" style={{ borderTop: '1px solid rgba(255,255,255,0.04)' }}>
          <p className="text-xs text-center" style={{ color: 'rgba(255,255,255,0.35)' }}>
            No telemetry. No cloud. No compromise. Your documents never leave your machine.
          </p>
        </div>
      </div>
    </footer>
  )
}

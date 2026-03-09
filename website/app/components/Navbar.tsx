'use client'

export default function Navbar() {
  return (
    <nav className="fixed top-0 left-0 right-0 z-50 h-16" style={{ background: 'rgba(8,8,8,0.85)', backdropFilter: 'blur(20px)', borderBottom: '1px solid rgba(255,255,255,0.06)' }}>
      <div className="max-w-6xl mx-auto px-6 h-full flex items-center justify-between">
        <a href="#" className="flex items-center gap-2.5 group">
          <svg width="24" height="24" viewBox="0 0 28 28" fill="none" xmlns="http://www.w3.org/2000/svg">
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
          <span className="text-white font-semibold text-sm tracking-widest uppercase">Justice AI</span>
        </a>
        <div className="flex items-center gap-4">
        <a
          href="/changelog"
          className="text-sm font-medium transition-colors"
          style={{ color: 'rgba(255,255,255,0.45)' }}
          onMouseEnter={e => { (e.currentTarget as HTMLAnchorElement).style.color = 'rgba(255,255,255,0.8)' }}
          onMouseLeave={e => { (e.currentTarget as HTMLAnchorElement).style.color = 'rgba(255,255,255,0.45)' }}
        >
          Changelog
        </a>
        <a
          href="#download"
          className="text-sm font-medium tracking-wider uppercase px-5 py-2 rounded-md transition-all duration-200"
          style={{ border: '1px solid rgba(201,168,76,0.45)', color: '#c9a84c', background: 'rgba(201,168,76,0.06)' }}
          onMouseEnter={e => {
            (e.currentTarget as HTMLAnchorElement).style.background = 'rgba(201,168,76,0.12)'
            ;(e.currentTarget as HTMLAnchorElement).style.borderColor = 'rgba(201,168,76,0.7)'
            ;(e.currentTarget as HTMLAnchorElement).style.color = '#e8c97e'
          }}
          onMouseLeave={e => {
            (e.currentTarget as HTMLAnchorElement).style.background = 'rgba(201,168,76,0.06)'
            ;(e.currentTarget as HTMLAnchorElement).style.borderColor = 'rgba(201,168,76,0.45)'
            ;(e.currentTarget as HTMLAnchorElement).style.color = '#c9a84c'
          }}
        >
          Download
        </a>
        </div>
      </div>
    </nav>
  )
}

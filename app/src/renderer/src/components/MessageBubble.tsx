import { ChatMessage, Citation } from '../../../../../shared/src/types'
import SourceCard from './SourceCard'

interface Props {
  message: ChatMessage
  onViewCitation?: (citation: Citation) => void
}

function formatTime(ts: number): string {
  return new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

function GavelAvatar(): JSX.Element {
  return (
    <div
      className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full mt-0.5"
      style={{ background: 'rgba(201,168,76,0.09)', border: '1px solid rgba(201,168,76,0.2)' }}
    >
      <svg width="13" height="13" viewBox="0 0 20 20" fill="none">
        <rect x="1" y="3" width="11" height="4" rx="1.25" fill="#c9a84c" transform="rotate(45 6.5 5)" />
        <line x1="10.5" y1="10.5" x2="18.5" y2="18.5" stroke="#c9a84c" strokeWidth="2.5" strokeLinecap="round" />
        <rect x="0.5" y="16.5" width="8.5" height="2.5" rx="0.75" fill="#c9a84c" opacity="0.38" />
      </svg>
    </div>
  )
}

// ── Lightweight markdown renderer (no external deps) ─────────────────────────
// Handles bold, italic, inline code, fenced code blocks, bullet + numbered lists.
function renderMarkdown(text: string): JSX.Element {
  const lines = text.split('\n')
  const elements: JSX.Element[] = []
  let i = 0

  while (i < lines.length) {
    const line = lines[i]

    // Fenced code block
    if (line.startsWith('```')) {
      const lang = line.slice(3).trim()
      const codeLines: string[] = []
      i++
      while (i < lines.length && !lines[i].startsWith('```')) {
        codeLines.push(lines[i])
        i++
      }
      elements.push(
        <pre key={i} style={{ background: '#080808', border: '1px solid rgba(255,255,255,0.1)', borderRadius: 10, padding: '12px 16px', overflowX: 'auto', margin: '8px 0' }}>
          <code style={{ fontFamily: "'SF Mono','Fira Mono',monospace", fontSize: '0.82em', color: 'rgba(255,255,255,0.82)', lineHeight: 1.65 }} data-lang={lang}>
            {codeLines.join('\n')}
          </code>
        </pre>
      )
      i++
      continue
    }

    // Bullet list item
    if (/^[-*•]\s/.test(line)) {
      const items: string[] = []
      while (i < lines.length && /^[-*•]\s/.test(lines[i])) {
        items.push(lines[i].replace(/^[-*•]\s+/, ''))
        i++
      }
      elements.push(
        <ul key={i} style={{ listStyle: 'disc', paddingLeft: '1.25em', margin: '4px 0 6px' }}>
          {items.map((item, j) => (
            <li key={j} style={{ marginBottom: '0.15em' }}>{inlineMarkdown(item)}</li>
          ))}
        </ul>
      )
      continue
    }

    // Numbered list item
    if (/^\d+\.\s/.test(line)) {
      const items: string[] = []
      while (i < lines.length && /^\d+\.\s/.test(lines[i])) {
        items.push(lines[i].replace(/^\d+\.\s+/, ''))
        i++
      }
      elements.push(
        <ol key={i} style={{ listStyle: 'decimal', paddingLeft: '1.25em', margin: '4px 0 6px' }}>
          {items.map((item, j) => (
            <li key={j} style={{ marginBottom: '0.15em' }}>{inlineMarkdown(item)}</li>
          ))}
        </ol>
      )
      continue
    }

    // Headings
    const headingMatch = line.match(/^(#{1,3})\s+(.+)/)
    if (headingMatch) {
      const level = headingMatch[1].length
      const content = headingMatch[2]
      const style: React.CSSProperties = { fontWeight: 600, color: '#fff', margin: '8px 0 3px', lineHeight: 1.3, fontSize: level === 1 ? '1.05em' : level === 2 ? '0.98em' : '0.93em' }
      elements.push(<p key={i} style={style}>{inlineMarkdown(content)}</p>)
      i++
      continue
    }

    // Blank line
    if (line.trim() === '') {
      // Only add spacing if there's content above
      if (elements.length > 0) {
        elements.push(<div key={i} style={{ height: '0.4em' }} />)
      }
      i++
      continue
    }

    // Normal paragraph line
    elements.push(
      <p key={i} style={{ margin: 0 }}>{inlineMarkdown(line)}</p>
    )
    i++
  }

  return <>{elements}</>
}

// Inline markdown: **bold**, *italic*, `code`
function inlineMarkdown(text: string): (string | JSX.Element)[] {
  const parts: (string | JSX.Element)[] = []
  // Combined regex: **bold**, *italic*, `code`
  const re = /(\*\*(.+?)\*\*|\*(.+?)\*|`([^`]+)`)/g
  let last = 0
  let m: RegExpExecArray | null

  while ((m = re.exec(text)) !== null) {
    if (m.index > last) parts.push(text.slice(last, m.index))
    if (m[0].startsWith('**')) {
      parts.push(<strong key={m.index} style={{ color: '#fff', fontWeight: 600 }}>{m[2]}</strong>)
    } else if (m[0].startsWith('*')) {
      parts.push(<em key={m.index}>{m[3]}</em>)
    } else {
      parts.push(
        <code key={m.index} style={{ fontFamily: "'SF Mono','Fira Mono',monospace", fontSize: '0.85em', background: 'rgba(255,255,255,0.07)', border: '1px solid rgba(255,255,255,0.08)', borderRadius: 4, padding: '0.1em 0.35em' }}>
          {m[4]}
        </code>
      )
    }
    last = m.index + m[0].length
  }

  if (last < text.length) parts.push(text.slice(last))
  return parts
}

export default function MessageBubble({ message, onViewCitation }: Props): JSX.Element {
  const isUser = message.role === 'user'

  if (isUser) {
    return (
      <div className="flex justify-end" style={{ animation: 'fadeUp 0.25s ease both' }}>
        <div style={{ maxWidth: '72%' }}>
          <div
            className="rounded-2xl rounded-tr-sm px-4 py-3"
            style={{
              background: '#161616',
              border: '1px solid rgba(255,255,255,0.09)',
              boxShadow: '0 1px 8px rgba(0,0,0,0.25)',
            }}
          >
            <p className="text-[13.5px] text-white leading-relaxed whitespace-pre-wrap">
              {message.content}
            </p>
          </div>
          <p className="mt-1.5 text-right text-[10px]" style={{ color: 'rgba(255,255,255,0.2)' }}>
            {formatTime(message.timestamp)}
          </p>
        </div>
      </div>
    )
  }

  const isNotFound = message.notFound

  return (
    <div className="flex gap-3" style={{ animation: 'fadeUp 0.25s ease both' }}>
      <GavelAvatar />

      <div className="flex-1 min-w-0">
        <p
          className="mb-2.5 text-[10px] font-bold tracking-[0.14em] uppercase"
          style={{ color: 'rgba(201,168,76,0.6)' }}
        >
          Justice AI
        </p>

        {isNotFound ? (
          <div
            className="rounded-xl px-4 py-3.5"
            style={{
              background: 'rgba(255,255,255,0.02)',
              border: '1px solid rgba(255,255,255,0.07)',
              borderLeft: '2px solid rgba(255,255,255,0.15)',
            }}
          >
            <div className="flex items-start gap-2.5">
              <svg width="13" height="13" viewBox="0 0 16 16" fill="none" className="shrink-0 mt-0.5">
                <path d="M8 1a7 7 0 1 0 0 14A7 7 0 0 0 8 1zm0 3.5a.75.75 0 0 1 .75.75v3a.75.75 0 0 1-1.5 0v-3A.75.75 0 0 1 8 4.5zm0 6.5a.75.75 0 1 1 0-1.5.75.75 0 0 1 0 1.5z" fill="rgba(255,255,255,0.25)" />
              </svg>
              <p
                className="text-[13px] leading-relaxed whitespace-pre-wrap"
                style={{ color: 'rgba(255,255,255,0.85)' }}
              >
                {message.content}
              </p>
            </div>
          </div>
        ) : (
          <div
            className="rounded-2xl rounded-tl-sm px-4 py-4"
            style={{
              background: '#0d0d0d',
              border: '1px solid rgba(255,255,255,0.06)',
              borderLeft: '2px solid rgba(201,168,76,0.22)',
            }}
          >
            <div className="text-[13.5px] text-white leading-[1.75]">
              {message.content.trim()
                ? renderMarkdown(message.content)
                : <span style={{ color: 'rgba(255,255,255,0.2)', fontStyle: 'italic' }}>No response generated. Please try again.</span>
              }
            </div>
          </div>
        )}

        <p className="mt-2 text-[10px]" style={{ color: 'rgba(255,255,255,0.22)' }}>
          {formatTime(message.timestamp)}
        </p>

        {!isNotFound && message.citations && message.citations.length > 0 && (
          <div className="mt-3">
            <p
              className="mb-2 text-[10px] font-semibold uppercase tracking-[0.12em]"
              style={{ color: 'rgba(255,255,255,0.18)' }}
            >
              Sources
            </p>
            <div className="flex flex-col gap-1.5">
              {message.citations.slice(0, 3).map((citation, idx) => (
                <SourceCard key={idx} citation={citation} onView={onViewCitation} />
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  )
}

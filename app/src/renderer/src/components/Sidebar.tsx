import { useRef, useState } from 'react'
import { ChatSession } from '../../../../../shared/src/types'

interface Props {
  sessions: ChatSession[]
  currentSessionId: string
  isLoading: boolean
  onGoHome: () => void
  onNewChat: () => void
  onLoadSession: (session: ChatSession) => void
  onDeleteSession: (sessionId: string) => void
  onRenameSession: (id: string, newName: string) => void
  onClearSessions: () => void
  onAddFiles: () => void
  onOpenSettings: () => void
}

function GavelIcon({ size = 16 }: { size?: number }): JSX.Element {
  return (
    <svg width={size} height={size} viewBox="0 0 20 20" fill="none">
      <rect x="1" y="3" width="11" height="4" rx="1.25" fill="#c9a84c" transform="rotate(45 6.5 5)" />
      <line x1="10.5" y1="10.5" x2="18.5" y2="18.5" stroke="#c9a84c" strokeWidth="2.5" strokeLinecap="round" />
      <rect x="0.5" y="16.5" width="8.5" height="2.5" rx="0.75" fill="#c9a84c" opacity="0.38" />
    </svg>
  )
}

function groupSessions(sessions: ChatSession[]): { label: string; items: ChatSession[] }[] {
  const now = Date.now()
  const day = 86_400_000
  const groups = [
    { label: 'Today', items: [] as ChatSession[] },
    { label: 'Yesterday', items: [] as ChatSession[] },
    { label: 'Last 7 days', items: [] as ChatSession[] },
    { label: 'Last 30 days', items: [] as ChatSession[] },
    { label: 'Older', items: [] as ChatSession[] },
  ]
  for (const s of sessions) {
    const age = now - s.updatedAt
    if (age < day) groups[0].items.push(s)
    else if (age < 2 * day) groups[1].items.push(s)
    else if (age < 7 * day) groups[2].items.push(s)
    else if (age < 30 * day) groups[3].items.push(s)
    else groups[4].items.push(s)
  }
  return groups.filter((g) => g.items.length > 0)
}

function SessionItem({
  session,
  isActive,
  onLoad,
  onDelete,
  onRename,
}: {
  session: ChatSession
  isActive: boolean
  onLoad: () => void
  onDelete: () => void
  onRename: (newName: string) => void
}): JSX.Element {
  const [hovered, setHovered] = useState(false)
  const [editing, setEditing] = useState(false)
  const [editName, setEditName] = useState('')
  const inputRef = useRef<HTMLInputElement>(null)

  function startEdit(e: React.MouseEvent): void {
    e.stopPropagation()
    setEditName(session.name)
    setEditing(true)
    setTimeout(() => inputRef.current?.select(), 0)
  }

  function commitEdit(): void {
    setEditing(false)
    const trimmed = editName.trim()
    if (trimmed && trimmed !== session.name) {
      onRename(trimmed)
    }
  }

  function handleEditKeyDown(e: React.KeyboardEvent<HTMLInputElement>): void {
    if (e.key === 'Enter') { e.preventDefault(); commitEdit() }
    if (e.key === 'Escape') setEditing(false)
  }

  if (editing) {
    return (
      <div
        className="relative flex items-center gap-2 rounded-lg px-3 py-1.5"
        style={{ background: '#191919', border: '1px solid rgba(201,168,76,0.3)' }}
      >
        <div
          className="absolute left-0 top-1/2 -translate-y-1/2 w-[2px] h-4 rounded-full"
          style={{ background: '#c9a84c' }}
        />
        <input
          ref={inputRef}
          value={editName}
          onChange={(e) => setEditName(e.target.value)}
          onBlur={commitEdit}
          onKeyDown={handleEditKeyDown}
          autoFocus
          className="flex-1 bg-transparent text-[12px] leading-snug outline-none text-white placeholder-white/30"
          style={{ minWidth: 0 }}
          maxLength={60}
        />
      </div>
    )
  }

  return (
    <div
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onClick={onLoad}
      className="group relative flex items-center gap-2 rounded-lg px-3 py-2 cursor-pointer transition-all"
      style={{
        background: isActive ? '#191919' : hovered ? '#111' : 'transparent',
        color: isActive ? 'rgba(255,255,255,0.85)' : hovered ? 'rgba(255,255,255,0.6)' : 'rgba(255,255,255,0.32)',
      }}
    >
      {isActive && (
        <div
          className="absolute left-0 top-1/2 -translate-y-1/2 w-[2px] h-4 rounded-full"
          style={{ background: '#c9a84c' }}
        />
      )}
      <span className="flex-1 truncate text-[12px] leading-snug" title={session.name}>
        {session.name}
      </span>
      {hovered && (
        <div className="no-drag shrink-0 flex items-center gap-1">
          {/* Rename button */}
          <button
            onClick={startEdit}
            title="Rename"
            className="flex h-4 w-4 items-center justify-center rounded transition-colors"
            style={{ color: 'rgba(255,255,255,0.18)' }}
            onMouseEnter={(e) => { (e.currentTarget as HTMLButtonElement).style.color = 'rgba(255,255,255,0.55)' }}
            onMouseLeave={(e) => { (e.currentTarget as HTMLButtonElement).style.color = 'rgba(255,255,255,0.18)' }}
          >
            <svg width="9" height="9" viewBox="0 0 16 16" fill="currentColor">
              <path d="M11.013 1.427a1.75 1.75 0 0 1 2.474 0l1.086 1.086a1.75 1.75 0 0 1 0 2.474l-8.61 8.61c-.21.21-.47.364-.756.445l-3.251.93a.75.75 0 0 1-.927-.928l.929-3.25c.081-.286.235-.547.445-.758l8.61-8.61zm.176 4.823L9.75 4.81l-6.286 6.287a.253.253 0 0 0-.064.108l-.558 1.953 1.953-.558a.253.253 0 0 0 .108-.064l6.286-6.286zm1.238-3.763a.25.25 0 0 0-.354 0L10.811 3.75l1.439 1.44 1.263-1.263a.25.25 0 0 0 0-.354l-1.086-1.086z" />
            </svg>
          </button>
          {/* Delete button */}
          <button
            onClick={(e) => { e.stopPropagation(); onDelete() }}
            title="Delete"
            className="flex h-4 w-4 items-center justify-center rounded transition-colors"
            style={{ color: 'rgba(255,255,255,0.18)' }}
            onMouseEnter={(e) => { (e.currentTarget as HTMLButtonElement).style.color = '#f85149' }}
            onMouseLeave={(e) => { (e.currentTarget as HTMLButtonElement).style.color = 'rgba(255,255,255,0.18)' }}
          >
            <svg width="9" height="9" viewBox="0 0 12 12" fill="currentColor">
              <path d="M1.22 1.22a.75.75 0 0 1 1.06 0L6 4.94l3.72-3.72a.75.75 0 1 1 1.06 1.06L7.06 6l3.72 3.72a.75.75 0 1 1-1.06 1.06L6 7.06l-3.72 3.72a.75.75 0 0 1-1.06-1.06L4.94 6 1.22 2.28a.75.75 0 0 1 0-1.06z" />
            </svg>
          </button>
        </div>
      )}
    </div>
  )
}

export default function Sidebar({
  sessions,
  currentSessionId,
  isLoading,
  onGoHome,
  onNewChat,
  onLoadSession,
  onDeleteSession,
  onRenameSession,
  onClearSessions,
  onAddFiles,
  onOpenSettings,
}: Props): JSX.Element {
  const [searchQuery, setSearchQuery] = useState('')

  const filteredSessions = searchQuery.trim()
    ? sessions.filter((s) =>
        s.name.toLowerCase().includes(searchQuery.toLowerCase())
      )
    : sessions

  const groups = groupSessions(filteredSessions)

  return (
    <aside
      className="flex h-screen w-[240px] shrink-0 flex-col"
      style={{ background: '#080808', borderRight: '1px solid rgba(255,255,255,0.05)' }}
    >
      {/* Drag region + logo */}
      <div className="drag-region flex items-center gap-2.5 px-4 pt-4 pb-4">
        <button
          onClick={onGoHome}
          className="no-drag flex items-center gap-2 hover:opacity-75 transition-opacity"
        >
          <GavelIcon size={17} />
          <span className="text-[14px] font-semibold tracking-[-0.015em] text-white">
            Justice <span style={{ color: '#c9a84c' }}>AI</span>
          </span>
        </button>
      </div>

      {/* New Chat button */}
      <div className="px-3 pt-0 pb-3">
        <button
          onClick={onNewChat}
          className="no-drag flex w-full items-center gap-2.5 rounded-xl px-3 py-2.5 text-[12px] font-medium transition-all"
          style={{
            background: 'rgba(255,255,255,0.04)',
            border: '1px solid rgba(255,255,255,0.07)',
            color: 'rgba(255,255,255,0.45)',
          }}
          onMouseEnter={(e) => {
            const el = e.currentTarget as HTMLButtonElement
            el.style.background = 'rgba(255,255,255,0.07)'
            el.style.borderColor = 'rgba(255,255,255,0.1)'
            el.style.color = 'rgba(255,255,255,0.75)'
          }}
          onMouseLeave={(e) => {
            const el = e.currentTarget as HTMLButtonElement
            el.style.background = 'rgba(255,255,255,0.04)'
            el.style.borderColor = 'rgba(255,255,255,0.07)'
            el.style.color = 'rgba(255,255,255,0.45)'
          }}
        >
          <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
            <path d="M7.75 2a.75.75 0 0 1 .75.75V7h4.25a.75.75 0 0 1 0 1.5H8.5v4.25a.75.75 0 0 1-1.5 0V8.5H2.75a.75.75 0 0 1 0-1.5H7V2.75A.75.75 0 0 1 7.75 2z" />
          </svg>
          New chat
        </button>
      </div>

      {/* Divider */}
      <div className="mx-3 mb-2 h-px" style={{ background: 'rgba(255,255,255,0.04)' }} />

      {/* Sessions header + search */}
      {sessions.length > 0 && (
        <>
          <div className="flex items-center justify-between px-5 mb-1.5">
            <span className="text-[10px] font-semibold uppercase tracking-[0.1em]" style={{ color: 'rgba(255,255,255,0.18)' }}>
              Chats
            </span>
            <button
              onClick={onClearSessions}
              title="Clear all conversations"
              className="flex items-center gap-1 text-[9px] font-semibold px-2 py-0.5 rounded-md transition-all"
              style={{
                background: 'rgba(255,255,255,0.04)',
                border: '1px solid rgba(255,255,255,0.08)',
                color: 'rgba(255,255,255,0.25)',
              }}
              onMouseEnter={(e) => {
                const el = e.currentTarget as HTMLButtonElement
                el.style.color = '#f85149'
                el.style.borderColor = 'rgba(248,81,73,0.3)'
              }}
              onMouseLeave={(e) => {
                const el = e.currentTarget as HTMLButtonElement
                el.style.color = 'rgba(255,255,255,0.25)'
                el.style.borderColor = 'rgba(255,255,255,0.08)'
              }}
            >
              <svg width="9" height="9" viewBox="0 0 16 16" fill="currentColor">
                <path d="M11 1.75V3h2.25a.75.75 0 0 1 0 1.5H2.75a.75.75 0 0 1 0-1.5H5V1.75C5 .784 5.784 0 6.75 0h2.5C10.216 0 11 .784 11 1.75zM4.496 6.675l.66 6.6a.25.25 0 0 0 .249.225h5.19a.25.25 0 0 0 .249-.225l.66-6.6a.75.75 0 0 1 1.492.149l-.66 6.6A1.748 1.748 0 0 1 10.595 15h-5.19a1.75 1.75 0 0 1-1.741-1.575l-.66-6.6a.75.75 0 1 1 1.492-.15z" />
              </svg>
              Clear all
            </button>
          </div>

          {/* Search */}
          <div className="px-3 mb-1.5">
            <div
              className="flex items-center gap-2 rounded-lg px-2.5 py-1.5"
              style={{ background: 'rgba(255,255,255,0.03)', border: '1px solid rgba(255,255,255,0.06)' }}
            >
              <svg width="10" height="10" viewBox="0 0 16 16" fill="rgba(255,255,255,0.25)" className="shrink-0">
                <path d="M10.68 11.74a6 6 0 0 1-7.922-8.982 6 6 0 0 1 8.982 7.922l3.04 3.04a.75.75 0 1 1-1.06 1.06l-3.04-3.04zm-5.943-1.044a4.5 4.5 0 1 0 6.364-6.364 4.5 4.5 0 0 0-6.364 6.364z" />
              </svg>
              <input
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Search chats…"
                className="flex-1 bg-transparent text-[11px] text-white placeholder-white/20 outline-none"
              />
              {searchQuery && (
                <button onClick={() => setSearchQuery('')} style={{ color: 'rgba(255,255,255,0.25)' }}>
                  <svg width="9" height="9" viewBox="0 0 12 12" fill="currentColor">
                    <path d="M1.22 1.22a.75.75 0 0 1 1.06 0L6 4.94l3.72-3.72a.75.75 0 1 1 1.06 1.06L7.06 6l3.72 3.72a.75.75 0 1 1-1.06 1.06L6 7.06l-3.72 3.72a.75.75 0 0 1-1.06-1.06L4.94 6 1.22 2.28a.75.75 0 0 1 0-1.06z" />
                  </svg>
                </button>
              )}
            </div>
          </div>
        </>
      )}

      {/* Sessions list */}
      <div className="flex-1 overflow-y-auto px-2 py-1">
        {sessions.length === 0 ? (
          <div className="flex flex-col items-center py-12 text-center px-4">
            <div
              className="mb-3 flex h-9 w-9 items-center justify-center rounded-xl"
              style={{ background: 'rgba(255,255,255,0.03)', border: '1px solid rgba(255,255,255,0.05)' }}
            >
              <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
                <path
                  d="M1 2.75C1 1.784 1.784 1 2.75 1h10.5c.966 0 1.75.784 1.75 1.75v7.5A1.75 1.75 0 0 1 13.25 12H9.06l-2.573 2.573A1.458 1.458 0 0 1 4 13.543V12H2.75A1.75 1.75 0 0 1 1 10.25z"
                  stroke="rgba(255,255,255,0.1)"
                  strokeWidth="1.2"
                  fill="none"
                />
              </svg>
            </div>
            <p className="text-[11px]" style={{ color: 'rgba(255,255,255,0.15)' }}>No chats yet</p>
            <p className="mt-0.5 text-[10px]" style={{ color: 'rgba(255,255,255,0.09)' }}>Sessions auto-save</p>
          </div>
        ) : filteredSessions.length === 0 ? (
          <div className="flex flex-col items-center py-8 text-center px-4">
            <p className="text-[11px]" style={{ color: 'rgba(255,255,255,0.18)' }}>No results for "{searchQuery}"</p>
          </div>
        ) : (
          <div className="flex flex-col gap-5 py-1">
            {groups.map((group) => (
              <div key={group.label}>
                <p
                  className="mb-1 px-3 text-[10px] font-semibold uppercase tracking-[0.1em]"
                  style={{ color: 'rgba(255,255,255,0.18)' }}
                >
                  {group.label}
                </p>
                <div className="flex flex-col gap-0.5">
                  {group.items.map((session) => (
                    <SessionItem
                      key={session.id}
                      session={session}
                      isActive={session.id === currentSessionId}
                      onLoad={() => onLoadSession(session)}
                      onDelete={() => onDeleteSession(session.id)}
                      onRename={(name) => onRenameSession(session.id, name)}
                    />
                  ))}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Bottom actions */}
      <div
        className="px-3 py-3 flex flex-col gap-1"
        style={{ borderTop: '1px solid rgba(255,255,255,0.04)' }}
      >
        <button
          onClick={onAddFiles}
          disabled={isLoading}
          className="no-drag flex w-full items-center gap-2.5 rounded-lg px-3 py-2.5 text-[12px] transition-all disabled:opacity-40"
          style={{ color: 'rgba(255,255,255,0.35)' }}
          onMouseEnter={(e) => {
            const el = e.currentTarget as HTMLButtonElement
            el.style.background = 'rgba(255,255,255,0.04)'
            el.style.color = 'rgba(255,255,255,0.65)'
          }}
          onMouseLeave={(e) => {
            const el = e.currentTarget as HTMLButtonElement
            el.style.background = 'transparent'
            el.style.color = 'rgba(255,255,255,0.35)'
          }}
        >
          <svg width="12" height="12" viewBox="0 0 16 16" fill="rgba(201,168,76,0.6)">
            <path d="M2 1.75C2 .784 2.784 0 3.75 0h6.586c.464 0 .909.184 1.237.513l2.914 2.914c.329.328.513.773.513 1.237v9.586A1.75 1.75 0 0 1 13.25 16h-9.5A1.75 1.75 0 0 1 2 14.25V1.75zM8.75 9.25a.75.75 0 0 0-1.5 0v1.5H5.75a.75.75 0 0 0 0 1.5h1.5v1.5a.75.75 0 0 0 1.5 0v-1.5h1.5a.75.75 0 0 0 0-1.5H8.75v-1.5z" />
          </svg>
          {isLoading ? 'Loading…' : 'Add Documents'}
        </button>
        <button
          onClick={onOpenSettings}
          className="no-drag flex w-full items-center gap-2.5 rounded-lg px-3 py-2.5 text-[12px] transition-all"
          style={{ color: 'rgba(255,255,255,0.35)' }}
          onMouseEnter={(e) => {
            const el = e.currentTarget as HTMLButtonElement
            el.style.background = 'rgba(255,255,255,0.04)'
            el.style.color = 'rgba(255,255,255,0.65)'
          }}
          onMouseLeave={(e) => {
            const el = e.currentTarget as HTMLButtonElement
            el.style.background = 'transparent'
            el.style.color = 'rgba(255,255,255,0.35)'
          }}
        >
          <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
            <path d="M8 0a8.2 8.2 0 0 1 .701.031C9.444.095 9.99.645 10.16 1.29l.288 1.107c.018.066.079.158.212.224.231.114.454.243.668.386.123.082.233.09.299.071l1.103-.303c.644-.176 1.392.021 1.82.63.27.385.506.792.704 1.218.315.675.111 1.422-.364 1.891l-.814.806c-.049.048-.098.147-.088.294.016.257.016.515 0 .772-.01.147.038.246.088.294l.814.806c.475.469.679 1.216.364 1.891a7.977 7.977 0 0 1-.704 1.217c-.428.61-1.176.807-1.82.63l-1.103-.303c-.066-.019-.176-.011-.299.071a5.909 5.909 0 0 1-.668.386c-.133.066-.194.158-.212.224l-.288 1.107c-.17.645-.715 1.195-1.459 1.259a8.205 8.205 0 0 1-1.402 0c-.744-.064-1.289-.614-1.459-1.259l-.288-1.107c-.017-.066-.079-.158-.212-.224a5.738 5.738 0 0 1-.668-.386c-.123-.082-.233-.09-.299-.071l-1.103.303c-.644.176-1.392-.021-1.82-.63a8.12 8.12 0 0 1-.704-1.218c-.315-.675-.111-1.422.363-1.891l.815-.806c.05-.048.098-.147.088-.294a6.214 6.214 0 0 1 0-.772c.01-.147-.038-.246-.088-.294l-.815-.806C.635 6.045.431 5.298.746 4.623a7.92 7.92 0 0 1 .704-1.217c.428-.61 1.176-.807 1.82-.63l1.103.303c.066.019.176.011.299-.071.214-.143.437-.272.668-.386.133-.066.194-.158.212-.224l.288-1.107C6.01.645 6.556.095 7.299.03 7.53.01 7.765 0 8 0zm-.571 1.525c-.036.003-.108.036-.137.146l-.289 1.105c-.147.561-.549.967-.998 1.189-.173.086-.34.183-.5.29-.417.278-.97.423-1.529.27l-1.103-.303c-.109-.03-.175.016-.195.045-.22.312-.412.644-.573.99-.014.031-.021.11.059.19l.815.806c.411.406.562.957.53 1.456a4.709 4.709 0 0 0 0 .582c.032.499-.119 1.05-.53 1.456l-.815.806c-.081.08-.073.159-.059.19.162.346.353.677.573.989.02.03.085.076.195.046l1.102-.303c.56-.153 1.113-.008 1.53.27.161.107.328.204.501.29.447.222.85.629.997 1.189l.289 1.105c.029.109.101.143.137.146a6.6 6.6 0 0 0 1.142 0c.036-.003.108-.036.137-.146l.289-1.105c.147-.561.549-.967.998-1.189.173-.086.34-.183.5-.29.417-.278.97-.423 1.529-.27l1.103.303c.109.029.175-.016.195-.045.22-.313.411-.644.573-.99.014-.031.021-.11-.059-.19l-.815-.806c-.411-.406-.562-.957-.53-1.456a4.709 4.709 0 0 0 0-.582c-.032-.499.119-1.05.53-1.456l.815-.806c.081-.08.073-.159.059-.19a6.464 6.464 0 0 0-.573-.989c-.02-.03-.085-.076-.195-.046l-1.102.303c-.56.153-1.113.008-1.53-.27a4.44 4.44 0 0 0-.501-.29c-.447-.222-.85-.629-.997-1.189l-.289-1.105c-.029-.11-.101-.143-.137-.146a6.6 6.6 0 0 0-1.142 0zM8 5.5a2.5 2.5 0 1 1 0 5 2.5 2.5 0 0 1 0-5z" />
          </svg>
          Settings
        </button>
      </div>
    </aside>
  )
}

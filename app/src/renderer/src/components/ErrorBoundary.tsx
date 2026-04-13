import React from 'react'

interface Props {
  children: React.ReactNode
}

interface State {
  hasError: boolean
  error?: Error
}

/**
 * Top-level error boundary. Without this, any synchronous render error in any
 * component crashes the entire React tree and leaves a blank screen. With it,
 * users see a recoverable error message instead of a white void.
 */
export default class ErrorBoundary extends React.Component<Props, State> {
  constructor(props: Props) {
    super(props)
    this.state = { hasError: false }
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error }
  }

  componentDidCatch(error: Error, info: React.ErrorInfo): void {
    console.error('[Justice AI] Unhandled render error:', error, info)
  }

  render(): React.ReactNode {
    if (this.state.hasError) {
      return (
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            height: '100vh',
            background: 'var(--bg)',
            color: 'rgb(var(--ov) / 0.7)',
            gap: 12,
            padding: 32,
            textAlign: 'center',
          }}
        >
          <svg width="32" height="32" viewBox="0 0 16 16" fill="none">
            <path
              d="M8 1a7 7 0 1 0 0 14A7 7 0 0 0 8 1zm0 3.5a.75.75 0 0 1 .75.75v3a.75.75 0 0 1-1.5 0v-3A.75.75 0 0 1 8 4.5zm0 6.5a.75.75 0 1 1 0-1.5.75.75 0 0 1 0 1.5z"
              fill="rgba(248,81,73,0.7)"
            />
          </svg>
          <p style={{ fontSize: 15, fontWeight: 600, color: 'rgb(var(--ov) / 0.85)' }}>
            Something went wrong
          </p>
          <p style={{ fontSize: 12.5, color: 'rgb(var(--ov) / 0.55)', maxWidth: 380 }}>
            {this.state.error?.message ?? 'An unexpected error occurred.'}
          </p>
          <button
            onClick={() => this.setState({ hasError: false, error: undefined })}
            style={{
              marginTop: 8,
              padding: '8px 20px',
              borderRadius: 10,
              background: 'rgba(201,168,76,0.1)',
              border: '1px solid rgba(201,168,76,0.25)',
              color: '#c9a84c',
              fontSize: 13,
              cursor: 'pointer',
            }}
          >
            Try again
          </button>
        </div>
      )
    }
    return this.props.children
  }
}

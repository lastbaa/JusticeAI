import React from 'react'

interface Props {
  children: React.ReactNode
  name: string
}

interface State {
  hasError: boolean
}

/**
 * Lightweight error boundary for individual panels.
 * Catches render errors in a single panel without crashing the entire app.
 */
export default class PanelErrorBoundary extends React.Component<Props, State> {
  constructor(props: Props) {
    super(props)
    this.state = { hasError: false }
  }

  static getDerivedStateFromError(): State {
    return { hasError: true }
  }

  componentDidCatch(error: Error, info: React.ErrorInfo): void {
    console.error(`[Justice AI] Error in ${this.props.name}:`, error, info)
  }

  render(): React.ReactNode {
    if (this.state.hasError) {
      return (
        <div
          className="flex flex-col items-center justify-center gap-2 p-6 text-center"
          style={{ color: 'rgb(var(--ov) / 0.5)' }}
        >
          <p className="text-sm font-medium" style={{ color: 'rgb(var(--ov) / 0.7)' }}>
            {this.props.name} encountered an error
          </p>
          <button
            onClick={() => this.setState({ hasError: false })}
            className="text-xs px-3 py-1.5 rounded-lg transition-colors"
            style={{
              background: 'rgba(201,168,76,0.1)',
              border: '1px solid rgba(201,168,76,0.25)',
              color: '#c9a84c',
            }}
          >
            Retry
          </button>
        </div>
      )
    }
    return this.props.children
  }
}

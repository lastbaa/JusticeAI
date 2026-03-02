import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import './globals.css'
import { api } from './api'

// Expose api globally so all existing components work unchanged
;(window as unknown as Record<string, unknown>).api = api

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
)

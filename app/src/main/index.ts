import { app, BrowserWindow, ipcMain, dialog, shell } from 'electron'
import { join } from 'path'
import { readFileSync } from 'fs'
import { electronApp, optimizer, is } from '@electron-toolkit/utils'
import Store from 'electron-store'
import { readdirSync } from 'fs'
import { DocParser } from './services/docParser'
import { RagPipeline } from './services/ragPipeline'
import { OllamaService } from './services/ollama'
import { AppSettings, DEFAULT_SETTINGS, IPC, FileInfo, ChatSession } from '../../../shared/src/types'

// ── Services ────────────────────────────────────────────────────────────────
const store = new Store<{ settings: AppSettings }>({
  defaults: { settings: DEFAULT_SETTINGS },
})

// Encrypted store for chat history — data is never stored in plaintext
const chatStore = new Store<{ sessions: ChatSession[] }>({
  name: 'chat-history',
  encryptionKey: 'justice-ai-chat-v1-a8f3c2e9b4d7f1a6',
  defaults: { sessions: [] },
})

const ollamaService = new OllamaService()
const docParser = new DocParser()
const ragPipeline = new RagPipeline(ollamaService)

// Promise-based lock so concurrent IPC calls never double-initialize
let initPromise: Promise<void> | null = null

function ensureInitialized(): Promise<void> {
  if (!initPromise) {
    initPromise = (async () => {
      const settings = store.get('settings', DEFAULT_SETTINGS)
      await ragPipeline.initialize(settings)
    })()
  }
  return initPromise
}

// ── Window ──────────────────────────────────────────────────────────────────
function createWindow(): void {
  const mainWindow = new BrowserWindow({
    width: 1280,
    height: 800,
    minWidth: 900,
    minHeight: 600,
    show: false,
    titleBarStyle: 'hiddenInset',
    backgroundColor: '#080808',
    webPreferences: {
      preload: join(__dirname, '../preload/index.js'),
      sandbox: false,
      contextIsolation: true,
      nodeIntegration: false,
    },
  })

  mainWindow.on('ready-to-show', () => {
    mainWindow.show()
  })

  mainWindow.webContents.setWindowOpenHandler((details) => {
    shell.openExternal(details.url)
    return { action: 'deny' }
  })

  if (is.dev && process.env['ELECTRON_RENDERER_URL']) {
    mainWindow.loadURL(process.env['ELECTRON_RENDERER_URL'])
  } else {
    mainWindow.loadFile(join(__dirname, '../renderer/index.html'))
  }
}

// ── App lifecycle ────────────────────────────────────────────────────────────
app.whenReady().then(async () => {
  electronApp.setAppUserModelId('com.justiceai.app')

  app.on('browser-window-created', (_, window) => {
    optimizer.watchWindowShortcuts(window)
  })

  await ensureInitialized()
  createWindow()

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow()
    }
  })
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit()
  }
})

// ── IPC Handlers ─────────────────────────────────────────────────────────────

// Check Ollama status
ipcMain.handle(IPC.CHECK_OLLAMA, async () => {
  try {
    const settings = store.get('settings', DEFAULT_SETTINGS)
    return await ollamaService.checkStatus(
      settings.ollamaBaseUrl,
      'saul-7b-instruct',
      settings.embedModel
    )
  } catch (err) {
    console.error('CHECK_OLLAMA error:', err)
    return {
      running: false,
      models: [],
      hasLlmModel: false,
      hasEmbedModel: false,
      llmModelName: 'saul-7b-instruct',
      embedModelName: DEFAULT_SETTINGS.embedModel,
    }
  }
})

// Open file dialog
ipcMain.handle(IPC.OPEN_FILE_DIALOG, async () => {
  const result = await dialog.showOpenDialog({
    properties: ['openFile', 'multiSelections'],
    filters: [
      { name: 'Documents', extensions: ['pdf', 'docx'] },
      { name: 'PDF Files', extensions: ['pdf'] },
      { name: 'Word Documents', extensions: ['docx'] },
    ],
  })
  return result.canceled ? [] : result.filePaths
})

// Open folder dialog
ipcMain.handle(IPC.OPEN_FOLDER_DIALOG, async () => {
  const result = await dialog.showOpenDialog({
    properties: ['openDirectory'],
  })
  if (result.canceled || result.filePaths.length === 0) {
    return null
  }
  return result.filePaths[0]
})

// Load files: parse + chunk + embed
ipcMain.handle(IPC.LOAD_FILES, async (_event, filePaths: string[]) => {
  try {
    await ensureInitialized()
    const settings = store.get('settings', DEFAULT_SETTINGS)
    const results: FileInfo[] = []

    // Expand folders to individual files
    const expandedPaths: string[] = []
    for (const fp of filePaths) {
      try {
        const entries = readdirSync(fp)
        for (const entry of entries) {
          if (entry.toLowerCase().endsWith('.pdf') || entry.toLowerCase().endsWith('.docx')) {
            expandedPaths.push(join(fp, entry))
          }
        }
      } catch {
        // Not a directory — treat as file
        expandedPaths.push(fp)
      }
    }

    for (const filePath of expandedPaths) {
      try {
        const parsed = await docParser.parseFile(filePath)
        const fileInfo = await ragPipeline.addDocument(parsed, settings)
        results.push(fileInfo)
      } catch (err) {
        console.error(`Failed to load file ${filePath}:`, err)
      }
    }

    return results
  } catch (err) {
    console.error('LOAD_FILES error:', err)
    return []
  }
})

// Get current files
ipcMain.handle(IPC.GET_FILES, async () => {
  try {
    await ensureInitialized()
    return ragPipeline.getFiles()
  } catch (err) {
    console.error('GET_FILES error:', err)
    return []
  }
})

// Remove a file from the index
ipcMain.handle(IPC.REMOVE_FILE, async (_event, fileId: string) => {
  try {
    await ensureInitialized()
    await ragPipeline.removeDocument(fileId)
  } catch (err) {
    console.error('REMOVE_FILE error:', err)
  }
})

// Query: RAG pipeline
ipcMain.handle(IPC.QUERY, async (_event, question: string) => {
  try {
    await ensureInitialized()
    const settings = store.get('settings', DEFAULT_SETTINGS)
    return await ragPipeline.query(question, settings)
  } catch (err) {
    console.error('QUERY error:', err)
    return {
      answer: `Error processing your query: ${err instanceof Error ? err.message : String(err)}`,
      citations: [],
      notFound: true,
    }
  }
})

// Get settings
ipcMain.handle(IPC.GET_SETTINGS, async () => {
  return store.get('settings', DEFAULT_SETTINGS)
})

// Save settings
ipcMain.handle(IPC.SAVE_SETTINGS, async (_event, settings: AppSettings) => {
  store.set('settings', settings)
  // Reset the init lock so the pipeline re-initializes with new settings
  initPromise = null
  await ensureInitialized()
})

// ── Encrypted Chat History ────────────────────────────────────────────────────

// Save or update a session (upsert by id)
ipcMain.handle(IPC.SAVE_SESSION, async (_event, session: ChatSession) => {
  try {
    const sessions = chatStore.get('sessions', [])
    const idx = sessions.findIndex((s) => s.id === session.id)
    if (idx >= 0) {
      sessions[idx] = { ...session, updatedAt: Date.now() }
    } else {
      sessions.unshift({ ...session, updatedAt: Date.now() })
    }
    // Keep last 50 sessions
    chatStore.set('sessions', sessions.slice(0, 50))
    return true
  } catch (err) {
    console.error('SAVE_SESSION error:', err)
    return false
  }
})

// Get all sessions (metadata + messages, ordered newest first)
ipcMain.handle(IPC.GET_SESSIONS, async () => {
  try {
    return chatStore.get('sessions', [])
  } catch (err) {
    console.error('GET_SESSIONS error:', err)
    return []
  }
})

// Return a file's raw bytes as base64 (for PDF viewer in renderer)
ipcMain.handle(IPC.GET_FILE_DATA, (_event, filePath: string) => {
  try {
    const buf = readFileSync(filePath)
    return buf.toString('base64')
  } catch (err) {
    console.error('GET_FILE_DATA error:', err)
    throw new Error(`Could not read file: ${filePath}`)
  }
})

// Return the extracted text for a specific page (used by DOCX viewer)
ipcMain.handle(IPC.GET_PAGE_TEXT, async (_event, filePath: string, pageNumber: number) => {
  await ensureInitialized()
  return ragPipeline.getPageText(filePath, pageNumber)
})

// Delete a session by id
ipcMain.handle(IPC.DELETE_SESSION, async (_event, sessionId: string) => {
  try {
    const sessions = chatStore.get('sessions', [])
    chatStore.set('sessions', sessions.filter((s) => s.id !== sessionId))
    return true
  } catch (err) {
    console.error('DELETE_SESSION error:', err)
    return false
  }
})

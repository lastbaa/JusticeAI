import { describe, it, expect, vi, beforeEach } from 'vitest'

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}))
vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}))

import { invoke } from '@tauri-apps/api/core'
import { api } from '../api'

describe('api shim', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('getSettings invokes correct command', async () => {
    const mockSettings = { chunkSize: 1000, chunkOverlap: 150, topK: 6, theme: 'dark' }
    vi.mocked(invoke).mockResolvedValue(mockSettings)
    const result = await api.getSettings()
    expect(invoke).toHaveBeenCalledWith('get_settings')
    expect(result).toEqual(mockSettings)
  })

  it('getFiles invokes correct command', async () => {
    vi.mocked(invoke).mockResolvedValue([])
    const result = await api.getFiles()
    expect(invoke).toHaveBeenCalledWith('get_files')
    expect(result).toEqual([])
  })

  it('checkModels invokes correct command', async () => {
    vi.mocked(invoke).mockResolvedValue({ llmReady: true, llmSizeGb: 4.5, downloadRequiredGb: 0, ocrReady: false })
    const result = await api.checkModels()
    expect(invoke).toHaveBeenCalledWith('check_models')
    expect(result).toEqual({ llmReady: true, llmSizeGb: 4.5, downloadRequiredGb: 0, ocrReady: false })
  })

  it('loadFiles invokes with correct arguments', async () => {
    vi.mocked(invoke).mockResolvedValue([])
    await api.loadFiles(['/path/to/file.pdf'])
    expect(invoke).toHaveBeenCalledWith('load_files', { filePaths: ['/path/to/file.pdf'], caseId: null })
  })

  it('removeFile invokes with fileId', async () => {
    vi.mocked(invoke).mockResolvedValue(undefined)
    await api.removeFile('file-123')
    expect(invoke).toHaveBeenCalledWith('remove_file', { fileId: 'file-123' })
  })

  it('saveSettings invokes with settings payload', async () => {
    const settings = { chunkSize: 1000, chunkOverlap: 150, topK: 6, theme: 'dark' as const }
    vi.mocked(invoke).mockResolvedValue(undefined)
    await api.saveSettings(settings)
    expect(invoke).toHaveBeenCalledWith('save_settings', { settings })
  })

  it('deleteSession invokes with sessionId', async () => {
    vi.mocked(invoke).mockResolvedValue(true)
    const result = await api.deleteSession('sess-1')
    expect(invoke).toHaveBeenCalledWith('delete_session', { sessionId: 'sess-1' })
    expect(result).toBe(true)
  })
})

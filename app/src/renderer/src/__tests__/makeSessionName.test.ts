import { describe, it, expect } from 'vitest'
import { makeSessionName } from '../utils/sessionName'
import { ChatMessage } from '../../../../../../shared/src/types'

function userMsg(content: string): ChatMessage[] {
  return [{ id: '1', role: 'user', content, timestamp: Date.now() }]
}

describe('makeSessionName', () => {
  it('returns "New Chat" for empty messages', () => {
    expect(makeSessionName([])).toBe('New Chat')
  })

  it('returns "New Chat" for assistant-only messages', () => {
    expect(makeSessionName([{ id: '1', role: 'assistant', content: 'Hello', timestamp: Date.now() }])).toBe('New Chat')
  })

  it('strips greeting words', () => {
    const result = makeSessionName(userMsg('Hey, can you explain the lease agreement?'))
    expect(result).not.toMatch(/^Hey/)
    expect(result.toLowerCase()).toContain('lease agreement')
  })

  it('strips action verbs', () => {
    const result = makeSessionName(userMsg('Can you summarize the contract?'))
    expect(result).not.toMatch(/^Can you/)
    expect(result.toLowerCase()).toContain('contract')
  })

  it('capitalizes first letter', () => {
    const result = makeSessionName(userMsg('lease agreement details'))
    expect(result[0]).toBe(result[0].toUpperCase())
  })

  it('truncates long names with ellipsis', () => {
    const longText = 'explain the comprehensive analysis of the multi-party settlement agreement and its implications'
    const result = makeSessionName(userMsg(longText))
    expect(result.length).toBeLessThanOrEqual(44) // 40 + ellipsis char + margin
  })

  it('removes trailing question marks', () => {
    const result = makeSessionName(userMsg('What is a tort???'))
    expect(result).not.toMatch(/\?/)
  })

  it('strips file paths', () => {
    const result = makeSessionName(userMsg('analyze /Users/test/document.pdf'))
    expect(result).not.toContain('/Users')
  })

  it('strips URLs', () => {
    const result = makeSessionName(userMsg('check https://example.com for info'))
    expect(result).not.toContain('https://')
  })

  it('handles legal abbreviations', () => {
    const result = makeSessionName(userMsg('Smith v. Jones case'))
    expect(result).toContain('v.')
  })

  it('returns "New Chat" for whitespace-only content', () => {
    expect(makeSessionName(userMsg('   '))).toBe('New Chat')
  })
})

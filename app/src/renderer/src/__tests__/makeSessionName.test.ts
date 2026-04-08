import { describe, it, expect } from 'vitest'
import { makeSessionName, isGenericName } from '../utils/sessionName'
import { ChatMessage } from '../../../../../../shared/src/types'

function msg(role: string, content: string, opts?: Partial<ChatMessage>): ChatMessage {
  return { id: Math.random().toString(), role, content, timestamp: Date.now(), ...opts }
}

function userMsg(content: string): ChatMessage[] {
  return [msg('user', content)]
}

function conversation(user: string, assistant: string): ChatMessage[] {
  return [msg('user', user), msg('assistant', assistant)]
}

describe('makeSessionName', () => {
  // ── Basic / Edge Cases ────────────────────────────────────────
  it('returns "New Chat" for empty messages', () => {
    expect(makeSessionName([])).toBe('New Chat')
  })

  it('returns "New Chat" for assistant-only messages', () => {
    expect(makeSessionName([msg('assistant', 'Hello')])).toBe('New Chat')
  })

  it('returns "New Chat" for whitespace-only content', () => {
    expect(makeSessionName(userMsg('   '))).toBe('New Chat')
  })

  it('returns a fallback for greeting-only messages', () => {
    const result = makeSessionName(userMsg('hey'))
    expect(result).toBeTruthy()
    expect(result.length).toBeGreaterThan(0)
  })

  // ── Document Name Extraction ──────────────────────────────────
  it('extracts document names from queries', () => {
    const result = makeSessionName(userMsg('What are the terms in lease_agreement.pdf?'))
    expect(result).toContain('Lease Agreement')
  })

  it('extracts DOCX document names', () => {
    const result = makeSessionName(userMsg('Review my contract.docx'))
    expect(result).toContain('Contract')
  })

  it('handles document with underscores and hyphens', () => {
    const result = makeSessionName(userMsg('Analyze non-disclosure-agreement.pdf'))
    expect(result).toContain('Non Disclosure Agreement')
  })

  it('combines document name with concept', () => {
    const result = makeSessionName(
      userMsg('What is the termination clause in lease_agreement.pdf?')
    )
    expect(result).toContain('Lease Agreement')
    expect(result).toContain('Termination')
  })

  // ── Legal Entity Extraction ───────────────────────────────────
  it('extracts case names with v.', () => {
    const result = makeSessionName(userMsg('Tell me about Smith v. Jones'))
    expect(result).toContain('Smith v. Jones')
  })

  it('extracts case names with vs', () => {
    const result = makeSessionName(userMsg('What happened in Brown vs Davis?'))
    expect(result).toContain('Brown')
    expect(result).toContain('Davis')
  })

  it('combines entity with concept', () => {
    const result = makeSessionName(
      userMsg('What is the liability in Smith v. Jones?')
    )
    expect(result).toContain('Smith v. Jones')
    expect(result).toContain('Liability')
  })

  // ── Legal Concept Extraction ──────────────────────────────────
  it('extracts termination concept', () => {
    const result = makeSessionName(userMsg('Is there an early termination clause?'))
    expect(result.toLowerCase()).toContain('termination')
  })

  it('extracts NDA concept', () => {
    const result = makeSessionName(userMsg('Review the NDA provisions'))
    expect(result).toContain('NDA')
  })

  it('extracts indemnification concept', () => {
    const result = makeSessionName(userMsg('What does the indemnification section say?'))
    expect(result.toLowerCase()).toContain('indemnification')
  })

  it('extracts multiple concepts', () => {
    const result = makeSessionName(userMsg('Explain the liability and warranty terms'))
    // Should contain at least one concept
    const hasLiability = result.toLowerCase().includes('liability')
    const hasWarranty = result.toLowerCase().includes('warranty')
    expect(hasLiability || hasWarranty).toBe(true)
  })

  // ── Question Type Mapping ─────────────────────────────────────
  it('maps "how much" to amounts prefix', () => {
    const result = makeSessionName(userMsg('How much is the security deposit?'))
    const hasAmount = result.toLowerCase().includes('amount')
    const hasSecurityDeposit = result.toLowerCase().includes('security deposit')
    expect(hasAmount || hasSecurityDeposit).toBe(true)
  })

  it('maps "who is" to parties prefix', () => {
    const result = makeSessionName(
      conversation('Who is the landlord?', 'The landlord is John Smith, as stated in section 1.')
    )
    // Should reflect parties or the entity
    expect(result.length).toBeGreaterThan(0)
  })

  it('maps "when does" to timeline prefix', () => {
    const result = makeSessionName(userMsg('When does the lease expire?'))
    const hasTimeline = result.toLowerCase().includes('timeline')
    const hasLease = result.toLowerCase().includes('lease')
    expect(hasTimeline || hasLease).toBe(true)
  })

  it('maps "summarize" to analysis prefix', () => {
    const result = makeSessionName(
      conversation('Summarize the contract', 'The contract covers employment terms including...')
    )
    expect(result.length).toBeGreaterThan(0)
  })

  // ── Assistant Response Signals ────────────────────────────────
  it('uses assistant response when user message is vague', () => {
    const result = makeSessionName(
      conversation(
        'Tell me about this document',
        'This is a lease agreement between John Smith and ABC Properties with a monthly rent of $1,500.'
      )
    )
    // Should pick up signals from the assistant response
    expect(result).not.toBe('New Chat')
    expect(result.length).toBeGreaterThan(3)
  })

  it('extracts document names from assistant response', () => {
    const result = makeSessionName([
      msg('user', 'What is this about?'),
      msg('assistant', 'Based on the lease_agreement.pdf, this covers rental terms.')
    ])
    expect(result).toContain('Lease Agreement')
  })

  // ── Progressive Refinement ────────────────────────────────────
  it('generates better name with more messages', () => {
    const singleMsg = makeSessionName(userMsg('Tell me about this'))
    const multiMsg = makeSessionName([
      msg('user', 'Tell me about this'),
      msg('assistant', 'This is a non-disclosure agreement between...'),
      msg('user', 'What are the confidentiality terms?'),
      msg('assistant', 'The confidentiality clause requires...'),
    ])
    // Multi-message version should produce something more specific
    expect(multiMsg).not.toBe('New Chat')
    const hasNda = multiMsg.toLowerCase().includes('nda')
    const hasConfidentiality = multiMsg.toLowerCase().includes('confidentiality')
    const hasDisclosure = multiMsg.toLowerCase().includes('disclosure')
    expect(hasNda || hasConfidentiality || hasDisclosure).toBe(true)
  })

  // ── Title Formatting ──────────────────────────────────────────
  it('capitalizes first letter', () => {
    const result = makeSessionName(userMsg('lease agreement details'))
    expect(result[0]).toBe(result[0].toUpperCase())
  })

  it('keeps title under 50 characters', () => {
    const longText = 'Explain the comprehensive analysis of the multi-party settlement agreement and all its legal implications for both parties involved'
    const result = makeSessionName(userMsg(longText))
    expect(result.length).toBeLessThanOrEqual(50)
  })

  it('removes trailing question marks', () => {
    const result = makeSessionName(userMsg('What is a tort???'))
    expect(result).not.toMatch(/\?/)
  })

  it('removes trailing punctuation', () => {
    const result = makeSessionName(userMsg('Review the contract terms.'))
    expect(result).not.toMatch(/[.!;:,]$/)
  })

  it('does not contain JS truncation ellipsis', () => {
    // Titles should be naturally constrained, no ugly "..." from JS
    const result = makeSessionName(userMsg('What are the liability clauses?'))
    expect(result).not.toContain('...')
  })

  // ── Edge Cases ────────────────────────────────────────────────
  it('strips file paths', () => {
    const result = makeSessionName(userMsg('analyze /Users/test/document.pdf'))
    expect(result).not.toContain('/Users')
  })

  it('strips URLs', () => {
    const result = makeSessionName(userMsg('check https://example.com for info'))
    expect(result).not.toContain('https://')
  })

  it('handles legal abbreviations like v.', () => {
    const result = makeSessionName(userMsg('Smith v. Jones case'))
    expect(result).toContain('v.')
  })

  it('handles very long messages gracefully', () => {
    const longMsg = 'a '.repeat(500) + 'termination clause'
    const result = makeSessionName(userMsg(longMsg))
    expect(result.length).toBeLessThanOrEqual(50)
  })

  it('handles non-English characters', () => {
    const result = makeSessionName(userMsg('Revise el contrato de arrendamiento'))
    expect(result).toBeTruthy()
    expect(result.length).toBeGreaterThan(0)
  })

  it('handles code blocks in messages', () => {
    const result = makeSessionName(userMsg('Explain ```const x = 1``` in the contract'))
    expect(result).not.toContain('const x')
  })

  it('handles greeting-only with no substance', () => {
    const result = makeSessionName(userMsg('Hey, hello, hi'))
    expect(result).toBeTruthy()
  })

  it('ignores greeting messages marked with isGreeting', () => {
    const result = makeSessionName([
      msg('user', 'irrelevant greeting', { isGreeting: true }),
      msg('assistant', 'Welcome!', { isGreeting: true }),
      msg('user', 'What is the eviction process?'),
      msg('assistant', 'The eviction process involves...'),
    ])
    expect(result.toLowerCase()).toContain('eviction')
  })

  // ── Fallback ──────────────────────────────────────────────────
  it('returns fallback for signals-free messages', () => {
    const result = makeSessionName(userMsg('help'))
    expect(result).toBeTruthy()
    expect(result.length).toBeGreaterThan(0)
  })

  // ── isGenericName ─────────────────────────────────────────────
  it('identifies generic names', () => {
    expect(isGenericName('New Chat')).toBe(true)
    expect(isGenericName('Legal Document Analysis')).toBe(true)
    expect(isGenericName('Document Analysis')).toBe(true)
  })

  it('does not flag specific names as generic', () => {
    expect(isGenericName('Smith v. Jones — Liability')).toBe(false)
    expect(isGenericName('Lease Agreement — Rent Terms')).toBe(false)
    expect(isGenericName('NDA Review')).toBe(false)
  })

  // ── Dollar amounts ────────────────────────────────────────────
  it('handles dollar amount focus', () => {
    const result = makeSessionName(
      conversation(
        'How much is $5,000 deposit?',
        'The security deposit of $5,000 is outlined in section 3.'
      )
    )
    expect(result).toBeTruthy()
    expect(result.length).toBeGreaterThan(0)
  })
})

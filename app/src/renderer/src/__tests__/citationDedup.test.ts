import { describe, it, expect } from 'vitest'
import { deduplicateCitations } from '../utils/citations'
import { Citation } from '../../../../../../shared/src/types'

function makeCitation(fileName: string, pageNumber: number, score: number): Citation {
  return { fileName, filePath: `/docs/${fileName}`, pageNumber, score, excerpt: `chunk from ${fileName} p${pageNumber}` }
}

describe('deduplicateCitations', () => {
  it('returns empty for no citations', () => {
    const { unique, hasDuplicates } = deduplicateCitations([])
    expect(unique).toEqual([])
    expect(hasDuplicates).toBe(false)
  })

  it('keeps all citations when no duplicates', () => {
    const citations = [
      makeCitation('a.pdf', 1, 0.9),
      makeCitation('a.pdf', 2, 0.8),
      makeCitation('b.pdf', 1, 0.7),
    ]
    const { unique, hasDuplicates } = deduplicateCitations(citations)
    expect(unique).toHaveLength(3)
    expect(hasDuplicates).toBe(false)
  })

  it('deduplicates by file+page, keeping first (highest score)', () => {
    const citations = [
      makeCitation('a.pdf', 1, 0.95),
      makeCitation('a.pdf', 1, 0.85),
      makeCitation('a.pdf', 1, 0.75),
    ]
    const { unique, hasDuplicates } = deduplicateCitations(citations)
    expect(unique).toHaveLength(1)
    expect(unique[0].score).toBe(0.95)
    expect(hasDuplicates).toBe(true)
  })

  it('handles mixed duplicates and unique', () => {
    const citations = [
      makeCitation('a.pdf', 1, 0.9),
      makeCitation('b.pdf', 1, 0.85),
      makeCitation('a.pdf', 1, 0.7),
      makeCitation('b.pdf', 2, 0.6),
    ]
    const { unique, hasDuplicates } = deduplicateCitations(citations)
    expect(unique).toHaveLength(3)
    expect(hasDuplicates).toBe(true)
  })
})

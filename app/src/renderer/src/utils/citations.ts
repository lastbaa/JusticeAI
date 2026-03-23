import { Citation } from '../../../../../shared/src/types'

/**
 * Deduplicate citations by page -- keep highest-scored citation per file+page.
 * Assumes citations are sorted descending by score (first seen = highest).
 */
export function deduplicateCitations(citations: Citation[]): {
  unique: Citation[]
  hasDuplicates: boolean
} {
  const seen = new Map<string, Citation>()
  for (const c of citations) {
    const key = `${c.fileName}::${c.pageNumber}`
    if (!seen.has(key)) {
      seen.set(key, c)
    }
    // Already sorted descending by score, so first seen = highest
  }
  const unique = Array.from(seen.values())
  return { unique, hasDuplicates: unique.length < citations.length }
}

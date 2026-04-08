import { ChatMessage } from '../../../../../shared/src/types'

// ── Legal concepts ──────────────────────────────────────────────
const LEGAL_CONCEPTS: [RegExp, string][] = [
  [/\beviction\b/i, 'Eviction'],
  [/\btermination\s*(clause|provision|right|fee)?\b/i, 'Termination'],
  [/\bnon-?disclosure|NDA\b/i, 'NDA'],
  [/\bindemnif(y|ication|ied)\b/i, 'Indemnification'],
  [/\bbreach\b/i, 'Breach'],
  [/\bliabilit(y|ies)\b/i, 'Liability'],
  [/\bnegligen(ce|t)\b/i, 'Negligence'],
  [/\bforce\s*majeure\b/i, 'Force Majeure'],
  [/\barbitration\b/i, 'Arbitration'],
  [/\bconfidentialit(y|ies)\b/i, 'Confidentiality'],
  [/\bnon-?compete\b/i, 'Non-Compete'],
  [/\bnon-?solicit(ation)?\b/i, 'Non-Solicitation'],
  [/\bintellectual\s*property|IP\s+rights?\b/i, 'Intellectual Property'],
  [/\bwarrant(y|ies)\b/i, 'Warranty'],
  [/\bsecurity\s+deposit\b/i, 'Security Deposit'],
  [/\brent(al)?\s*(amount|increase|payment|term)?\b/i, 'Rent'],
  [/\blease\s*(term|agreement|renewal|period)?\b/i, 'Lease'],
  [/\bsettlement\b/i, 'Settlement'],
  [/\bdamage(s)?\b/i, 'Damages'],
  [/\bcustody\b/i, 'Custody'],
  [/\balimony|spousal\s+support\b/i, 'Alimony'],
  [/\bchild\s+support\b/i, 'Child Support'],
  [/\bbankruptcy\b/i, 'Bankruptcy'],
  [/\bforeclosure\b/i, 'Foreclosure'],
  [/\bpower\s+of\s+attorney\b/i, 'Power of Attorney'],
  [/\bdue\s+diligence\b/i, 'Due Diligence'],
  [/\bcompliance\b/i, 'Compliance'],
  [/\bemployment\s*(agreement|contract|law)?\b/i, 'Employment'],
  [/\bwrongful\s+termination\b/i, 'Wrongful Termination'],
  [/\bdefamation\b/i, 'Defamation'],
  [/\bcopyright\b/i, 'Copyright'],
  [/\btrademark\b/i, 'Trademark'],
  [/\bpatent\b/i, 'Patent'],
  [/\bfraud\b/i, 'Fraud'],
  [/\bmisrepresentation\b/i, 'Misrepresentation'],
  [/\bstatute\s+of\s+limitations?\b/i, 'Statute of Limitations'],
  [/\bjurisdiction\b/i, 'Jurisdiction'],
  [/\bvenue\b/i, 'Venue'],
  [/\bgoverning\s+law\b/i, 'Governing Law'],
  [/\binsurance\b/i, 'Insurance'],
  [/\bclaim(s)?\b/i, 'Claims'],
]

// ── Question type to prefix mapping ─────────────────────────────
const QUESTION_PREFIXES: [RegExp, string][] = [
  [/^who\s+(is|are|was|were)\b/i, 'Parties in'],
  [/^what\s+(is|are|was|were)\s+(the|a|an)\s+(\w+)/i, ''],  // handled dynamically
  [/^how\s+much\b/i, 'Amounts in'],
  [/^when\s+(does|did|is|was|will)\b/i, 'Timeline for'],
  [/^(summarize|analyze|review|explain)\b/i, 'Analysis of'],
  [/^(compare|contrast)\b/i, 'Comparison of'],
  [/^(is\s+there|are\s+there|does\s+it\s+have)\b/i, ''],
  [/^(list|identify|find)\b/i, ''],
]

// ── Document name extraction ────────────────────────────────────
const FILE_PATTERN = /[\w-]+\.(pdf|docx?|txt|csv|xlsx?|html?|eml|md)\b/gi

function extractDocNames(text: string): string[] {
  const matches = text.match(FILE_PATTERN)
  if (!matches) return []
  return [...new Set(matches)].map((f) => {
    // "lease_agreement.pdf" → "Lease Agreement"
    const base = f.replace(/\.\w+$/, '')
    return base
      .replace(/[_-]+/g, ' ')
      .replace(/\b\w/g, (c) => c.toUpperCase())
      .trim()
  })
}

// ── Entity extraction ───────────────────────────────────────────
// Matches "Smith v. Jones", "Smith vs Jones", "Smith vs. Jones"
const VS_PATTERN = /([A-Z][a-z]+(?:\s+[A-Z][a-z]+)*)\s+v\.?\s*s?\.?\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+)*)/g

// Matches capitalized proper nouns (2+ word sequences)
const PROPER_NOUN_PATTERN = /\b([A-Z][a-z]+(?:\s+[A-Z][a-z]+)+)\b/g

// Common words that look like proper nouns but aren't
const FALSE_POSITIVES = new Set([
  'The', 'This', 'That', 'These', 'Those', 'What', 'When', 'Where', 'Which',
  'How', 'Who', 'New Chat', 'Legal Document', 'Document Analysis', 'Legal Question',
  'Can You', 'Could You', 'Would You', 'Will You', 'Please Tell', 'I Want',
  'Let Me', 'Help Me', 'Tell Me', 'Go Ahead', 'New York', 'Los Angeles',
  'San Francisco', 'United States',
])

function extractEntities(text: string): string[] {
  const entities: string[] = []

  // Case names (Smith v. Jones)
  let vsMatch: RegExpExecArray | null
  const vsRegex = new RegExp(VS_PATTERN.source, VS_PATTERN.flags)
  while ((vsMatch = vsRegex.exec(text)) !== null) {
    entities.push(`${vsMatch[1]} v. ${vsMatch[2]}`)
  }

  return entities
}

function extractProperNouns(text: string): string[] {
  const nouns: string[] = []
  let match: RegExpExecArray | null
  const regex = new RegExp(PROPER_NOUN_PATTERN.source, PROPER_NOUN_PATTERN.flags)
  while ((match = regex.exec(text)) !== null) {
    const noun = match[1]
    if (!FALSE_POSITIVES.has(noun)) {
      nouns.push(noun)
    }
  }
  return nouns
}

// ── Dollar amounts ──────────────────────────────────────────────
const DOLLAR_PATTERN = /\$[\d,]+(?:\.\d{2})?/g

function extractDollarAmounts(text: string): string[] {
  return text.match(DOLLAR_PATTERN) ?? []
}

// ── Concept extraction ──────────────────────────────────────────
function extractConcepts(text: string): string[] {
  const found: string[] = []
  for (const [pattern, label] of LEGAL_CONCEPTS) {
    if (pattern.test(text)) {
      found.push(label)
    }
  }
  return found
}

// ── Question prefix ─────────────────────────────────────────────
function getQuestionPrefix(text: string): string {
  const trimmed = text.trim()
  for (const [pattern, prefix] of QUESTION_PREFIXES) {
    if (pattern.test(trimmed)) {
      if (prefix) return prefix
    }
  }
  return ''
}

// ── Title case helper ───────────────────────────────────────────
const LOWERCASE_WORDS = new Set(['a', 'an', 'the', 'and', 'but', 'or', 'for', 'nor', 'in', 'on', 'at', 'to', 'of', 'by', 'with', 'is', 'are', 'was', 'were'])

// Words that should keep their original casing (acronyms, abbreviations)
const PRESERVE_CASE = new Set(['NDA', 'IP', 'LLC', 'IRS', 'CEO', 'CFO', 'HIPAA', 'GDPR', 'ADA', 'OSHA', 'EEOC', 'DOJ', 'SEC', 'FTC', 'FDA', 'EPA', 'ERISA', 'COBRA', 'FMLA', 'FLSA', 'IRS', 'USCIS'])

function toTitleCase(str: string): string {
  return str
    .split(/\s+/)
    .map((word, i) => {
      // Preserve known acronyms
      if (PRESERVE_CASE.has(word.toUpperCase())) return word.toUpperCase()
      if (i === 0) return word.charAt(0).toUpperCase() + word.slice(1).toLowerCase()
      if (LOWERCASE_WORDS.has(word.toLowerCase())) return word.toLowerCase()
      return word.charAt(0).toUpperCase() + word.slice(1).toLowerCase()
    })
    .join(' ')
}

// ── Generic name patterns (for progressive refinement) ──────────
const GENERIC_NAMES = [
  'New Chat',
  'Legal Document Analysis',
  'Document Analysis',
  'Legal Question',
  'Document Review',
  'Legal Research',
]

export function isGenericName(name: string): boolean {
  return GENERIC_NAMES.some((g) => g.toLowerCase() === name.toLowerCase())
}

// ── Main title generator ────────────────────────────────────────
/**
 * Generate a concise chat title from the conversation messages.
 * Uses multiple signals: document names, legal entities, concepts,
 * question types, and assistant response content.
 *
 * Returns a 3-8 word Title Case string, max ~50 chars.
 */
export function makeSessionName(messages: ChatMessage[]): string {
  const userMsgs = messages.filter((m) => m.role === 'user' && !m.isGreeting)
  const assistantMsgs = messages.filter((m) => m.role === 'assistant' && !m.isGreeting)

  if (userMsgs.length === 0) return 'New Chat'

  // Combine text from messages for signal extraction
  const firstUser = userMsgs[0].content.trim()
  const firstAssistant = assistantMsgs[0]?.content.trim() ?? ''

  // For progressive refinement, consider more messages
  const userText = userMsgs.slice(0, 3).map((m) => m.content).join(' ')
  const assistantText = assistantMsgs.slice(0, 3).map((m) => m.content).join(' ')
  const allText = userText + ' ' + assistantText

  if (!firstUser) return 'New Chat'

  // ── Extract signals ─────────────────────────────────────────
  const docNames = extractDocNames(allText)
  const entities = extractEntities(allText)
  const properNouns = extractProperNouns(firstUser + ' ' + firstAssistant)
  const concepts = extractConcepts(allText)
  const dollars = extractDollarAmounts(firstUser)
  const questionPrefix = getQuestionPrefix(firstUser)

  // ── Build title from signals ────────────────────────────────
  let title = ''

  // Priority 1: Entity + concept (e.g., "Smith v. Jones — Liability")
  if (entities.length > 0 && concepts.length > 0) {
    title = `${entities[0]} — ${concepts[0]}`
  }
  // Priority 2: Document + concept (e.g., "Lease Agreement — Rent Terms")
  else if (docNames.length > 0 && concepts.length > 0) {
    title = `${docNames[0]} — ${concepts[0]}`
  }
  // Priority 3: Document only (e.g., "Lease Agreement Review")
  else if (docNames.length > 0) {
    title = questionPrefix
      ? `${questionPrefix} ${docNames[0]}`
      : `${docNames[0]} Review`
  }
  // Priority 4: Entity only (e.g., "Smith v. Jones")
  else if (entities.length > 0) {
    title = entities[0]
  }
  // Priority 5: Question prefix + concept (e.g., "Amounts in Settlement")
  else if (questionPrefix && concepts.length > 0) {
    title = `${questionPrefix} ${concepts[0]}`
  }
  // Priority 6: Just concept(s) (e.g., "Early Termination Clause")
  else if (concepts.length > 0) {
    title = concepts.length > 1
      ? `${concepts[0]} and ${concepts[1]}`
      : concepts[0]
  }
  // Priority 7: Dollar amounts with context
  else if (dollars.length > 0 && concepts.length === 0) {
    title = `${dollars[0]} Payment Analysis`
  }
  // Priority 8: Proper nouns from conversation
  else if (properNouns.length > 0) {
    title = properNouns[0]
  }
  // Priority 9: Fallback — extract key phrase from user message
  else {
    title = extractKeyPhrase(firstUser)
  }

  // ── Clean up and constrain ──────────────────────────────────
  if (!title || title.trim().length === 0) {
    return 'Legal Document Analysis'
  }

  // Apply title case (but preserve "v." in case names)
  if (!title.includes(' v. ') && !title.includes(' — ')) {
    title = toTitleCase(title)
  }

  // Remove trailing punctuation
  title = title.replace(/[.!?;:,]+$/, '').trim()

  // Ensure max ~50 chars (cut at word boundary)
  if (title.length > 50) {
    const cut = title.lastIndexOf(' ', 48)
    title = title.slice(0, cut > 20 ? cut : 48).trim()
  }

  // Capitalize first char (safety)
  if (title.length > 0) {
    title = title.charAt(0).toUpperCase() + title.slice(1)
  }

  return title || 'Legal Document Analysis'
}

// ── Fallback key phrase extraction ──────────────────────────────
function extractKeyPhrase(text: string): string {
  // Strip code, URLs, paths
  let cleaned = text
    .replace(/```[\s\S]*?```/g, '')
    .replace(/`[^`]+`/g, '')
    .replace(/https?:\/\/\S+/g, '')
    .replace(/\/[\w./\\-]+/g, '')
    .trim()

  // Use first line only
  const firstLine = cleaned.split('\n').filter(Boolean)[0]?.trim() ?? cleaned
  let phrase = firstLine

  // Strip greeting / filler words
  const GREETINGS = /^(hey|hi|hello|ok|okay|so|well|um|uh|basically|actually),?\s*/i
  const FILLER = /^(can you|could you|would you|will you|please|i want to|i need to|i'd like to|help me|i want you to|let's|let me|go ahead and|tell me about|what is|what are|explain|summarize|understand|know about|describe|review|analyze|look at|check)\s+/i
  const ARTICLES = /^(the|a|an)\s+/i

  let prev = ''
  while (phrase !== prev) {
    prev = phrase
    phrase = phrase.replace(GREETINGS, '')
    phrase = phrase.replace(FILLER, '')
  }
  phrase = phrase.replace(ARTICLES, '')
  phrase = phrase.replace(/\?+$/, '').trim()

  // Capitalize first letter
  if (phrase.length > 0) {
    phrase = phrase.charAt(0).toUpperCase() + phrase.slice(1)
  }

  // Truncate at word boundary around 45 chars
  if (phrase.length > 48) {
    const cut = phrase.lastIndexOf(' ', 45)
    phrase = phrase.slice(0, cut > 20 ? cut : 45).trim()
  }

  return phrase || 'Legal Document Analysis'
}

/**
 * Build a short summary from the first few user messages.
 */
export function makeSessionSummary(messages: ChatMessage[]): string {
  const userMsgs = messages.filter((m) => m.role === 'user').slice(0, 3)
  return userMsgs.map((m) => m.content.trim().slice(0, 80)).join('; ')
}

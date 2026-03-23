import { ChatMessage } from '../../../../../shared/src/types'

/**
 * Generate a concise chat title from the first user message.
 * Short noun-phrase titles like "Lease Agreement Questions" or "W-9 Form Details".
 */
export function makeSessionName(messages: ChatMessage[]): string {
  const first = messages.find((m) => m.role === 'user')
  if (!first) return 'New Chat'

  let text = first.content.trim()
  if (!text) return 'New Chat'

  // Strip file paths, URLs, code blocks
  text = text.replace(/```[\s\S]*?```/g, '').replace(/`[^`]+`/g, '')
  text = text.replace(/https?:\/\/\S+/g, '').replace(/\/[\w./\\-]+/g, '')
  text = text.trim()

  // Use the first line only (don't split on dots — legal text has "v.", "Mr.", "3.2", "U.S." etc.)
  const firstLine = text.split('\n').filter(Boolean)[0]?.trim() ?? text
  let phrase = firstLine

  // Strip leading filler — loop so "Can you please explain" cascades correctly.
  const GREETINGS = /^(hey|hi|hello|ok|okay|so|well|um|uh|basically|actually),?\s*/i
  const FILLER = /^(can you|could you|would you|will you|please|i want to|i need to|i'd like to|help me|i want you to|let's|let me|go ahead and|tell me about|what is|what are|explain|summarize|understand|know about|describe|review|analyze|look at|check)\s+/i
  const ARTICLES = /^(the|a|an)\s+/i
  let prev = ''
  while (phrase !== prev) {
    prev = phrase
    phrase = phrase.replace(GREETINGS, '')
    phrase = phrase.replace(FILLER, '')
  }

  // Strip leading articles (after all filler is gone)
  phrase = phrase.replace(ARTICLES, '')

  // Remove trailing question mark for cleaner titles
  phrase = phrase.replace(/\?+$/, '').trim()

  // Capitalize first letter
  if (phrase.length > 0) {
    phrase = phrase.charAt(0).toUpperCase() + phrase.slice(1)
  }

  // Truncate at a word boundary around 40 chars (fits 240px sidebar at 12px)
  if (phrase.length > 43) {
    const cut = phrase.lastIndexOf(' ', 40)
    phrase = phrase.slice(0, cut > 20 ? cut : 40) + '\u2026'
  }

  return phrase || 'New Chat'
}

/**
 * Build a short summary from the first few user messages.
 */
export function makeSessionSummary(messages: ChatMessage[]): string {
  const userMsgs = messages.filter((m) => m.role === 'user').slice(0, 3)
  return userMsgs.map((m) => m.content.trim().slice(0, 80)).join('; ')
}

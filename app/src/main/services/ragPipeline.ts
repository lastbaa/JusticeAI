import { LocalIndex, MetadataTypes } from 'vectra'
import { join } from 'path'
import { app } from 'electron'
import { v4 as uuidv4 } from 'uuid'
import {
  ParsedDocument,
  DocumentChunk,
  FileInfo,
  Citation,
  QueryResult,
  AppSettings,
} from '../../../../shared/src/types'
import { OllamaService } from './ollama'
import { askSaul } from './saul'

const SYSTEM_PROMPT = `You are Justice AI, a secure legal research assistant designed for legal professionals.

Your only job is to help the user find information within the documents they have loaded. You are NOT providing legal advice. You are a research and retrieval tool to support the legal professional using you.

Rules you must never break:
1. Answer ONLY using the document excerpts provided in the context below.
2. Always cite the exact filename and page number for every claim you make.
3. Always include a direct quoted excerpt from the source document to support your answer.
4. If the answer cannot be found in the provided documents, respond only with: "I could not find information about this in your loaded documents. Please ensure the relevant files are loaded."
5. Never use pretrained knowledge to fill gaps. Never guess. Never hallucinate.
6. Never provide legal advice or legal conclusions. If asked for a legal opinion, remind the user that Justice AI is a research tool and that legal conclusions are theirs to make.

Context from loaded documents:
{context}`

// Minimum similarity score to include a chunk (0–1 scale)
const SCORE_THRESHOLD = 0.35

// Max chunks from any single (file, page) pair to ensure diversity
const MAX_CHUNKS_PER_PAGE = 2

interface ChunkMetadata extends DocumentChunk {
  itemId: string
}

export class RagPipeline {
  private index: LocalIndex | null = null
  private fileRegistry = new Map<string, FileInfo>()
  private chunkRegistry = new Map<string, ChunkMetadata>()
  private docChunkIds = new Map<string, string[]>()
  private ollamaService: OllamaService
  private indexPath: string

  constructor(ollamaService: OllamaService) {
    this.ollamaService = ollamaService
    this.indexPath = join(app.getPath('userData'), 'vector-index')
  }

  async initialize(_settings: AppSettings): Promise<void> {
    this.index = new LocalIndex(this.indexPath)
    const exists = await this.index.isIndexCreated()
    if (!exists) {
      await this.index.createIndex()
    }

    try {
      const allItems = await this.index.listItems()
      const docMap = new Map<string, { meta: ChunkMetadata; count: number }>()

      for (const item of allItems) {
        const meta = item.metadata as unknown as ChunkMetadata
        if (meta && meta.itemId && meta.documentId) {
          this.chunkRegistry.set(meta.itemId, meta)
          const existing = this.docChunkIds.get(meta.documentId) || []
          existing.push(meta.itemId)
          this.docChunkIds.set(meta.documentId, existing)

          if (!docMap.has(meta.documentId)) {
            docMap.set(meta.documentId, { meta, count: 1 })
          } else {
            docMap.get(meta.documentId)!.count++
          }
        }
      }

      for (const [docId, { meta, count }] of docMap) {
        if (!this.fileRegistry.has(docId)) {
          this.fileRegistry.set(docId, {
            id: docId,
            fileName: meta.fileName,
            filePath: meta.filePath,
            totalPages: meta.pageNumber,
            wordCount: 0,
            loadedAt: Date.now(),
            chunkCount: count,
          })
        }
      }

      for (const item of allItems) {
        const meta = item.metadata as unknown as ChunkMetadata
        if (meta && meta.documentId && this.fileRegistry.has(meta.documentId)) {
          const fi = this.fileRegistry.get(meta.documentId)!
          if (meta.pageNumber > fi.totalPages) {
            fi.totalPages = meta.pageNumber
          }
        }
      }
    } catch {
      // Index may be empty — that's fine
    }
  }

  async addDocument(doc: ParsedDocument, settings: AppSettings): Promise<FileInfo> {
    if (!this.index) throw new Error('RagPipeline not initialized')

    const chunks = this.chunkDocument(doc, settings.chunkSize, settings.chunkOverlap)
    const itemIds: string[] = []

    for (const chunk of chunks) {
      try {
        const embedding = await this.ollamaService.embed(
          chunk.text,
          settings.embedModel,
          settings.ollamaBaseUrl
        )
        const itemId = uuidv4()
        const meta: ChunkMetadata = { ...chunk, itemId }
        await this.index.insertItem({
          id: itemId,
          vector: embedding,
          metadata: meta as unknown as Record<string, MetadataTypes>,
        })
        this.chunkRegistry.set(itemId, meta)
        itemIds.push(itemId)
      } catch (err) {
        console.error(`Failed to embed chunk ${chunk.chunkIndex} of ${doc.fileName}:`, err)
      }
    }

    this.docChunkIds.set(doc.id, itemIds)

    const fileInfo: FileInfo = {
      id: doc.id,
      fileName: doc.fileName,
      filePath: doc.filePath,
      totalPages: doc.totalPages,
      wordCount: doc.wordCount,
      loadedAt: doc.loadedAt,
      chunkCount: itemIds.length,
    }

    this.fileRegistry.set(doc.id, fileInfo)
    return fileInfo
  }

  async removeDocument(fileId: string): Promise<void> {
    if (!this.index) throw new Error('RagPipeline not initialized')

    const itemIds = this.docChunkIds.get(fileId) || []
    for (const id of itemIds) {
      try {
        await this.index.deleteItem(id)
        this.chunkRegistry.delete(id)
      } catch (err) {
        console.error(`Failed to delete item ${id}:`, err)
      }
    }

    this.docChunkIds.delete(fileId)
    this.fileRegistry.delete(fileId)
  }

  async query(question: string, settings: AppSettings): Promise<QueryResult> {
    if (!this.index) throw new Error('RagPipeline not initialized')

    if (!settings.hfToken?.trim()) {
      throw new Error('HuggingFace token is not configured. Open Settings to add your token.')
    }

    // Retrieve more candidates than topK so we can filter and diversify
    const candidateK = Math.min(settings.topK * 3, 30)

    let queryEmbedding: number[]
    try {
      queryEmbedding = await this.ollamaService.embed(
        question,
        settings.embedModel,
        settings.ollamaBaseUrl
      )
    } catch (err) {
      throw new Error(`Failed to embed query: ${err}`)
    }

    let rawResults: Array<{ item: { metadata: Record<string, MetadataTypes> }; score: number }> = []
    try {
      rawResults = await this.index.queryItems(queryEmbedding, question, candidateK)
    } catch {
      rawResults = []
    }

    // 1. Filter by score threshold
    const filtered = rawResults.filter((r) => r.score >= SCORE_THRESHOLD)

    // 2. Diversity: cap chunks per (filePath, pageNumber) pair
    const pageCount = new Map<string, number>()
    const diverse = filtered.filter((r) => {
      const meta = r.item.metadata as unknown as ChunkMetadata
      const key = `${meta.filePath}::${meta.pageNumber}`
      const count = pageCount.get(key) ?? 0
      if (count >= MAX_CHUNKS_PER_PAGE) return false
      pageCount.set(key, count + 1)
      return true
    })

    // 3. Take topK after diversification
    const results = diverse.slice(0, settings.topK)

    const retrievedChunks = results.map((r) => {
      const meta = r.item.metadata as unknown as ChunkMetadata
      return { ...meta, score: r.score }
    })

    if (retrievedChunks.length === 0) {
      return {
        answer: 'I could not find information about this in your loaded documents. Please ensure the relevant files are loaded.',
        citations: [],
        notFound: true,
      }
    }

    // Build context — include filename, page, and full chunk text
    const contextParts = retrievedChunks.map((chunk, idx) => {
      return `[${idx + 1}] File: "${chunk.fileName}" | Page ${chunk.pageNumber}\n${chunk.text}`
    })
    const context = contextParts.join('\n\n---\n\n')
    const systemWithContext = SYSTEM_PROMPT.replace('{context}', context)

    let answer: string
    try {
      answer = await askSaul(systemWithContext, question, settings.hfToken)
    } catch (err) {
      throw new Error(`Failed to generate answer: ${err}`)
    }

    const notFound =
      answer.toLowerCase().includes('i could not find') ||
      answer.toLowerCase().includes('no relevant')

    // Build citations with best-sentence excerpts, sorted by score
    const citations: Citation[] = retrievedChunks
      .sort((a, b) => b.score - a.score)
      .map((chunk) => ({
        fileName: chunk.fileName,
        filePath: chunk.filePath,
        pageNumber: chunk.pageNumber,
        excerpt: this.bestExcerpt(chunk.text, question),
        score: chunk.score,
      }))

    return { answer, citations: notFound ? [] : citations, notFound }
  }

  getFiles(): FileInfo[] {
    return Array.from(this.fileRegistry.values())
  }

  getPageText(filePath: string, pageNumber: number): string {
    const texts: string[] = []
    for (const chunk of this.chunkRegistry.values()) {
      if (chunk.filePath === filePath && chunk.pageNumber === pageNumber) {
        texts.push(chunk.text)
      }
    }
    return texts.join(' ')
  }

  // ── Private helpers ────────────────────────────────────────────────────────

  /**
   * Extract the single most relevant sentence from a chunk given the query.
   * Falls back to the first 280 chars if no good match.
   */
  private bestExcerpt(text: string, query: string): string {
    const sentences = text
      .split(/(?<=[.!?])\s+/)
      .map((s) => s.trim())
      .filter((s) => s.length > 20)

    if (sentences.length === 0) {
      return text.slice(0, 280) + (text.length > 280 ? '…' : '')
    }

    const queryWords = new Set(
      query.toLowerCase().split(/\W+/).filter((w) => w.length > 3)
    )

    let best = sentences[0]
    let bestScore = 0

    for (const sentence of sentences) {
      const words = sentence.toLowerCase().split(/\W+/)
      const hits = words.filter((w) => queryWords.has(w)).length
      const score = hits / Math.sqrt(words.length)
      if (score > bestScore) {
        bestScore = score
        best = sentence
      }
    }

    return best.length > 320 ? best.slice(0, 320) + '…' : best
  }

  /**
   * Sentence-boundary aware chunker.
   * Tries to break at sentence endings rather than mid-word.
   */
  private chunkDocument(
    doc: ParsedDocument,
    chunkSize: number,
    chunkOverlap: number
  ): DocumentChunk[] {
    const chunks: DocumentChunk[] = []
    let globalChunkIndex = 0

    for (const page of doc.pages) {
      const text = page.text
      if (!text || text.trim().length === 0) continue

      // Split into sentences, recombine into chunks respecting sentence boundaries
      const sentences = text.split(/(?<=[.!?])\s+/).filter((s) => s.trim().length > 0)

      let currentChunk = ''
      const sentenceBuffer: string[] = []

      const flush = () => {
        const chunkText = currentChunk.trim()
        if (chunkText.length > 0) {
          chunks.push({
            id: uuidv4(),
            documentId: doc.id,
            fileName: doc.fileName,
            filePath: doc.filePath,
            pageNumber: page.pageNumber,
            chunkIndex: globalChunkIndex++,
            text: chunkText,
            tokenCount: Math.ceil(chunkText.length / 4),
          })
        }
      }

      for (const sentence of sentences) {
        if (currentChunk.length + sentence.length + 1 > chunkSize && currentChunk.length > 0) {
          flush()
          // Overlap: carry last N chars worth of sentences into next chunk
          let overlapText = ''
          for (let i = sentenceBuffer.length - 1; i >= 0; i--) {
            const candidate = sentenceBuffer[i] + ' ' + overlapText
            if (candidate.length > chunkOverlap) break
            overlapText = candidate
          }
          currentChunk = overlapText.trim()
          sentenceBuffer.length = 0
        }
        currentChunk += (currentChunk ? ' ' : '') + sentence
        sentenceBuffer.push(sentence)
      }

      flush()
    }

    return chunks
  }
}

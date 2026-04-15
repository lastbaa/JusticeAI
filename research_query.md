# Deep Research Query: Best Open-Source Retrieval Backend for a Local Rust RAG Pipeline

## Context

I have a **desktop app** (Tauri 2 / Rust backend) that does local-only legal document RAG:
- PDF/DOCX → chunk → embed (fastembed BGE-small-en-v1.5, 384 dims) → retrieve → LLM (Qwen3-8B GGUF via llama-cpp-2; originally Saul-7B, upgraded April 2026)
- Everything runs on-device (macOS, Apple Silicon). No server, no cloud.
- Corpus is small: typically 1–20 documents, 50–500 chunks total, all in-memory.
- Current retrieval: hand-rolled BM25 + cosine similarity hybrid, MMR diversity selection. Works OK for keyword-heavy queries but fails on semantic gaps (e.g. "what is the person's name?" doesn't retrieve a chunk containing "Liam Neild 18 Eagle Row" because there's zero lexical overlap).

## What I Need

A **Rust-native or Rust-compatible** retrieval/ranking solution that:
1. Works locally, no network calls
2. Handles the semantic gap problem (query terms don't appear in the target chunk)
3. Is a real open-source crate, actively maintained (not abandoned)
4. Isn't overkill for a small in-memory corpus (I don't need distributed search)
5. Integrates cleanly — I already have embeddings, I just need better scoring/ranking

## Specific Questions

1. **Tantivy** — Is it the right tool here? It's a full-text search engine (Rust Lucene). Does it actually help with the semantic gap, or is it just better BM25? Can it do hybrid semantic+keyword search, or would I still need to combine it with cosine similarity myself?

2. **Cross-encoder rerankers** — fastembed (which I already use for embeddings) supports reranker models. Is a reranker the better solve for my problem? Something like `BAAI/bge-reranker-v2-m3` that takes (query, passage) pairs and scores relevance directly. What's the latency like for 50–500 chunks on Apple Silicon?

3. **Other Rust crates** — Are there other options I'm missing? Things like:
   - `hnsw` or `hora` for better ANN search
   - `qdrant` client (but that's a server...)
   - Any Rust bindings to ONNX rerankers
   - `candle` for running a cross-encoder locally

4. **What does the SOTA RAG stack look like in 2025/2026?** Specifically for small-corpus local retrieval. Is the consensus still "embed + BM25 hybrid + reranker" or has something better emerged?

5. **Practical recommendation** — Given my constraints (Rust binary, local-only, small corpus, already have fastembed + cosine + BM25), what's the minimum-complexity change that would most improve retrieval quality? I have a pluggable `RetrievalBackend` trait ready to go — I just need to know what to plug in.

## Current Architecture (for context)

```rust
// I have this trait ready:
pub trait RetrievalBackend {
    fn retrieve(&self, query_text: &str, query_vector: &[f32],
                corpus: &RetrievalCorpus, config: &RetrievalConfig) -> Vec<ScoredResult>;
    fn name(&self) -> &str;
}

// Current default:
pub struct HybridBm25Cosine { alpha: f32, form_boost: f32 }

// Embedding: fastembed crate, BGE-small-en-v1.5 (ONNX, ~33MB)
// LLM: llama-cpp-2 crate, Qwen3-8B Q4_K_M GGUF (~5GB)
// Corpus: Vec<EmbeddedChunkEntry> { vector: Vec<f32>, meta: ChunkMetadata }
```

## What I DON'T want
- A Python solution (this is a compiled Rust binary)
- A server/service I have to run alongside the app (Qdrant, Milvus, etc.)
- Something that requires >500MB of additional model downloads
- Over-engineered abstractions — I want the simplest thing that works

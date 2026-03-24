# Project Guide

This file provides guidance when working with code in this repository.

## Project Overview

Justice AI is a privacy-first legal research desktop app built with Tauri 2 (Rust backend + React frontend). All processing runs locally — document parsing, embedding (fastembed BGE-small-en-v1.5), and LLM inference (Saul-7B-Instruct Q4_K_M via llama-cpp-2) happen on-device. No cloud services, no API keys required.

## Monorepo Structure

```
/
├── app/          # Tauri 2 + React desktop app (primary codebase)
├── website/      # Next.js marketing site
├── shared/       # Shared TypeScript types (IPC channels, DTOs, models)
└── package.json  # Workspace root
```

## Commands

All commands run from the **repo root** unless noted:

```bash
npm run app              # Start desktop app in dev mode (Tauri dev with Vite HMR)
npm run website          # Start Next.js site at localhost:3000
npm run build:app        # Build production app bundle
npm run build:website    # Build website for production
cd app/src-tauri && cargo test  # Run Rust unit tests
```

## Prerequisites

- Node.js 20+
- Rust toolchain (install via [rustup](https://rustup.rs/))
- Platform build tools required by Tauri (see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/))

No external services needed — models auto-download on first launch.

## Architecture: Rust Backend (`app/src-tauri/src/`)

All business logic lives in Rust, exposed to the frontend via Tauri IPC commands:

- **`lib.rs`** — Tauri setup, state initialization, plugin registration
- **`state.rs`** — `RagState`, `AppSettings`, and all domain types
- **`pipeline.rs`** — Core RAG pipeline: document chunking, embedding via fastembed, BM25 + cosine hybrid retrieval with Reciprocal Rank Fusion (RRF), MMR for diversity, and LLM inference via llama-cpp-2
- **`commands/rag.rs`** — Tauri command handlers (IPC bridge between frontend and pipeline)
- **`commands/doc_parser.rs`** — PDF (`lopdf`), DOCX (`zip` + `roxmltree`), TXT, CSV, HTML, EML, XLSX, and image OCR parsing
- **`assertions.rs`** — Citation format validation, number exactness checks, hallucination detection

Storage:
- **Vector store** — In-memory `Vec<EmbeddedChunkEntry>` with cosine similarity, persisted to `{app_data}/chunks.json`
- **Models** — Saul-7B GGUF (~4.5 GB) at `{app_data}/models/saul.gguf`, fastembed ONNX (~33 MB) at `{app_data}/models/fastembed-bge/`
- **Settings & chat history** — Tauri-managed app data directory

## Architecture: Frontend (`app/src/renderer/src/`)

React app. State management lives in `App.tsx` (messages, files, sessions, settings). Key components:

- **`App.tsx`** — Root state management (messages, files, sessions, settings)
- **`components/ChatInterface.tsx`** — Main chat UI. Sends queries through the RAG pipeline via Tauri invoke.
- **`components/MessageBubble.tsx`** — Renders chat messages. Uses `react-markdown` for assistant responses.
- **`components/Sidebar.tsx`** — Session list, case management, file organization
- **`components/Settings.tsx`** — RAG configuration (chunk size, topK), practice area presets, theme selection
- **`components/ModelSetup.tsx`** — First-launch model download screen with progress tracking
- **`api.ts`** — `window.api` shim using Tauri `invoke()` and `@tauri-apps/plugin-dialog`

## Shared Types (`shared/src/types.ts`)

All IPC contracts and domain models (`AppSettings`, `ChatSession`, `FileInfo`, `ModelStatus`, `DownloadProgress`, etc.) are defined here. **Always update this file when adding new Tauri commands.**

Default settings: `chunkSize: 1000`, `chunkOverlap: 150`, `topK: 6`.

## Styling

Tailwind CSS with a Navy (`#0d1117`) + Gold (`#c9a84c`) color scheme. Both `app/` and `website/` have their own `tailwind.config.ts`.

## Key Technical Notes

- **Zero cloud calls** — All documents, embeddings, and LLM inference stay on-device. Do not introduce external API calls without explicit user confirmation.
- **Tauri security** — Commands are exposed via `invoke()`. No node integration. File dialogs use `@tauri-apps/plugin-dialog` on the JS side to avoid async deadlocks.
- **TypeScript** — Vite + React with `@renderer` path alias resolving to `src/renderer/src` (configured in `vite.config.ts`).
- **Retrieval pipeline** — Hybrid BM25 + cosine similarity with Reciprocal Rank Fusion (RRF). MMR reranking for diversity. Legal synonym expansion in BM25. Pluggable `RetrievalBackend` trait.
- **Models auto-download** — On first launch, `ModelSetup.tsx` triggers `download_models` Tauri command. Saul-7B GGUF (~4.5 GB) + fastembed BGE ONNX (~33 MB). Progress reported via `download-progress` events.
- **Eval system** — `cargo run --bin harness` runs 77 eval cases across 8 fixtures. Metrics: MRR, P@1, recall, partial score. Design doc at `app/src-tauri/EVAL_SYSTEM_DESIGN.md`.

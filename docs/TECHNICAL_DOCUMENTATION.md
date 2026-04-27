# Technical Documentation

## Purpose

This document is the maintainer-oriented technical reference for the active Justice AI desktop application.

Its scope is intentionally bounded to the code that powers the current product runtime:

- `app/src-tauri` — active Rust + Tauri backend
- `app/src/renderer` — active React renderer
- `shared/src/types.ts` — shared frontend/backend contract

It does **not** attempt to fully document:

- the legacy Electron runtime in `app/src/main` and `app/src/preload`
- the marketing website in `website`
- the separate security prototype in `security`

Those areas may still be useful for reference, but they are not the source of truth for the current desktop app.

## Source Of Truth

When this document and older repo notes disagree, prefer the active runtime code.

Primary source-of-truth files:

- `app/src-tauri/src/lib.rs`
- `app/src-tauri/src/state.rs`
- `app/src-tauri/src/commands/rag.rs`
- `app/src-tauri/src/commands/doc_parser.rs`
- `app/src-tauri/src/pipeline.rs`
- `app/src/renderer/src/App.tsx`
- `app/src/renderer/src/api.ts`
- `shared/src/types.ts`

## High-Level Architecture

Justice AI is a local-first desktop app built as a three-layer system:

1. **Renderer**
   - React + TypeScript + Vite
   - Owns UI state, views, interactions, and user workflow orchestration

2. **Bridge**
   - Tauri `invoke()` commands and event listeners
   - Converts UI actions into backend calls and streams backend progress back to the UI

3. **Backend**
   - Rust command handlers, parser pipeline, retrieval pipeline, model management, persistence, and local file serving

At a high level, the desktop app does this:

1. Start Tauri and load persisted state
2. Parse and ingest user documents
3. Chunk and embed those documents locally
4. Retrieve relevant chunks for a user question
5. Generate a grounded answer with citations
6. Persist cases, sessions, settings, and document metadata locally

## Repository Boundaries

### Active desktop runtime

- `app/src-tauri`
- `app/src/renderer`
- `shared`

### Secondary but relevant

- `docs`
- `app/src-tauri/tests`
- `app/src-tauri/src/bin/harness.rs`
- `app/src/renderer/src/__tests__`

### Not the active desktop runtime

- `app/src/main`
- `app/src/preload`
- `website`
- `security`

## Core Module Map

### Backend bootstrap and shell

`app/src-tauri/src/lib.rs`

- Starts the Tauri app
- Resolves the app data directory
- Loads persisted `RagState` before the window opens
- Runs one-time migrations for stale embeddings and garbled stored chunks
- Starts the loopback file server used by the document viewer
- Registers all Tauri commands
- Implements the close-confirmation handshake between Rust and the renderer

This file should stay thin. If logic grows here, it is usually a sign it belongs in `state.rs`, `commands/rag.rs`, or `pipeline.rs`.

### Persistence and shared backend state

`app/src-tauri/src/state.rs`

- Defines the persisted domain model
- Mirrors the shared TypeScript contract
- Stores the canonical in-memory `RagState`
- Saves and loads JSON-backed data from disk
- Owns utility methods such as excerpt selection and cosine similarity helpers

`RagState` is the backend's long-lived application state. It includes:

- document registry
- embedded chunks
- chunk registry and chunk-to-document mappings
- chat sessions
- settings
- cases
- entities
- model directory info
- BM25 cache

If a feature changes what the app needs to remember between runs, this file is almost always involved.

### Command/controller layer

`app/src-tauri/src/commands/rag.rs`

- Exposes most of the app command surface to the renderer
- Coordinates state, parser, retrieval, sessions, cases, and model operations
- Emits progress and streaming events
- Translates UI-oriented actions into backend workflows

This is the controller layer, not the algorithm layer.

A good rule:

- put orchestration here
- put reusable retrieval/parsing/model logic in `pipeline.rs` or `doc_parser.rs`
- put durable data shape changes in `state.rs`

### Document parsing

`app/src-tauri/src/commands/doc_parser.rs`

- Accepts supported file types
- Extracts text and metadata
- Handles PDF extraction, OCR-assisted image ingestion, XML handling, and document normalization
- Produces the `DocumentPage` representation consumed by chunking and retrieval

This file is the right place to add a new ingest format or improve extraction quality for an existing format.

### Retrieval and model pipeline

`app/src-tauri/src/pipeline.rs`

- Chunking
- Embedding
- Retrieval backends
- BM25 / cosine / RRF / MMR logic
- Query rewriting and special-case routing
- Inference mode parameters
- LLM prompt construction and invocation
- Jurisdiction detection
- Fact and entity extraction helpers

This file is the core algorithm layer of the app.

### Answer quality and grading

`app/src-tauri/src/assertions.rs`

- Citation-format checks
- fabricated-entity checks
- number exactness checks
- hallucination-oriented heuristics
- confidence blending

`app/src-tauri/src/grading.rs`

- Evaluation and scoring utilities used for quality measurement rather than normal UI flow

### Renderer orchestration

`app/src/renderer/src/App.tsx`

- Loads settings, files, sessions, and cases on startup
- Owns most top-level UI state
- Coordinates file ingestion, querying, document role updates, case assignment, and session persistence
- Listens to backend progress and token events
- Composes the main application view from child components

This is the renderer orchestration layer. It is intentionally state-heavy.

### Renderer bridge

`app/src/renderer/src/api.ts`

- Wraps Tauri `invoke()` calls
- Defines event listeners for streaming and progress
- Exposes a stable `window.api` shape to the renderer

This file is the clean boundary between the UI and the backend.

### Shared contract

`shared/src/types.ts`

- TypeScript mirror of the Rust domain model
- Shared DTOs for files, settings, sessions, citations, cases, entities, and query results

This file must remain aligned with `app/src-tauri/src/state.rs`.

## Startup Flow

The active startup flow is:

1. `npm run app` launches `tauri dev`
2. `app/src-tauri/src/main.rs` calls `run()`
3. `app/src-tauri/src/lib.rs` builds the Tauri application
4. `RagState` is created and loaded from disk
5. migration steps run if needed
6. the local file server starts
7. commands are registered
8. the renderer mounts through `app/src/renderer/src/main.tsx`
9. `App.tsx` loads settings, files, sessions, cases, and model status
10. the UI decides whether to show the normal shell or setup flow

Maintainability note:

- backend state is loaded before the window opens so first renderer calls see a consistent state
- the close-protection flow is split across Rust and React; changes to app shutdown should account for both sides

## Document Ingestion Flow

The ingestion path is:

1. The renderer picks files or folders through `api.ts`
2. `window.api.loadFiles(...)` calls the `load_files` command in `commands/rag.rs`
3. `rag.rs` delegates content extraction to `doc_parser.rs`
4. Parsed pages are chunked by `pipeline::chunk_document(...)`
5. Chunks are embedded locally
6. File metadata, chunks, and embeddings are inserted into `RagState`
7. The backend persists updated state to disk
8. Progress events are emitted back to the renderer
9. `App.tsx` updates the file list and progress UI

Important maintainability details:

- ingestion is not just file parsing; it also updates file registry, chunk registry, embeddings, cache validity, and persistence
- if ingestion changes the corpus, BM25 cache invalidation matters
- adding a file type should be treated as a parser + metadata + testing change, not just a parser change

## Query / Retrieval / Generation Flow

The query path is:

1. The user asks a question in `ChatInterface.tsx`
2. `App.tsx` builds history and case context, then calls `window.api.query(...)`
3. `commands/rag.rs::query(...)` orchestrates the entire backend question flow
4. `pipeline.rs` handles retrieval logic, query rewriting, embedding, and prompt assembly
5. `ask_llm(...)` runs the local model
6. streaming tokens are emitted through `query-token`
7. status and phase updates are emitted through `query-status`
8. the final `QueryResult` returns answer, citations, assertions, and confidence
9. the renderer stores the response in the current chat session and updates source panels

The current architecture deliberately separates responsibilities:

- `rag.rs` decides **when** to run each stage
- `pipeline.rs` defines **how** retrieval and generation behave
- `assertions.rs` checks answer quality after generation

Maintainability note:

- if you are changing search quality, start in `pipeline.rs`
- if you are changing query workflow, start in `rag.rs`
- if you are changing answer post-processing or confidence, start in `assertions.rs`

## State And Persistence Model

Persisted app data lives under the app data directory managed by Tauri.

Important persisted artifacts include:

- settings
- chat sessions
- cases
- file registry
- chunk metadata and embeddings
- file hashes
- embedding model marker

Persistence responsibilities:

- `state.rs` owns paths, serialization, and save/load helpers
- command handlers call those save methods when mutations occur

Maintainability rules:

- any new persistent field should be added compatibly
- Rust defaults and TypeScript defaults should not drift silently
- if you change persisted schemas, update load behavior so older data still works when practical

## Frontend / Backend Contract Boundaries

The most important maintainability boundary in the repo is the shared contract:

- Rust structs in `state.rs`
- TypeScript interfaces in `shared/src/types.ts`
- command wrappers in `api.ts`

When adding or changing app data:

1. update the Rust type
2. update the shared TypeScript type
3. update the backend command payload or response
4. update the `api.ts` wrapper
5. update the renderer usage

If only one side is changed, the app may still compile in parts while failing at runtime.

High-risk contract areas:

- `AppSettings`
- `QueryResult`
- `Citation`
- `FileInfo`
- `ChatSession`
- `Case`
- `DocumentRole`

## Telemetry And Event Flow

The app uses two kinds of backend-to-frontend event flows:

### Streaming/query events

- `query-token`
- `query-status`

Used for:

- streamed answer text
- query phase status
- progress reporting

### Ingestion/model events

- `file-load-progress`
- `download-progress`

Used for:

- document ingestion progress
- model download/setup progress

Maintainability note:

- backend event names are part of the runtime contract
- if you rename or reshape an event in Rust, you must update `api.ts` and the renderer listeners

## UI Structure

`App.tsx` composes the main UI from focused components. The most important ones are:

- `Sidebar.tsx` — sessions, cases, navigation
- `ChatInterface.tsx` — message thread, input, and progress presentation
- `ContextPanel.tsx` — citations, facts, and surrounding context
- `DocumentViewer.tsx` — document inspection and source viewing
- `Settings.tsx` — retrieval and appearance settings
- `ModelSetup.tsx` — initial model setup and upgrade flow

This split is mostly by user workflow, not by data ownership. Data ownership remains centralized in `App.tsx`.

Maintainability rule:

- keep shared state orchestration in `App.tsx`
- keep presentation logic in leaf components
- if the same backend-driven state is needed in multiple places, prefer lifting it into `App.tsx` rather than duplicating fetch logic

## Testing And Regression Surfaces

The project has three important testing layers.

### Rust unit tests

Located primarily in:

- `pipeline.rs`
- `assertions.rs`
- `doc_parser.rs`
- `state.rs`
- `lib.rs`

These are the main regression surface for:

- retrieval behavior
- chunking
- parser behavior
- hallucination checks
- serialization behavior

### Integration and harness evaluation

- `app/src-tauri/tests/pipeline_integration.rs`
- `app/src-tauri/src/bin/harness.rs`

Use these when changing:

- retrieval ranking
- parsing quality
- citation grounding
- evaluation metrics

### Renderer tests

- `app/src/renderer/src/__tests__`

These cover targeted UI/runtime helpers, API behavior, and renderer-side logic.

## Safe Change Guide

### Adding a new backend command

1. implement the command in `commands/rag.rs` or another command module
2. register it in `lib.rs`
3. add the wrapper to `api.ts`
4. update shared types if needed
5. call it from the renderer
6. add a regression test if the behavior is important

### Adding a new supported file type

1. extend parsing in `doc_parser.rs`
2. ensure it can produce `DocumentPage` output
3. confirm chunking and role behavior still make sense
4. update file dialog filters in `api.ts`
5. update user-facing docs if needed
6. add parser tests

### Changing retrieval behavior

1. prefer `pipeline.rs`
2. keep `rag.rs` changes minimal unless orchestration must change
3. validate effects with unit tests and harness runs
4. watch for BM25 cache assumptions and corpus mutation points

### Adding a field to shared state

1. add it to Rust in `state.rs`
2. add it to `shared/src/types.ts`
3. propagate serialization/deserialization safely
4. update all commands returning that type
5. update renderer state initialization and saves

### Changing persistence

1. update path/save/load logic in `state.rs`
2. preserve backward compatibility when practical
3. avoid partial updates that leave in-memory and on-disk state inconsistent

## Known Maintenance Risks

### Contract drift

`state.rs` and `shared/src/types.ts` intentionally mirror one another. They are one of the easiest places for silent drift.

### Mixed old and new architecture in the repo

The repo still contains legacy Electron code. It can be useful for reference, but it is easy to waste time editing the wrong runtime path.

For current desktop behavior, prefer:

- `app/src-tauri`
- `app/src/renderer`

### Controller bloat

`App.tsx` and `commands/rag.rs` both carry a lot of orchestration load. They are the most likely places to become hard to maintain if new features are added without preserving boundaries.

### Retrieval complexity growth

Most feature pressure lands in `pipeline.rs`. This file is powerful but can become overly coupled if new heuristics are added without tests and clear separation between:

- ingestion helpers
- retrieval logic
- prompt logic
- answer utilities

## Recommended Reading Order For New Contributors

If you are new to the codebase, read in this order:

1. `README.md`
2. `docs/TECHNICAL_DOCUMENTATION.md`
3. `app/src-tauri/src/lib.rs`
4. `app/src/renderer/src/App.tsx`
5. `app/src/renderer/src/api.ts`
6. `app/src-tauri/src/state.rs`
7. `app/src-tauri/src/commands/rag.rs`
8. `app/src-tauri/src/commands/doc_parser.rs`
9. `app/src-tauri/src/pipeline.rs`

That path gives the fastest understanding of the active runtime before diving into details.

## Maintenance Summary

If you remember only a few rules, keep these:

- treat `app/src-tauri` + `app/src/renderer` as the active desktop runtime
- keep `state.rs` and `shared/src/types.ts` aligned
- use `rag.rs` for orchestration, `pipeline.rs` for retrieval/generation logic, and `doc_parser.rs` for extraction
- keep event names and payloads synchronized across Rust, `api.ts`, and React listeners
- add tests when changing parser behavior, retrieval logic, or persistence
- avoid editing legacy Electron code unless you are explicitly working on migration/reference tasks

# Contributing to Justice AI

## Development Setup

```bash
git clone https://github.com/lastbaa/CS-370-Justice-AI-Project.git
cd CS-370-Justice-AI-Project
npm install
npm run app          # Launches Tauri dev mode (Rust + Vite with hot reload)
```

See [INSTALLATION.md](INSTALLATION.md) for platform-specific prerequisites (Rust, Node.js, system libraries).

## Project Structure

```
/
├── app/
│   ├── src-tauri/          # Rust backend (Tauri 2)
│   │   ├── src/
│   │   │   ├── lib.rs          # App setup, state initialization
│   │   │   ├── state.rs        # RagState, AppSettings, domain types
│   │   │   ├── commands/       # Tauri IPC commands
│   │   │   │   ├── rag.rs      # RAG pipeline (embed, chunk, search, ask)
│   │   │   │   ├── doc_parser.rs  # PDF/DOCX parsing
│   │   │   │   └── ollama.rs   # Local model status checks
│   │   │   └── pipeline.rs     # Retrieval backends (BM25+Cosine, Reranker)
│   │   ├── Cargo.toml
│   │   └── tauri.conf.json
│   └── src/renderer/src/       # React frontend
│       ├── App.tsx             # Root component, state management
│       ├── api.ts              # IPC bridge (Tauri invoke wrappers)
│       └── components/         # UI components
├── shared/src/types.ts         # Shared TypeScript types & IPC definitions
├── website/                    # Next.js marketing site
└── docs/                       # Documentation
```

## Running Tests

```bash
cd app/src-tauri
cargo test
```

The project also includes an eval harness for retrieval quality:

```bash
cd app/src-tauri
cargo run --bin harness -- --eval tests/fixtures/eval.json
```

## Code Style

- **Rust**: Standard `rustfmt` formatting. Run `cargo fmt` before committing.
- **TypeScript**: Follow existing project conventions. No linter is configured, so match the style of surrounding code.
- **Commits**: Use conventional-style messages (e.g., `feat:`, `fix:`, `docs:`).

## Pull Request Process

1. Fork the repository and create a feature branch from `main`
2. Make your changes
3. Verify your changes compile and pass tests:
   ```bash
   cd app/src-tauri
   cargo check
   cargo test
   cargo fmt -- --check
   ```
4. Submit a pull request against `main` with a clear description of the change

## Architecture Overview

Justice AI uses **Tauri 2** with a Rust backend and React frontend. Key design decisions:

- **Fully local**: No cloud APIs. The Qwen3-8B LLM runs on-device via `llama-cpp-2`. Embeddings use `fastembed` (BGE-Small, ONNX runtime). Documents never leave the machine.
- **Hybrid retrieval**: BM25 keyword search + cosine similarity over embeddings, fused with Reciprocal Rank Fusion (RRF). An optional cross-encoder reranker backend is also available.
- **IPC via Tauri commands**: The frontend calls Rust functions through Tauri's `invoke()` API. All command definitions live in `app/src-tauri/src/commands/`. Shared types are in `shared/src/types.ts`.
- **Vector storage**: In-memory `Vec<EmbeddedChunkEntry>` with cosine similarity, persisted to `chunks.json`. No external database.
- **Document parsing**: PDF via `lopdf`, DOCX via `zip` + `roxmltree`, all in Rust.

## Adding a New IPC Command

1. Define the command function in `app/src-tauri/src/commands/`
2. Register it in `lib.rs` via `.invoke_handler(tauri::generate_handler![...])`
3. Add the TypeScript types to `shared/src/types.ts`
4. Add the frontend wrapper in `app/src/renderer/src/api.ts`

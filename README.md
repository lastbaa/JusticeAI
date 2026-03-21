[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

# Justice AI

Private, citation-grounded legal document research on your local machine.

> Justice AI is a research assistant, not legal advice. Attorneys remain responsible for legal conclusions.

## Features

- **Privacy-first** — All processing happens on-device. Documents never leave your machine.
- **Citation-grounded answers** — Every response includes file name, page number, and excerpt references.
- **Hybrid retrieval** — BM25 keyword search + semantic cosine similarity, fused with Reciprocal Rank Fusion (RRF) for best-of-both-worlds accuracy.
- **Local legal LLM** — Saul-7B-Instruct (Q4_K_M) runs entirely on your hardware via llama-cpp-2. No API keys, no cloud.
- **Case management** — Organize documents into cases with scoped retrieval per case.
- **16 supported file types** — PDF, DOCX, TXT, MD, CSV, EML, HTML, HTM, MHTML, XML, XLSX, PNG, JPG, JPEG, TIF, TIFF — including OCR for images.
- **Practice area presets** — Tailored retrieval settings for different legal domains.
- **Dark / light theme** — Switch between themes in settings.
- **Cross-platform** — macOS, Windows, and Linux.

## Architecture

```
┌─────────────────────────────────────────────────┐
│                React UI (Vite)                   │
│  Sidebar │ Chat Interface │ Document Viewer      │
├─────────────────────────────────────────────────┤
│              Tauri IPC Bridge                    │
├─────────────────────────────────────────────────┤
│              Rust Backend                        │
│  Doc Parser │ Chunker │ Embedder │ Retriever     │
│                  │                               │
│          Saul-7B (llama-cpp-2)                   │
│          BGE-small (fastembed)                   │
└─────────────────────────────────────────────────┘
```

- **Frontend**: React + Vite + Tailwind CSS
- **Backend**: Rust (Tauri 2) — handles parsing, chunking, embedding, retrieval, and LLM inference
- **Embedding model**: BGE-small-en-v1.5 via fastembed (ONNX, ~33 MB)
- **LLM**: Saul-7B-Instruct Q4_K_M GGUF via llama-cpp-2 (~4.5 GB)
- **Vector store**: In-memory with cosine similarity, persisted to disk

## System Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| RAM | 8 GB | 16 GB+ |
| Disk | 6 GB free (for models) | 10 GB+ |
| OS | macOS 12+, Windows 10+, Ubuntu 22.04+ | Latest stable |
| CPU | 4 cores | 8+ cores (Apple Silicon or modern x86) |

## How It Works

Justice AI uses a **Retrieval-Augmented Generation (RAG)** pipeline:

1. **Parse** — Documents are parsed locally (PDF via `lopdf`, DOCX via `zip`/`roxmltree`, images via Tesseract OCR, and more).
2. **Chunk** — Text is split into overlapping chunks (default: 500 tokens, 50 overlap) to preserve context across boundaries.
3. **Embed** — Each chunk is embedded using BGE-small-en-v1.5 (local ONNX model via fastembed). No network calls.
4. **Retrieve** — When you ask a question, the query is matched against chunks using hybrid retrieval:
   - **BM25** keyword scoring with legal synonym expansion
   - **Cosine similarity** on embeddings
   - **Reciprocal Rank Fusion** merges both ranked lists
   - **MMR reranking** ensures diversity in results
5. **Generate** — Top chunks are passed as context to Saul-7B-Instruct, which generates an answer with inline citations referencing specific files and pages.

## Supported File Types

| Type | Extensions | Notes |
|------|-----------|-------|
| PDF | `.pdf` | Native text extraction via `lopdf` |
| Word | `.docx` | XML-based extraction |
| Plain text | `.txt`, `.md` | Direct ingestion |
| Spreadsheet | `.csv`, `.xlsx` | Row-based chunking |
| Email | `.eml` | Header + body extraction |
| Web | `.html`, `.htm`, `.mhtml`, `.xml` | Tag stripping, DTD/entity safety checks |
| Image | `.png`, `.jpg`, `.jpeg`, `.tif`, `.tiff` | OCR via Tesseract (quality depends on source) |

> Note: All listed types can be ingested, but extraction quality depends on the source. OCR cannot rectify poor image quality.

## Setup

### Prerequisites

- **Node.js** 20+
- **Rust toolchain** — install via [rustup.rs](https://rustup.rs/)
- **Platform build tools** — see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

No external services, API keys, or accounts required.

### Install dependencies

From the repo root:

```bash
npm install
```

## Run the App (Development)

From the repo root:

```bash
npm run app
```

On first launch, the app will:

1. Download the Saul-7B model (~4.5 GB, one-time)
2. Download the BGE-small embedding model (~33 MB, one-time)
3. Check/install OCR runtime (Tesseract) for image text extraction when possible

If OCR auto-install cannot complete on your platform, the setup screen shows manual instructions.

## Build

### Desktop app

```bash
npm run build:app
```

Compiles the app and runs Tauri bundling (produces platform-specific installers).

### Marketing website

```bash
npm run build:website
```

Builds the Next.js marketing site for production.

## Development Commands

```bash
npm run app                        # Tauri dev (Rust + Vite HMR)
npm run website                    # Next.js dev server at localhost:3000
cd app/src-tauri && cargo check    # Type-check Rust backend
cd app/src-tauri && cargo test     # Run Rust unit tests
```

## Project Structure

```
/
├── app/
│   ├── src-tauri/src/             # Rust backend
│   │   ├── lib.rs                 # Tauri setup, state init
│   │   ├── state.rs               # RagState, AppSettings, domain types
│   │   ├── pipeline.rs            # RAG pipeline (chunk, embed, retrieve, generate)
│   │   ├── assertions.rs          # Citation validation, hallucination detection
│   │   └── commands/              # Tauri IPC command handlers
│   │       ├── rag.rs
│   │       └── doc_parser.rs
│   └── src/renderer/src/          # React frontend
│       ├── App.tsx                # Root state management
│       ├── api.ts                 # Tauri invoke() shim
│       └── components/
│           ├── ChatInterface.tsx   # Main chat UI
│           ├── MessageBubble.tsx   # Markdown message rendering
│           ├── Sidebar.tsx         # Sessions + case management
│           ├── Settings.tsx        # Config + practice area presets
│           └── ModelSetup.tsx      # First-launch download screen
├── shared/src/types.ts            # Shared TypeScript types
├── website/                       # Next.js marketing site
└── package.json                   # Workspace root
```

## Troubleshooting

### First query is slow
The LLM loads into memory on the first query after launch. Subsequent queries are much faster. On machines with 8 GB RAM, initial load may take 15-30 seconds.

### Model download fails or stalls
Models download to `{app_data}/models/`. If a download is interrupted, delete the partial file and restart the app to re-trigger the download. Check your internet connection — this is the only time the app requires network access.

### OCR not working for images
Image text extraction requires Tesseract. The app attempts to install it automatically on first run. If auto-install fails:
- **macOS**: `brew install tesseract`
- **Ubuntu/Debian**: `sudo apt install tesseract-ocr`
- **Windows**: Download from [UB Mannheim](https://github.com/UB-Mannheim/tesseract/wiki)

### App crashes on launch
Ensure you have the required Tauri platform dependencies installed. See [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

## Security & Privacy

- All parsing, embedding, and LLM inference run locally
- Zero cloud dependencies in the query flow
- Documents never leave the machine
- File parsing includes format validation and hardening checks
- XML parser rejects unsafe DTD/entity constructs
- OCR uses local Tesseract runtime

## Documentation

- [User Guide](docs/USER_GUIDE.md)
- [Installation Guide](docs/INSTALLATION.md)
- [Contributing](docs/CONTRIBUTING.md)

## License

MIT

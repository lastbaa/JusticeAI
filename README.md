# Justice AI

Private, citation-grounded legal document research on your local machine.

## What It Is

Justice AI is a desktop app for legal professionals to search loaded case documents and answer questions with source citations.

- Runs locally (Tauri + Rust backend + React UI)
- Uses local embedding + local LLM inference
- Returns answers with file/page/excerpt citations

> Important: Justice AI is a research assistant, not legal advice. Attorneys remain responsible for legal conclusions.

## Current Architecture

- `app/src-tauri` — active desktop backend (Rust + Tauri)
- `app/src/renderer` — active desktop frontend (React + Vite)
- `shared` — shared TypeScript models/contracts
- `website` — marketing site

Note: legacy Electron files still exist in `app/src/main` and `app/src/preload`, but the active runtime is Tauri.

## Supported File Types

- `.pdf`
- `.docx`
- `.txt`
- `.md`
- `.csv`
- `.eml`
- `.html`
- `.htm`
- `.mhtml`
- `.xml`
- `.xlsx`
- `.png`
- `.jpg`
- `.jpeg`
- `.tif`
- `.tiff`

## Setup

### Prerequisites

- Node.js 20+
- Rust toolchain (for Tauri builds)
- Platform build tools required by Tauri

### Install dependencies

From repo root:

```bash
npm install
```

## Run the App (Development)

From repo root:

```bash
npm run app
```

On first run, the app setup flow will:

1. Download the Saul model (one-time)
2. Check/install OCR runtime (Tesseract) for image text extraction when possible

If OCR auto-install cannot complete on your platform, setup shows manual instructions.

## Build

From repo root:

```bash
npm run build:app
```

This compiles the app and runs Tauri bundling.

From repo root:

```bash
npm run build:website
```

Builds the Next.js marketing site.

## How Retrieval Works

1. Parse local documents
2. Normalize/chunk content for retrieval efficiency
3. Embed chunks locally
4. Retrieve top relevant evidence
5. Generate local answer with citations

## Security & Privacy Notes

- Parsing and retrieval run locally
- No cloud inference dependency in normal query flow
- File parsing includes format validation and hardening checks
- XML parser rejects unsafe DTD/entity constructs
- OCR uses local Tesseract runtime

## Useful Development Commands

Desktop app dev:

```bash
npm run app
```

Rust check/tests (desktop backend):

```bash
cd app/src-tauri
cargo check
cargo test
```

## License

MIT

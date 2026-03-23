# Changelog

All notable changes to Justice AI are documented here.

## [1.4.0] — 2026-03-23

### Added
- Case deletion confirmation dialog with keep/delete options
- Windows and Linux cross-platform compatibility
- DEB package support for Linux
- GPU fallback and XDG data directory support on Linux
- Legal jurisdiction detection, hierarchy prompt injection, and UI selector
- Citation confidence scores, query templates, and key facts extraction
- Rubric compliance section on website

### Fixed
- Hardened file operations for Windows edge cases
- Cross-platform CI pipeline (Vulkan SDK, system deps)
- Missing closing brace in Sidebar onKeyDown handler

## [1.3.0] — 2026-03

### Added
- Case folders with scoped retrieval
- Improved session naming and UI polish
- PDF highlighting in document viewer
- Key Sources feature
- LLM response quality improvements (formatting instructions, sampler tuning, post-processing)
- Maximize window on launch, redesign cases as projects

### Changed
- Smooth download progress bar with speed and ETA display
- Deduplicated citations by page
- Replaced gavel logos with scales of justice
- Updated download link and version badge to v1.4.0

## [1.2.0] — 2026-02

### Added
- Streaming token output with pipeline status indicators
- Neighbor chunk expansion for better context
- Citation confidence scores in responses
- Multi-turn conversation context
- Pluggable retrieval backend (`RetrievalBackend` trait) with eval system overhaul
- Coordinate-aware PDF parser and RAG pipeline refactor
- Evaluation harness with 77 test cases across 8 fixtures
- Secure multi-format ingestion (DOCX, CSV, HTML, EML, XLSX) and OCR bootstrap

### Fixed
- 3 bugs found by unit tests, added 23 new tests
- 8 chunker/pipeline correctness issues
- PDF text normalization for wonky encodings

### Changed
- Paragraph-aware chunking with BGE score thresholds
- Upgraded embedding model to BGE-small-en-v1.5

## [1.1.0] — 2026-01

### Added
- MMR diversity reranking for retrieval results
- Abbreviation-aware chunking

### Fixed
- Garbled LLM output and non-response issues
- PDF font-encoding garbage characters
- SIGABRT crash from ggml_abort in llama context decode
- Metal GPU offload and context reduction for stability
- RAG retrieval quality for structured document facts

### Changed
- Replaced lopdf PDF extraction with pdf-extract (lopdf fallback)
- Reduced MAX_CONTEXT_CHARS to prevent prompt truncation

## [1.0.0] — 2025-12

### Added
- Tauri 2 desktop app with Rust backend (migrated from Electron)
- Fully local LLM inference via Saul-7B-Instruct Q4_K_M (llama-cpp-2)
- Local embeddings via fastembed (BGE-small-en-v1.5)
- Zero-config setup — no HuggingFace token or API keys required
- First-launch model download with progress tracking
- Document viewer panel with PDF rendering and highlighted citations
- Hybrid BM25 + cosine similarity retrieval with Reciprocal Rank Fusion
- Smart session naming
- Two-panel layout with citation highlighting
- macOS DMG release (v1.0.0)

### Changed
- Complete UI overhaul — removed onboarding, redesigned welcome screen

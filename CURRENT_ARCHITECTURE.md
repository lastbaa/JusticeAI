# Current Architecture (March 13, 2026)

## 1) Repository Reality Check

- **Current desktop runtime is Tauri + Rust backend + React renderer** (`app/src-tauri` + `app/src/renderer`).
- **Legacy Electron code is still tracked** (`app/src/main`, `app/src/preload`, `app/electron.vite.config.ts`) but is not the active runtime path for current `npm run app`.
- Website and security prototype code exist in parallel but are separate from desktop runtime.

## 2) High-Level Architecture

### Desktop app (active)

- **Frontend:** React + TypeScript + Vite (`app/src/renderer`).
- **Bridge/API:** Tauri `invoke` + event listeners (`app/src/renderer/src/api.ts`).
- **Backend:** Rust commands and state management (`app/src-tauri/src/commands`, `state.rs`, `pipeline.rs`).
- **RAG flow:** parse files -> chunk -> embed (`fastembed`) -> retrieve (hybrid backends) -> prompt Saul GGUF (`llama-cpp-2`) -> citations + streaming tokens/events.
- **Persistence:** JSON state in app data directory (`chunks`, `settings`, `sessions`, embedding model marker).
- **Viewer support:** local loopback file server for PDF rendering.

### Desktop app (legacy, still present)

- Electron main/preload/services implement an older architecture that used IPC + Vectra + HF/Ollama paths.
- Kept in repo, useful as migration history/reference, but diverges from active Tauri command surface.

## 3) File-by-File Role Map (tracked files)

## Root files

- `.github/workflows/deploy.yml` - GitHub Pages deployment workflow for `website`.
- `.gitignore` - ignore rules for node/tauri/next/build artifacts.
- `README.md` - project overview; partially stale vs current Tauri runtime.
- `CLAUDE.md` - collaborator guidance; partially stale (Electron-era assumptions).
- `BUG_CONTEXT.md` - deep investigation of PDF extraction bug and mitigation context.
- `USABILITY_REPORT.md` - UX/accessibility audit and prioritized improvement roadmap.
- `research_query.md` - retrieval/re-ranking research framing and experiment context.
- `package.json` - workspace root (`website`, `app`, `shared`) scripts.
- `package-lock.json` - npm dependency lockfile.

## Saul/ model helper scripts

- `Saul/download_gguf.py` - downloads base GGUF model from HuggingFace.
- `Saul/download_instruct.py` - downloads instruct GGUF model from HuggingFace.

## Shared package

- `shared/package.json` - shared types package metadata.
- `shared/src/types.ts` - cross-layer DTOs/types (files, settings, sessions, query results, IPC constants).

## Security prototype (separate utility)

- `security/demo.py` - CLI demo for encrypted vault flows (init/store/verify).
- `security/vault/__init__.py` - vault module exports.
- `security/vault/core.py` - encrypted vault implementation (scrypt KDF + AES-GCM + AAD + atomic writes).

## App workspace: configs and build

- `app/package.json` - app scripts/dependencies for Tauri + renderer.
- `app/Modelfile.saul-instruct` - Ollama modelfile template for Saul instruct behavior.
- `app/vite.config.ts` - active renderer Vite config for Tauri.
- `app/electron.vite.config.ts` - legacy Electron build config.
- `app/tailwind.config.ts` - renderer Tailwind theme config.
- `app/postcss.config.js` - Tailwind/PostCSS setup.
- `app/tsconfig.json` - TS project references.
- `app/tsconfig.node.json` - TS config for legacy node/electron files.
- `app/tsconfig.vite.json` - TS config for vite config typing.
- `app/tsconfig.web.json` - TS config for renderer source.

## App workspace: active renderer UI

- `app/src/renderer/index.html` - renderer host document + CSP meta.
- `app/src/renderer/public/pdf.worker.min.mjs` - PDF.js worker asset.
- `app/src/renderer/src/main.tsx` - React entry point, global API wiring, top-level error handling.
- `app/src/renderer/src/App.tsx` - primary app state/orchestration (sessions, files, query flow, model setup, export, toasts).
- `app/src/renderer/src/api.ts` - Tauri API shim (`invoke` + event listeners).
- `app/src/renderer/src/globals.css` - global styles/theme variables/animations.
- `app/src/renderer/src/theme.ts` - small theme utility helpers.
- `app/src/renderer/src/types/window.d.ts` - `window.api` typing contract.

### Renderer components

- `app/src/renderer/src/components/Sidebar.tsx` - session list, search, rename/delete controls, nav actions.
- `app/src/renderer/src/components/ChatInterface.tsx` - chat input/thread shell, loading state, query UX.
- `app/src/renderer/src/components/MessageBubble.tsx` - user/assistant message rendering and markdown-ish output.
- `app/src/renderer/src/components/ContextPanel.tsx` - source citation/context panel.
- `app/src/renderer/src/components/SourceCard.tsx` - individual citation card UI and copy action.
- `app/src/renderer/src/components/DocumentViewer.tsx` - PDF/DOCX content viewing from citation/file context.
- `app/src/renderer/src/components/Settings.tsx` - settings editor (retrieval params/theme/build info).
- `app/src/renderer/src/components/ModelSetup.tsx` - model readiness/download UX.
- `app/src/renderer/src/components/Toast.tsx` - transient notifications.
- `app/src/renderer/src/components/ErrorBoundary.tsx` - renderer crash containment.

## App workspace: legacy Electron layer (not active runtime)

- `app/src/main/index.ts` - Electron app lifecycle + IPC handlers + legacy service wiring.
- `app/src/main/services/docParser.ts` - legacy PDF/DOCX parsing path.
- `app/src/main/services/ollama.ts` - legacy Ollama tags/embedding client.
- `app/src/main/services/ragPipeline.ts` - legacy in-process RAG with Vectra.
- `app/src/main/services/saul.ts` - legacy HF chat completion wrapper.
- `app/src/preload/index.ts` - Electron contextBridge API exposure.

## App workspace: active Tauri backend

### Core rust files

- `app/src-tauri/Cargo.toml` - rust crate metadata, binaries, dependencies.
- `app/src-tauri/Cargo.lock` - rust lockfile.
- `app/src-tauri/build.rs` - build metadata embedding (git hash + UTC build timestamp).
- `app/src-tauri/tauri.conf.json` - Tauri app/window/bundle config.
- `app/src-tauri/.gitignore` - rust target/schema ignores.
- `app/src-tauri/.cargo/config.toml` - macOS deployment target env override for llama.cpp compatibility.
- `app/src-tauri/EVAL_SYSTEM_DESIGN.md` - architecture spec for evaluation harness/metrics.
- `app/src-tauri/src/main.rs` - binary entry point calling library `run()`.
- `app/src-tauri/src/lib.rs` - Tauri builder setup, command registration, close-intercept flow, local PDF file server setup.
- `app/src-tauri/src/state.rs` - canonical persisted state structs, save/load helpers, similarity/excerpt/page text utilities.
- `app/src-tauri/src/pipeline.rs` - embedding, prompt generation, chunking, sentence splitting, retrieval backends, hybrid/MMR/RRF logic.
- `app/src-tauri/src/assertions.rs` - answer assertion checks (citation format, number checks, blocklist/hallucination heuristics).

### Rust command modules

- `app/src-tauri/src/commands/mod.rs` - command module exports.
- `app/src-tauri/src/commands/ollama.rs` - model/daemon status command.
- `app/src-tauri/src/commands/doc_parser.rs` - robust PDF/DOCX extraction pipeline (pdf-extract + lopdf fallback + AcroForm handling + text normalization).
- `app/src-tauri/src/commands/rag.rs` - Tauri command handlers for models/files/query/settings/sessions/export; coordinates state + pipeline.

### Rust tools/tests

- `app/src-tauri/src/bin/harness.rs` - CLI retrieval evaluation harness (reports, compare mode, backends, metrics).
- `app/src-tauri/tests/pipeline_integration.rs` - end-to-end and regression tests for parsing/chunking/retrieval behaviors.

### Tauri capabilities/schemas

- `app/src-tauri/capabilities/default.json` - desktop capability permissions (core/window/dialog save/confirm).
- `app/src-tauri/gen/schemas/acl-manifests.json` - generated ACL schema.
- `app/src-tauri/gen/schemas/capabilities.json` - generated capabilities schema.
- `app/src-tauri/gen/schemas/desktop-schema.json` - generated desktop schema.
- `app/src-tauri/gen/schemas/macOS-schema.json` - generated macOS schema.

### Tauri test fixtures

- `app/src-tauri/tests/fixtures/eval.json` - query/expected retrieval fixture suite (incl. negative cases).
- `app/src-tauri/tests/fixtures/eval_baseline.json` - saved baseline run output for comparison.
- `app/src-tauri/tests/fixtures/generate_eval_fixtures.py` - synthetic eval PDF generator.
- `app/src-tauri/tests/fixtures/generate_test_pdfs.py` - synthetic parser-regression PDF generator.
- `app/src-tauri/tests/fixtures/plain_contract.pdf` - synthetic plain text contract fixture.
- `app/src-tauri/tests/fixtures/filled_form_simple.pdf` - synthetic filled-form fixture.
- `app/src-tauri/tests/fixtures/multipage_form.pdf` - synthetic multi-page form fixture.
- `app/src-tauri/tests/fixtures/confusable_lease.pdf` - synthetic confusable-entity/date stress fixture.
- `app/src-tauri/tests/fixtures/dense_nda.pdf` - synthetic dense-legal-text fixture.
- `app/src-tauri/tests/fixtures/settlement_breakdown.pdf` - synthetic numeric/settlement fixture.
- `app/src-tauri/tests/fixtures/bartending_contract.pdf` - real-world style contract fixture.
- `app/src-tauri/tests/fixtures/irs_w4.pdf` - real tax form fixture.
- `app/src-tauri/tests/fixtures/irs_w9.pdf` - real tax form fixture.
- `app/src-tauri/tests/fixtures/irs_w9_filled.pdf` - filled tax form fixture.
- `app/src-tauri/tests/fixtures/ga_statement_of_claim.pdf` - court form fixture.

### Tauri icons/assets

- `app/src-tauri/icons/*` top-level png/icns/ico/store/tile files - desktop packaging icons for macOS/Windows stores.
- `app/src-tauri/icons/android/**` - Android launcher icon variants/background XML.
- `app/src-tauri/icons/ios/**` - iOS app icon size variants.

## Website workspace

### Website root/config

- `website/.gitignore` - website-local ignore rules.
- `website/package.json` - Next.js app dependencies/scripts.
- `website/next.config.js` - build settings incl. GitHub Pages base path/asset prefix.
- `website/postcss.config.js` - Tailwind/PostCSS setup.
- `website/tailwind.config.ts` - website Tailwind theme extensions.
- `website/tsconfig.json` - TypeScript config.
- `website/next-env.d.ts` - Next TypeScript environment types.
- `website/lib/utils.ts` - shared `cn` class-merging utility.

### Website app/router files

- `website/app/layout.tsx` - root HTML shell + metadata.
- `website/app/page.tsx` - homepage composition from section components.
- `website/app/globals.css` - global website styles/animations.
- `website/app/changelog/page.tsx` - release/changelog page with version notes.

### Website section components

- `website/app/components/Navbar.tsx` - top navigation.
- `website/app/components/Hero.tsx` - hero section.
- `website/app/components/Marquee.tsx` - animated trust/feature marquee.
- `website/app/components/ProductDemo.tsx` - product walkthrough/visual demo section.
- `website/app/components/VaporizeStats.tsx` - animated headline/stat section.
- `website/app/components/FeaturesGrid.tsx` - feature cards section.
- `website/app/components/BentoCapabilities.tsx` - capability bento grid.
- `website/app/components/Compare.tsx` - comparison section.
- `website/app/components/UseCases.tsx` - use case section.
- `website/app/components/HowItWorks.tsx` - process explanation section.
- `website/app/components/LampCTA.tsx` - stylized CTA section.
- `website/app/components/FAQ.tsx` - FAQ section.
- `website/app/components/Download.tsx` - release download links section.
- `website/app/components/Footer.tsx` - footer.
- `website/app/components/Reveal.tsx` - scroll reveal utility wrapper.
- `website/app/components/Typewriter.tsx` - typewriter text effect.
- `website/app/components/WordReveal.tsx` - word-by-word reveal animation.
- `website/app/components/ValueProps.tsx` - value proposition section cards.

### Website UI primitives

- `website/components/ui/glowing-effect.tsx` - interactive glow border effect.
- `website/components/ui/lamp.tsx` - lamp/light cone animated container.
- `website/components/ui/vapour-text-effect.tsx` - canvas vaporize text animation.

### Website public assets

- `website/public/favicon.ico` - browser favicon ICO.
- `website/public/favicon.png` - PNG icon.
- `website/public/favicon.svg` - SVG icon.
- `website/public/releases/JusticeAI-1.0.0-mac.zip` - downloadable release artifact placeholder.
- `website/public/releases/JusticeAI-1.0.0-win.zip` - downloadable release artifact placeholder.
- `website/public/releases/JusticeAI-1.0.0-linux.zip` - downloadable release artifact placeholder.

## 4) Maintenance Guidance

- Treat `app/src-tauri` + `app/src/renderer` as source of truth for desktop behavior.
- Keep `shared/src/types.ts` and Rust `state.rs` contracts aligned to avoid UI/runtime drift.
- When keeping legacy Electron files, label them clearly as reference-only to avoid accidental edits against inactive code paths.
- Update this file when command names, persistence schema, or active runtime entrypoints change.

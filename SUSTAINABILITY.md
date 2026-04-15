# Sustainability

Justice AI is designed for long-term sustainability across environmental, economic, and technical dimensions.

## Privacy & Data Sovereignty

All processing runs locally on the user's device. No data is sent to external servers, no API keys are required, and no cloud costs are incurred. This eliminates ongoing operational expenses and ensures user data never leaves their machine.

## Resource Efficiency

- **On-demand model loading**: The LLM (Qwen3-8B, ~5 GB) and embedding model (~33 MB) are downloaded once and cached locally
- **Quantized inference**: Q4_K_M quantization reduces memory footprint by ~75% compared to full-precision models while maintaining quality
- **Efficient retrieval**: Hybrid BM25 + cosine similarity with Reciprocal Rank Fusion avoids expensive reranking API calls
- **No always-on services**: No background daemons, databases, or cloud subscriptions required

## Technical Maintainability

- **Type safety**: Rust backend with strict compiler checks prevents entire classes of runtime errors. TypeScript frontend with shared IPC types ensures contract consistency
- **Automated testing**: 37+ Rust unit tests, frontend component tests via Vitest, and an evaluation harness with 77 cases across 8 document fixtures
- **CI/CD pipeline**: GitHub Actions runs checks on macOS, Windows, and Linux. Includes `cargo audit` for dependency security scanning
- **Conventional commits**: Structured commit history enables automated changelog generation and clear audit trails

## Accessibility

- Keyboard-navigable interface with ARIA labels on interactive elements
- Light and dark theme support for visual comfort
- Screen reader compatible toast notifications with `role="status"` and `aria-live="polite"`
- High-contrast gold-on-dark color scheme meeting WCAG guidelines

## Cross-Platform Support

Justice AI builds natively for macOS (DMG), Windows (MSI/EXE), and Linux (DEB), ensuring broad accessibility without platform lock-in. The Tauri framework produces lightweight binaries (~15 MB before model download) compared to Electron alternatives.

## Future Sustainability

- Modular `RetrievalBackend` trait allows swapping retrieval strategies without changing the application layer
- Evaluation harness enables regression testing when updating models or retrieval parameters
- MIT license encourages community contributions and long-term maintenance

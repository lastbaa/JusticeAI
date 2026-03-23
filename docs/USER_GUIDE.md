# Justice AI User Guide

Justice AI is a privacy-first legal research desktop app. All document processing, embeddings, and LLM inference run entirely on your machine.

## Getting Started

When you first launch Justice AI, a setup screen will appear to download the required models:

1. **Saul-7B LLM** (~4.5 GB) -- powers answer generation
2. **BGE-Small embedding model** (~33 MB) -- powers semantic search

Click **Download Models** and wait for both downloads to complete. This is a one-time process; subsequent launches skip this step.

Once models are ready, you'll see the main interface: a sidebar on the left, a context panel, and the chat area.

## Loading Documents

Upload documents by clicking the **+** button in the sidebar or dragging files into the app.

**Supported file types:**

| Category | Extensions |
|----------|-----------|
| Documents | PDF, DOCX, TXT, MD |
| Data | CSV, XLSX, XML |
| Email | EML, MHTML |
| Web | HTML, HTM |
| Images (OCR) | PNG, JPG, JPEG, TIF, TIFF |

Documents are parsed and chunked locally. Embeddings are generated on-device and stored in a local vector index.

## Asking Questions

Type a question in the chat input and press **Enter**. Justice AI will:

1. Search your uploaded documents using hybrid BM25 + semantic retrieval
2. Retrieve the most relevant passages
3. Generate a citation-grounded answer using the Saul-7B legal LLM

**Example questions:**
- "What are the termination clauses in this contract?"
- "Summarize the key obligations of the tenant in the lease."
- "What is the indemnification provision?"
- "Are there any non-compete restrictions?"

Answers include citations pointing to specific document chunks so you can verify every claim.

## Managing Cases

Cases let you organize documents and chat sessions by matter.

- **Create a case** from the sidebar to group related documents together
- **Sessions** within a case keep separate lines of inquiry organized
- Retrieval is scoped to the active case's documents, keeping results focused
- **Delete a case** by right-clicking it in the sidebar. You'll be prompted to either keep the associated files and sessions (moved to the general workspace) or delete everything permanently.
- **Rename a case** by double-clicking its name in the sidebar.

## Document Viewer

Click a citation in a chat response to open the **Document Viewer** panel on the right side. This shows the source passage highlighted in context, so you can quickly verify the AI's answer against the original document.

## Exporting

- **Export chat**: Click the export icon in the chat header to save the conversation as a text file
- **Export citations**: Click "Export" in the context panel to save retrieved source passages

## Settings

Access settings via the gear icon. Available options:

| Setting | Description | Default |
|---------|-------------|---------|
| Chunk Size | Token count per document chunk | 500 |
| Chunk Overlap | Overlap between consecutive chunks | 50 |
| Top-K | Number of retrieved passages per query | 5 |
| Practice Area | Presets that tune retrieval for specific legal domains | General |
| Jurisdiction | Auto-detected from documents or manually selectable — tailors prompts to jurisdiction-specific legal terminology | Auto |
| Theme | Light or dark appearance | System |

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Enter` | Send message |
| `Shift + Enter` | Insert newline in message |
| `Esc` | Close settings / panels |

## Privacy & Security

Justice AI is designed with privacy as a core principle:

- **Documents never leave your machine.** Parsing, chunking, and embedding all happen locally.
- **LLM inference is local.** The Saul-7B model runs on-device via llama.cpp -- no cloud API calls.
- **No telemetry.** The app makes no network requests after the initial model download.
- **Encrypted storage.** Chat history is stored with AES encryption on disk.

## Troubleshooting

### Model download fails or stalls
- Check your internet connection. The initial download requires ~4.5 GB.
- Restart the app to resume the download. Progress is tracked automatically.
- If downloads repeatedly fail, check that your firewall is not blocking outbound HTTPS.

### First query is slow
- The LLM needs to load into memory on the first query after launch. Subsequent queries are faster.
- On machines with 8 GB RAM, expect longer load times. 16 GB is recommended.

### OCR / image documents
- Image files (PNG, JPG, TIFF) are processed with OCR. Quality depends on image resolution and clarity.
- For best results, use high-resolution scans with clear text.

### App won't start
- Ensure your OS meets the minimum requirements (macOS 12+, Windows 10+, Ubuntu 22.04+).
- On macOS, you may need to allow the app in **System Preferences > Security & Privacy** after first launch.
- On Linux, ensure required system libraries are installed (see [Installation Guide](INSTALLATION.md)).

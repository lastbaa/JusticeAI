# Justice AI User Guide

Justice AI is a local-first legal document research assistant. It searches the documents you load into the app and answers questions with source citations.

It is best used as a document-grounded research and review tool, not as a general legal oracle.

## Best Use Cases

Justice AI is a good fit when you want to:

- search a case file, contract set, policy set, or regulatory materials you already have
- summarize a document or set of related documents
- locate clauses, dates, parties, amounts, or obligations
- compare loaded documents inside the same matter
- ask document-grounded questions and verify the answer with citations
- review OCR-able scans or image documents locally

Examples:

- "Summarize the indemnification obligations in these contracts."
- "What deadlines appear across the loaded lease documents?"
- "Which document mentions termination for cause?"
- "What evidence in these files supports the plaintiff's timeline?"

## Outside The Scope Of The App

Justice AI is not designed to:

- replace a lawyer's judgment
- provide legal advice on its own
- guarantee that an answer is correct just because it sounds plausible
- search the public web or pull current law that you have not loaded
- act as a complete e-discovery platform for very large productions
- reliably fix poor scans, missing pages, or unreadable source documents
- make final compliance conclusions without human review

Important boundary:

If a rule, statute, contract, exhibit, or policy is not loaded into the app, Justice AI should not be treated as having authoritative access to it.

## Hardware Recommendations

Recommended for smooth use:

- 16 GB RAM or more(16 GB will run well for shorter documents, but more RAM is highly reccomended for more demanding workflows)
- Apple Silicon Mac or a modern 8+ core x86 CPU
- 10 GB+ free disk space
- SSD storage



What to expect:

- the first model download is large
- the first query after launch is slower because the model must load into memory
- larger documents and broader questions take longer than narrow, source-specific questions

## First Launch

On first launch, Justice AI will guide you through model setup.

The app downloads:

- the local LLM used for answer generation
- the local embedding model used for search
- OCR support when available

This is usually a one-time setup.

## Document Ingestion

### Step 1: Load files

Use the file picker or drag files into the app.

Common supported inputs include:

- PDF
- DOCX
- TXT / MD
- CSV / XLSX
- XML / HTML / EML
- PNG / JPG / TIFF via OCR

### Step 2: Wait for indexing

During ingestion, the app:

1. parses the file
2. extracts text
3. splits the text into chunks
4. creates local embeddings for search
5. stores the indexed result locally

You should wait for ingestion to finish before asking document-dependent questions.

### Step 3: Organize if needed

If you are working on a specific matter:

- create or select a case
- keep related files together
- assign document roles when useful

This improves organization and helps keep retrieval focused.

## Prompting Guide

Justice AI works best with focused, document-grounded prompts.

### Good prompting habits

- name the thing you want to find
- mention the document set or issue directly
- ask for support from the loaded documents
- ask for comparison when multiple files matter
- ask for citations when you need verification

### Better prompt patterns

Instead of:

- "Tell me about this."

Use:

- "Summarize the termination provisions in the loaded contracts."
- "Compare the two vendor agreements on indemnity and limitation of liability."
- "What evidence in these files supports the payment dispute timeline?"
- "Identify the sections of the loaded regulation that relate to chemical storage and runoff."

### When multiple documents are loaded

Be explicit about comparison.

Examples:

- "Compare Document A and Document B on notice requirements."
- "Which loaded file contains the strongest evidence of breach?"
- "List the differences between the compliant and non-compliant reports."

## What A Good Session Looks Like

1. Load the documents for one matter or one clear task.
2. Confirm ingestion is complete.
3. Start with a focused summary question.
4. Drill down with narrower follow-up questions.
5. Open citations and verify important claims in the source text.
6. Export or save the conversation if needed.

## How To Get Better Results

- load the actual governing documents, not just related background material
- avoid mixing unrelated matters into one case
- ask narrower questions when a broad prompt returns noisy results
- use good-quality scans when OCR is involved
- verify citations for important conclusions

For regulatory or compliance review:

- load the exact regulation sections you want compared
- load the business or factual documents separately
- ask for obligation-specific analysis rather than generic "is this compliant?" prompts when possible

## Privacy And Data Handling

Justice AI is designed to run locally.

- document parsing runs locally
- embeddings are created locally
- answer generation runs locally
- documents are stored on your machine

The main network-heavy step is initial model download and related setup.

## Short Troubleshooting

### The first query is slow

This is normal. The model is loading into memory.

### Search quality seems weak

- check that the right documents are loaded
- narrow the prompt
- verify OCR quality if the source was an image or scan

### A result sounds plausible but unhelpful

Treat it as a draft, not a conclusion. Open the cited source text and verify it.

### OCR documents perform poorly

Use cleaner scans, higher resolution, and pages with readable contrast.

## Final Reminder

Justice AI is most useful when it is treated as:

- a local document search assistant
- a citation-grounded drafting and review tool
- a way to accelerate review of materials you have already loaded

It is least useful when treated as:

- a substitute for legal analysis
- a source of law that has not been loaded
- a guarantee that an answer is complete or correct without source verification

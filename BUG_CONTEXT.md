# Justice AI — Bug Investigation Context

## System Architecture

Tauri 2 + Rust desktop app. Pipeline:

```
PDF/DOCX → parse → chunk → embed (BGE-small-en-v1.5) → cosine+MMR retrieve → Qwen3-8B → answer
```

All processing is local. No cloud calls except the original model download.

### Key Files

| File | Purpose |
|------|---------|
| `app/src-tauri/src/commands/doc_parser.rs` | PDF parsing — pdf-extract crate primary, lopdf fallback |
| `app/src-tauri/src/pipeline.rs` | Chunking, embedding, retrieval, LLM inference |
| `app/src-tauri/src/commands/rag.rs` | Tauri command handlers (thin wrappers around pipeline) |
| `app/src-tauri/src/state.rs` | RagState, AppSettings, all domain types |

### Settings Defaults

- `chunk_size`: 1000 chars
- `chunk_overlap`: 300 chars
- `top_k`: 6
- Embedding: BGE-small-en-v1.5 (fastembed, ~33MB ONNX)
- LLM: Qwen3-8B Q4_K_M GGUF (~5GB), context 32K tokens (originally Saul-7B, upgraded April 2026)

---

## Known Bugs

### Bug 1 — Filled form fields detached from labels (HIGH)

PDFs with typed-in form data have two text layers: the template (blank underlines) and the filled values. `pdf-extract` outputs them as separate text objects. The resulting text stream looks like:

```
Event Date: __________________  Event Time: ___________________  Guest Count: ___________
Event Location: ________________________________  Total Fee Contracted: ____________
...rest of template boilerplate...

Liam Neild  Party  williamaneild@gmail.com  Sat 2.28.26  3-7pm  101-125  $275 as signing
2/28/2026  2/25/2026  412-753-7609  $275
18 Eagle Row
Atlanta, GA
```

The filled values are dumped at the end with no label context. The chunker creates separate chunks for the template text and the filled values. The LLM sees `"Event Date: ______________"` in one chunk and `"Sat 2.28.26"` in a different chunk with no explicit connection between them.

**Effect:** The LLM either reports blanks (`$________`) or picks the wrong value.

---

### Bug 2 — Wrong date returned (HIGH, known regression)

When asked *"what date is the event?"*, the system returns **2/25/2026** (client signature date) instead of **2/28/2026** (the event date, written as `Sat 2.28.26` in the form).

**Root cause:** The filled-values dump contains `"2/28/2026  2/25/2026"` back-to-back — Forever Moore's signature date followed by the client's signature date. The event date `Sat 2.28.26` appears earlier in the dump but in a non-obvious format. The LLM associates the most prominent date pair with "the event date" and picks the wrong one.

**Regression guard:** `NotContains("2/25")` assertion in the test suite.

---

### Bug 3 — Repetitive / restarting response (MEDIUM)

The LLM response restarts numbered lists 4 times, repeating the same 3–4 facts each time:

```
1. The event is scheduled for 2/25/2026.
2. The deposit fee of $________ is due...
3. The deposit is nonrefundable...

1. The event is scheduled for 2/25/2026.
2. The deposit fee of $________ is due...
3. The deposit is nonrefundable...
4. The event location is _________.
...
```

**Suspected causes:**
- MMR retrieval still returning near-identical chunks from the same single-page document (LLM summarizes each independently)
- Or: repetition penalty isn't strong enough at low temperature to prevent loop behavior when context contains near-duplicate passages

---

## Test Document

**Path:** `/Users/liamneild/Desktop/Liam Neild 2.28.26 Bartending Contract.pdf`
**Also at:** `app/src-tauri/tests/fixtures/bartending_contract.pdf`

### Ground Truth Facts

| Field | Value |
|-------|-------|
| Client | Liam Neild |
| Event | Party |
| Event date | Sat 2.28.26 (February 28, 2026) |
| Event time | 3–7pm (bartender serves 2pm–6pm) |
| Location | 18 Eagle Row, Atlanta, GA |
| Total fee | $275 (full amount due at signing) |
| Guest count | 101–125 |
| Additional hours | $50/hr |
| Company | Forever Moore Ent (FME) |
| Owner | Sharina Moore |
| FME signature date | 2/28/2026 |
| Client signature date | 2/25/2026 |
| Email | williamaneild@gmail.com |
| Phone | 412-753-7609 |
| Governing law | Georgia State Liquor laws |
| Cancellation | Nonrefundable; transferable with 30-day notice |

---

## Current System Prompt (`RULES_PROMPT`)

```
You are Justice AI, a knowledgeable legal research assistant specializing in US federal
and state law. You help attorneys, paralegals, and individuals understand legal documents
and answer legal questions.

When document excerpts are provided in the user message:
- Cite every factual claim inline as [filename, p. N] right after the claim — never
  group citations at the end.
- State all numbers, dates, dollar amounts, and figures EXACTLY as written in the
  source — never round or paraphrase them.
- If the answer is not present in the excerpts, say: "I could not find information
  about this in your loaded documents."

When no document excerpts are provided:
- Answer from your knowledge of US law and general knowledge. Be helpful and direct.
- For legal questions, note when the answer may vary by state or when consulting a
  licensed attorney is advisable.

In all cases: never fabricate case citations, statutes, or facts. You are a research
and information tool — do not give specific legal advice.
```

---

## Test Suite

13 test cases defined in `app/src-tauri/tests/pipeline_integration.rs`.

**Tier structure:**

| Tier | What | Run with |
|------|------|----------|
| 1 — Extraction | Parse PDF, check raw text contains key facts | `cargo test` |
| 2 — Chunking | Verify facts survive chunking | `cargo test` |
| 3 — Retrieval | Embed query, check right chunks retrieved | `cargo test -- --include-ignored` |
| 4 — E2E | Full pipeline including LLM | `cargo test -- --include-ignored` |

Tiers 1 + 2 run instantly with no model files (20 tests total).

---

## PDF Structure Findings (Confirmed)

The bartending contract PDF has exactly this structure:

- **1 large Form XObject** — entire template (labels + blank underlines + boilerplate, ~10,760 chars)
- **14 separate Form XObjects** — one per filled field (each containing a single value)

Content stream renders them: **template first, then all 14 filled values in a batch**. Result in text stream:

```
Event Date: __________________  Event Time: ___________________  Guest Count: ___________
...~2300 chars of boilerplate...
Sat 2.28.26
3-7pm
101-125
```

### Coordinate data (from XObject placement matrices)

| Field | PDF y | x | Value |
|-------|-------|---|-------|
| "Event Date:" label | 603.2 | ~100 | (template) |
| Filled event date | 611.9 | 131.2 | Sat 2.28.26 |
| FME signature date | 150.9 | 414.4 | 2/28/2026 |
| Client signature date | 118.3 | 410.0 | 2/25/2026 |

Every label/value pair is within ≤9 PDF points vertically (same visual row). **The coordinates are correct — the problem is purely stream order.**

### Extraction method comparison

| Method | Label-value adjacency |
|--------|-----------------------|
| lopdf stream order | ❌ Template dump then values dump |
| pdf-extract (pdf-rs) | ❌ Same problem |
| pdfminer.six | ❌ Same problem |
| pypdf (heuristic) | ⚠️ Partially better |
| Coordinate sort (-y, x) | ✅ Correct |

### The Fix

**Sort text runs by `(-y, x)` before joining** — descending y (top-to-bottom), ascending x (left-to-right). This naturally interleaves template labels with their filled values on the same row, producing:

```
Event Date: __________________ Sat 2.28.26  Event Time: __________________ 3-7pm
```

Requires reading `cm` (concat matrix) transform operators from the lopdf content stream to recover each XObject's position, then sorting before text concatenation. Change is isolated to `doc_parser.rs`.

---

## Active Investigations

- [x] Raw pdf-extract text dump — **CONFIRMED: stream order is root cause**
- [x] Prompt engineering — `RULES_PROMPT` updated with form-field, date disambiguation, anti-repetition rules
- [ ] Pipeline refactor — extract core logic to `pipeline.rs`, add debug logging (agent running)
- [ ] CLI harness — `cargo run --bin harness -- --pdf <path> --query <text>` (agent running)
- [ ] Coordinate-aware sort in `doc_parser.rs` — **ready to implement, approach confirmed**

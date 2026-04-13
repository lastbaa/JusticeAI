# Justice AI — Eval System Design

## 1. The Problem

The current eval harness gives 100% recall on all 33 retrieval cases, which doesn't match real-world experience. Three root causes:

1. **Recall@6 is too generous.** Small docs (1-3 chunks) are trivially returned in full. Only irs_w9_filled.pdf (55 chunks) is a real retrieval test.
2. **`ensure_form_data_included()` is a cheat code.** It force-injects filled form data chunks regardless of score, making every form-field question trivially pass.
3. **No LLM evaluation.** The eval tests retrieval only. Real failures happen when the LLM has the right chunks but gives wrong/hallucinated/incomplete answers.

## 2. New Metrics

### Headline Dashboard
```
Pipeline: hybrid-bm25-cosine | 77 cases | 8 fixtures
MRR: 0.634 | P@1: 54.5% | Recall: 92.2% | Passed: 74/77 (96%)
Failures: confusable_lease entity disambiguation, settlement numbers, NDA blocklist
```

### Metric Definitions

| Metric | What it measures | Formula |
|--------|-----------------|---------|
| **MRR** (Mean Reciprocal Rank) | How high the first relevant chunk ranks | `avg(1/rank_of_first_hit)` across cases |
| **P@1** (Precision at 1) | Does the #1 chunk contain the answer? | `cases_where_rank1_has_answer / total_cases` |
| **Recall@k** | Is the answer anywhere in top-k? | Current metric, kept for comparison |
| **Partial Score** | Fraction of expected terms found | `terms_found / terms_expected` per case, averaged |

### Why These Matter
- **MRR** is the single best number. It moves when retrieval quality changes, unlike recall@6 which is pegged at 100%.
- **P@1** is the sharpest metric. If the #1 chunk has the answer, the LLM almost always gets it right.
- **Partial Score** replaces binary pass/fail. A case hitting 2/3 expected terms scores 0.67, not 0.0.

## 3. Eval JSON Schema (v2)

```json
{
  "pdf": "tests/fixtures/irs_w9_filled.pdf",
  "query": "What is the person's name on the W-9?",
  "expected": ["Liam Neild"],
  "must_not_contain": ["John", "Jane"],
  "difficulty": "hard",
  "tags": ["entity-extraction", "form-field"],
  "top_k": 6,
  "expected_rank": 1,
  "type": "retrieval",
  "notes": "Tests semantic gap: 'name' → 'Liam Neild' with no lexical overlap"
}
```

### New Fields

| Field | Type | Purpose |
|-------|------|---------|
| `must_not_contain` | `string[]` | Negative assertions — fail if these appear in top-k |
| `difficulty` | `easy\|medium\|hard` | Filters + weighted scoring |
| `tags` | `string[]` | Categorization for `--filter-tag` |
| `expected_rank` | `int?` | Ideal rank position of the answer chunk (null = don't check) |
| `type` | `retrieval\|e2e\|negative\|adversarial\|multi-fact\|cross-reference` | Case type — controls scoring logic |
| `notes` | `string?` | Human-readable explanation of what's being tested |

### Case Types

| Type | Expected | Pass Condition | Count |
|------|----------|----------------|-------|
| `retrieval` | Substrings to find in top-k | At least 1 expected term found | 33 |
| `negative` | `[]` (empty) | Pipeline recognizes info is NOT in the doc | 6 |
| `adversarial` | `[]` (empty) | Pipeline avoids confusing similar-but-wrong info | 5 |
| `multi-fact` | Multiple terms from potentially different chunks | ALL expected terms found in top-k | 4 |
| `cross-reference` | Terms using different vocabulary than the doc | Expected term found despite synonym mismatch | 4 |

**Current eval.json: 52 cases across 5 fixtures.**

### Example: Negative Case
```json
{
  "pdf": "tests/fixtures/irs_w9_filled.pdf",
  "query": "What is the filer's date of birth?",
  "expected": [],
  "type": "negative",
  "difficulty": "easy",
  "tags": ["negative", "w9", "missing-field"],
  "notes": "W-9 does not collect date of birth. Model should not hallucinate one."
}
```

### Example: Adversarial Case
```json
{
  "pdf": "tests/fixtures/irs_w9_filled.pdf",
  "query": "What is the filer's phone number?",
  "expected": [],
  "type": "adversarial",
  "difficulty": "medium",
  "tags": ["adversarial", "near-miss", "missing-field"],
  "notes": "W-9 has an EIN (123-45-6789) which looks like a phone number. Model must not confuse EIN with phone."
}
```

### Example: Multi-Fact Case
```json
{
  "pdf": "tests/fixtures/plain_contract.pdf",
  "query": "Is the security deposit more than two months' rent?",
  "expected": ["3,700", "1,850"],
  "type": "multi-fact",
  "difficulty": "hard",
  "tags": ["multi-fact", "reasoning", "lease"],
  "notes": "Requires retrieving both deposit ($3,700) and rent ($1,850), then comparing."
}
```

### Example: Cross-Reference Case
```json
{
  "pdf": "tests/fixtures/plain_contract.pdf",
  "query": "What is the lessor's full name?",
  "expected": ["Jane Thompson"],
  "type": "cross-reference",
  "difficulty": "medium",
  "tags": ["cross-reference", "synonym", "lease"],
  "notes": "Document says 'landlord' but query uses 'lessor'. Tests legal synonym handling."
}
```

## 4. Harness Architecture

### CLI Flags
```
harness --eval <eval.json>                    # Default: retrieval-only
        --backend <name>                      # hybrid | reranker (default: hybrid)
        --compare <b1>,<b2>                   # Side-by-side comparison
        --mode retrieval|full                 # retrieval-only or with LLM
        --json-out <path>                     # Machine-readable results
        --diff <baseline.json>                # Show regressions vs prior run
        --filter-tag <tag>                    # Run subset by tag
        --filter-tier <difficulty>            # Run subset by difficulty
        --filter-pdf <substring>              # Run subset by PDF name
        --min-mrr <float>                     # Fail (exit 1) if MRR below threshold
        --no-form-injection                   # Disable ensure_form_data_included
```

### Embedding Cache
Cache chunk embeddings keyed on `blake3(chunk_text)` in `target/eval-cache/`. Cuts repeated runs from ~15s to ~2s.

### Per-PDF Deduplication
Group cases by PDF path. Parse and embed each PDF once, reuse across all queries for that PDF.

### JSON Report Format
```json
{
  "timestamp": "2026-03-10T17:30:00Z",
  "git_sha": "abc1234",
  "backend": "hybrid-bm25-cosine",
  "metrics": {
    "mrr": 0.72,
    "precision_at_1": 0.58,
    "recall_at_5": 0.91,
    "partial_score": 0.74
  },
  "by_difficulty": {
    "easy": { "mrr": 0.93, "count": 15 },
    "medium": { "mrr": 0.78, "count": 18 },
    "hard": { "mrr": 0.61, "count": 12 }
  },
  "cases": [
    {
      "query": "What is the person's name?",
      "pdf": "irs_w9_filled.pdf",
      "pass": true,
      "recall": 1.0,
      "partial_score": 1.0,
      "mrr": 1.0,
      "answer_rank": 1,
      "top_scores": [0.82, 0.45, 0.38],
      "missed_terms": [],
      "elapsed_ms": 234
    }
  ]
}
```

## 5. Test Fixtures

### Current Fixtures
| Fixture | Pages | Chunks | Problem |
|---------|-------|--------|---------|
| plain_contract.pdf | 1 | 1 | Trivial — every query returns the only chunk |
| filled_form_simple.pdf | 1 | 2 | Trivial — top_k=6 returns everything |
| bartending_contract.pdf | 1 | 3 | Trivial — top_k=6 returns everything |
| ga_statement_of_claim.pdf | 1 | 9 | Mostly blank, limited test value |
| irs_w9_filled.pdf | 6 | 55 | Only real retrieval challenge |

### New Fixtures (Priority Order)

#### P0: Confusable Entities Lease (~15KB, 4-6 chunks)
Two similar party names ("James Morrison" landlord, "James Morrison Jr." tenant), five different dates (lease, move-in, first payment, renewal, termination notice). Tests entity disambiguation and date confusion.

**Sample queries:**
- "Who is the landlord?" → must distinguish Sr. from Jr.
- "When is the termination notice deadline?" → must pick correct date from 5 options
- "What is the landlord's address?" → must not return tenant's address

#### P0: Cross-Document Pair — MSA + Amendment (~25KB, 8-12 chunks)
Master services agreement with hourly rate $150/hr. Amendment changes it to $175/hr. When both are indexed, "What is the current hourly rate?" must retrieve from the amendment.

**Sample queries:**
- "What is the current hourly rate?" → $175 (amendment), NOT $150 (original)
- "When was the original agreement signed?" → retrieves from MSA
- "What changed in the amendment?" → retrieves from amendment

#### P1: Dense NDA (~30KB, 20-25 chunks)
10-page NDA with repetitive "Confidential Information" / "Receiving Party" in every chunk. One unique buried fact: "$500,000 damages cap in Section 8.3." Tests retrieval in high-similarity embedding space.

**Sample queries:**
- "What is the damages cap?" → must find the one chunk with $500,000
- "What is excluded from Confidential Information?" → specific section
- "Does this document mention arbitration?" → negative case (no)

#### P1: Financial Settlement (~15KB, 6-8 chunks)
Settlement breakdown: $12,500 medical, $8,750 lost wages, $5,000 pain/suffering, $26,250 total, 33.3% attorney fee. Tests numeric precision and disambiguation.

**Sample queries:**
- "What is the total settlement?" → $26,250 (not a sub-component)
- "What percentage is the attorney fee?" → 33.3% (not the medical amount)
- "How much are the medical expenses?" → $12,500

#### P2: DOCX Employment Offer Letter (~10KB, 3-5 chunks)
Covers the DOCX parsing path. Simple 2-page offer with salary, start date, benefits.

### Generation Strategy
All synthetic fixtures generated by `tests/fixtures/generate_eval_fixtures.py` using `reportlab`. Deterministic output, committed to git. Total new fixtures: ~95KB.

## 6. Current Results (First Honest Run)

**77 cases | 8 fixtures | hybrid-bm25-cosine backend**

```
Cases:          77
Passed:         74/77 (96%)
Avg recall:     92.2%
MRR:            0.634
Precision@1:    54.5% (42/77)
```

### What MRR and P@1 Reveal

The old "100% recall" masked that **only 54.5% of cases have the answer in the #1 chunk**. The LLM weighs early chunks most heavily, so this is the real quality number.

### Specific Failures Found

| Case | Type | Issue |
|------|------|-------|
| "Who is the tenant?" (confusable_lease) | blocklist | Both "100 Oak" (landlord addr) and tenant info in same chunks — can't disambiguate |
| "Total settlement amount?" (settlement) | missed | $31,650 not found in top-k chunks — possible PDF generation issue |
| "Net after attorney fees?" (settlement) | missed | $21,110.55 not in chunks — computed value not in document |
| "Penalties for breach?" (NDA) | adversarial | Found $500,000 liability cap — not a "penalty" but retrieval can't distinguish |
| "Lost wages?" (settlement) | blocklist | Found $8,750 but also $5,000 and $2,200 (other amounts) in same chunks |

### Key Insight
The confusable_lease and dense_nda fixtures are doing exactly what they were designed to do — **exposing retrieval precision weaknesses that the old tiny-doc fixtures couldn't surface**.

## 7. LLM Answer Evaluation

### Tier 1: Deterministic Assertions (every run, zero cost)
**Implemented in `src/assertions.rs`.** Run on the LLM's text output without calling any model:

1. **`check_citations(answer, known_files)`** — regex validates `[filename, p. N]` format, ensures at least one citation exists, verifies filenames against known documents
2. **`check_number_exactness(answer, expected)`** — verbatim substring check for expected numbers/dates
3. **`check_blocklist(answer, blocked)`** — case-insensitive detection of forbidden terms (hallucination markers)
4. **`check_no_hallucination(answer, chunks)`** — extracts sentences containing numbers or proper nouns, verifies each key token appears in at least one source chunk

Each returns `Vec<AssertionResult>` with pass/fail, type, and human-readable message.

### Tier 2: Live LLM Eval (manual, ~5-10 min)
Run with `--mode full`. Calls `ask_llm` for each case, applies Tier 1 assertions on the actual generated answer. Slow but catches prompt regressions and LLM reasoning failures.

### Tier 3: LLM-as-Judge (stretch goal, ~$0.30/run)
For the 10 hardest open-ended cases, call an LLM judge with a rubric:
- Does the answer address every part of the question? (0/1)
- Are all citations traceable to provided chunks? (0/1)
- Any facts not in the chunks (hallucination)? (0/1)
- Numbers exact, not rounded? (0/1)

## 8. Cross-Document Evaluation (Future)

Index multiple PDFs simultaneously and test whether retrieval returns chunks from the correct document.

```json
{
  "multi_doc": ["tests/fixtures/plain_contract.pdf", "tests/fixtures/bartending_contract.pdf"],
  "query": "How much is the monthly rent?",
  "expected": ["1,850"],
  "expected_source": "plain_contract.pdf",
  "must_not_contain_from": "bartending_contract.pdf"
}
```

Requires: `SourcedChunk` wrapper tracking origin PDF, merged `RetrievalCorpus`, source provenance assertions.

## 9. Implementation Phases

### Phase 1: Honest Metrics — DONE
- [x] Extend eval JSON schema (difficulty, tags, must_not_contain, type)
- [x] Add 6 negative/unanswerable cases
- [x] Add 5 adversarial near-miss cases
- [x] Add 4 multi-fact reasoning cases
- [x] Add 4 cross-reference synonym cases
- [x] Add MRR, P@1, partial score to harness output
- [x] Handle negative/adversarial cases in scoring (pass = nothing found)
- [x] Add `--report` flag for JSON output
- [x] Add `--diff` flag for regression detection
- [x] Add `--compare` flag for backend comparison
- [x] Generate 3 new fixtures: confusable_lease, dense_nda, settlement_breakdown
- [x] Add LLM assertion module (`src/assertions.rs`)
- [x] **Result: 77 cases, MRR 0.634, P@1 54.5% — real failures surfaced**

### Phase 2: Polish (1-2 days)
- [ ] Embedding cache (hash-keyed cached vectors, cut eval time from ~45s to ~5s)
- [ ] Per-PDF parsing deduplication in eval loop
- [ ] Fix settlement_breakdown.pdf generation (total $31,650 not appearing)
- [ ] Cross-document eval support (`multi_doc` field)
- [ ] `--no-form-injection` flag for honest retrieval-only testing
- [ ] Wire assertions.rs into `--mode full` eval

### Phase 3: Stretch Goals
- [ ] `--mode full` end-to-end eval with LLM
- [ ] `--filter-tag` / `--filter-tier` for targeted runs
- [ ] DOCX test fixture
- [ ] CI integration via GitHub Actions (retrieval-only, cached embeddings)
- [ ] LLM-as-judge via external API
- [ ] Latency tracking (embed_ms, search_ms, generate_ms)

## 10. What NOT to Build

| Idea | Why Skip |
|------|----------|
| NDCG@k | Overkill for <100 cases. MRR + P@1 is sufficient. |
| Difficulty-weighted composite scores | Complex to interpret. Filter by difficulty tag instead. |
| `--record` interactive mode | Just edit the JSON. You have <80 cases. |
| Golden-answer snapshots with similarity scoring | Partial credit on expected terms gives 90% of the value. |
| Temperature/seed pinning | llama.cpp at temp=0 is already nearly deterministic. |
| Confidence margin tracking | Interesting but not actionable at this stage. |

# Justice AI — Usability Audit Report

**Audit Date:** 2026-03-10
**App Version:** 1.0.0
**Audited By:** 5-agent synthesis (UX, Visual Design, Accessibility, Information Architecture, Onboarding)

---

## 1. Executive Summary

**Overall Grade: C+**

Justice AI is a technically ambitious app with a clear value proposition — fully local, private legal research with no API keys. The core pipeline works. The problem is that the surrounding UX hasn't kept pace with the backend quality. The app feels like a prototype that was shipped once the Rust backend stabilized, with the frontend left in a half-finished state. The most serious issues aren't polish problems — they're functional ones: no disk space check before a 4.5 GB download can brick a user's session; citations are cleared before new ones arrive, creating a flash of empty state; and there are zero ARIA landmarks, meaning the app is effectively unusable with a screen reader. Legal professionals (the target audience) include a higher-than-average proportion of users with accessibility needs, and law firms increasingly require WCAG AA compliance for internal tools. The visual design has good bones — the navy/gold palette works — but the execution is inconsistent enough to undermine the premium positioning. Given the app's differentiator (local and private), trust signals need to be everywhere, and right now they're scattered. The path from C+ to B+ is achievable in two focused sprints. The path to A requires a more significant accessibility pass.

---

## 2. Scoring Breakdown

| Dimension | Score | Rationale |
|---|---|---|
| **UX / Interaction** | 5/10 | Core flows work but have sharp edges: citation flash, no send-readiness signal, opaque progress states. The pipeline is solid; the feedback layer around it is not. |
| **Visual Design** | 6/10 | Palette is coherent and appropriately professional. Execution is fragmented: 10+ hardcoded font sizes, inconsistent border radii, gold used so frequently it loses meaning as a CTA signal. |
| **Accessibility** | 3/10 | No ARIA landmarks, no focus traps on modals, keyboard navigation broken by `outline: none`, icon buttons have no accessible names. This is the weakest dimension by a significant margin. |
| **Information Architecture** | 6/10 | Four-panel layout is a reasonable choice for the use case but breaks down on smaller screens. The two right-side panels (ContextPanel + DocumentViewer) compete with each other and the chat area. |
| **Onboarding** | 5/10 | ModelSetup screen handles the hard problem of a 4.5 GB download, but lacks the safety rails that make it trustworthy: no disk space check, no cancel, no ETA, error messages that don't help users recover. |

---

## 3. Consolidated Findings

Findings from all five agents have been deduplicated and grouped by theme. Redundant or near-identical issues are merged with all source tags noted.

---

### Theme A: Feedback Gaps — The App Doesn't Tell Users What's Happening

These are all variants of the same underlying problem: the frontend doesn't surface backend state clearly enough.

**A1. No "ready to chat" signal after document load**
The send button is disabled until documents are indexed, but there is no tooltip, status badge, or confirmation that explains why — or that tells the user when indexing is complete. First-time users can't distinguish "still processing" from "failed silently."
*(Source: #1)*

**A2. Citation flash on follow-up queries**
`setLastCitations([])` fires immediately when a new query starts, wiping the previous citations before the new ones arrive. If the new query returns zero citations, the panel is permanently empty with no fallback. This is a state management bug as much as a UX issue.
*(Source: #2)*

**A3. Progress indicators don't reflect actual pipeline state**
The typing indicator cycles through pre-written phrases on a 2.8s timer regardless of actual progress. For long queries (embedding + vector search + LLM inference), users see the same rotation for 30+ seconds with no differentiation. The Rust backend likely emits distinct phases; surface them.
*(Source: #3, #A6, #O11)*

**A4. Download progress lacks actionable information**
The model download screen shows percentage and GB transferred but no ETA, no speed, and no visible resume state after a retry. The Rust backend has resume logic but the frontend shows "0%" after retry, making users think progress was lost.
*(Source: #9, #O4, #O5)*

**A5. First-query latency warning comes too late**
The "first query may take a minute" notice appears only after 5 seconds have elapsed. By then, users have already concluded the app froze. This message needs to be shown *before* the user submits their first query.
*(Source: #O6)*

**A6. Document parsing progress is a black box**
"Processing documents..." is a spinner with no per-file or per-stage granularity. For a batch of large PDFs, users have no idea if anything is happening.
*(Source: #O11)*

---

### Theme B: Error Handling — Failures Don't Help Users Recover

**B1. Download errors are not differentiated**
Network timeout, disk full, 403 Forbidden, and server errors all surface as "Download failed. Check your connection." These require different recovery actions. A disk-full error needs a "free up space" prompt. A 403 needs "check network/VPN." Lumping them together is a dead end.
*(Source: #O3)*

**B2. No disk space validation before download**
The app initiates a 4.5 GB download without checking available disk space. A partial download that hits ENOSPC produces a generic network error with no recovery path. This is a data-loss-adjacent bug.
*(Source: #O1)*

**B3. No cancel button on download**
If the download stalls, the only exit is force-quitting. Users who change their mind or detect a bad network condition are stuck.
*(Source: #O2)*

**B4. File upload errors are one-size-fits-all**
"No supported files found" fires for format mismatch, parse failure, and permission errors alike. Each case requires a different user action.
*(Source: #6)*

**B5. Settings changes requiring reindex give no warning**
Changing `chunkSize` or `chunkOverlap` silently requires a full reindex to take effect. There's no indication of this in the UI, so users change the settings and see no difference in results.
*(Source: #IA5)*

---

### Theme C: Accessibility — Structural Violations

These are not polish issues. Several are WCAG Level A violations.

**C1. No ARIA landmarks**
The app has no `<main>`, no `<aside aria-label>` on the sidebar, and no `role="complementary"` on the ContextPanel or DocumentViewer. Screen readers cannot navigate the layout. This is a Level A violation.
*(Source: #A1)*

**C2. Icon-only buttons have no accessible names**
Add Documents, New Chat, Settings, Edit/Delete session, close DocumentViewer, and copy excerpt are all icon buttons with no `aria-label`. They are invisible to screen readers and voice navigation.
*(Source: #A2)*

**C3. Focus ring removed, no replacement**
`outline: none` is applied to the textarea and inputs with only a color change on focus. Keyboard users cannot see where focus is. This is a WCAG 2.4.7 Level AA violation.
*(Source: #A3)*

**C4. No focus trap on modals**
Tab focus escapes Settings and ModelSetup modals into background content. Combined with no `role="dialog"` or `aria-modal="true"`, these modals are non-functional for keyboard users.
*(Source: #A4, #A7)*

**C5. Text contrast failures**
`rgba(255,255,255,0.2–0.35)` on dark backgrounds produces contrast ratios of 1.6–2.8:1 against the AA minimum of 4.5:1. Timestamps, section headers, placeholders, and labels are all affected. This is a WCAG 1.4.3 Level AA violation and compounds the legal readability problem.
*(Source: #A5, #V1)*

**C6. No `aria-live` regions for dynamic content**
Query processing phases and download progress emit no announcements to assistive technology. Screen reader users get silence while the app processes.
*(Source: #A6)*

**C7. Spinner elements have no accessible label**
Spinners in ContextPanel and DocumentViewer have no `role="status"` or `aria-label`. Screen readers cannot announce loading state.
*(Source: #A10)*

**C8. Error alerts not announced**
Error `div` elements are missing `role="alert"` or `aria-live="assertive"`. Errors that appear visually are not surfaced to assistive technology.
*(Source: #A14)*

**C9. Color-only relevance indicators**
The Strong/Good/Weak relevance badges use only color (green/gold/gray) to convey meaning. Text labels exist but colorblind users who rely on shape differentiation cannot distinguish them from the dot alone.
*(Source: #A13)*

---

### Theme D: Visual Consistency — The Design System Is Implicit and Fragmented

**D1. Font size scale is ad hoc**
10+ hardcoded font sizes from `text-[10px]` to `text-[28px]` with no hierarchy logic. Changing the type scale requires hunting through components. A 6-stop Tailwind config would fix this.
*(Source: #V4)*

**D2. Border radius has no consistent language**
Buttons use `rounded-xl`, `rounded-lg`, `rounded-md` inconsistently. Inputs are `rounded-2xl`. The result looks assembled from different design sessions.
*(Source: #V5)*

**D3. Gold is overused as an accent**
40+ components use gold for icons, borders, spinners, progress bars, and loading states. When everything is emphasized, nothing is. Gold should be reserved for primary CTAs and critical status indicators.
*(Source: #V3)*

**D4. Ghost buttons have no affordance at rest**
"Load folder" and "Browse files" buttons are invisible as interactive elements until hover. Users scanning the UI can't identify clickable targets.
*(Source: #V2)*

**D5. Background shades are indistinguishable**
`#080808`, `#0d0d0d`, `#0c0c0c` are visually identical at a glance. Panel boundaries blur. Users lose orientation between the sidebar, chat area, and panels.
*(Source: #V6)*

**D6. Trust/privacy signals are inconsistent**
"On-device" appears in some places but not others. The embedding model has no "Local" badge. For an app whose primary differentiator is privacy, this is a missed opportunity to continuously reinforce the value proposition.
*(Source: #V8)*

---

### Theme E: Layout and Space Management

**E1. No sidebar collapse**
240px of sidebar is always visible. On 13" screens this is a significant tax, especially with DocumentViewer open. There's no way to reclaim this space.
*(Source: #IA2)*

**E2. Right-side panels compete for space**
ContextPanel (300px) and DocumentViewer (520px) both occupy the right side. With both visible on a 1440px display, the chat area is reduced to ~380px. There's no tab or toggle to choose one over the other. This needs either a tab switcher or a rule that ContextPanel collapses when DocumentViewer opens.
*(Source: #IA1, #IA6)*

**E3. Small interactive targets**
Edit and delete session buttons are 16×16px. WCAG AAA recommends 44×44px. Even WCAG AA (2.5.5, Level AA) recommends 24px. At 16px these are difficult to hit for users with motor impairments or touchpad users.
*(Source: #A11)*

---

### Theme F: Discoverability and Labeling

**F1. ContextPanel labels are opaque**
"Retrieved Context" and "Given Documents" mean nothing to legal professionals. These should be renamed to something like "Sources Used" and "Uploaded Documents."
*(Source: #8)*

**F2. RAG settings labels are too technical**
"Chunk size" and "chunk overlap" are ML implementation details. Legal users shouldn't need to know what chunking is. Rename or wrap these in preset profiles.
*(Source: #5)*

**F3. Model names shown without explanation**
"Qwen3-8B" and "BGE-small-en-v1.5" appear in Settings with no user-facing context. These should be surfaced as "Legal AI Model (local)" and "Search Model (local)" with the technical name as a secondary label.
*(Source: #O9)*

**F4. Export is invisible**
Export (Markdown conversation, CSV citations) is hidden behind small icon buttons with no keyboard shortcut. For legal professionals who need to document their research, this feature is essentially undiscoverable.
*(Source: #10)*

**F5. Session rename is hover-only**
The rename control is a hover-only 16px icon. There's no right-click menu, no double-click-to-edit, and no keyboard shortcut. Users who don't hover over the session name never discover it.
*(Source: #7)*

---

## 4. Prioritized Roadmap

---

### Tier 1: Quick Wins — Under 1 Day Each

These are single-file or few-line changes. A developer who knows the codebase can ship all of these in one focused day.

| # | Fix | Effort | Sources |
|---|---|---|---|
| **T1.1** | Add `aria-label` to all icon-only buttons. Audit every `<button>` that contains only an icon — Add Documents, New Chat, Settings, Edit Session, Delete Session, Close Viewer, Copy Excerpt. Add `aria-label` and `title`. | 2h | #A2 |
| **T1.2** | Fix citation flash: don't clear `lastCitations` until new citations arrive. Change logic so citations are replaced atomically — set new citations in the success handler, not the start handler. | 1h | #2 |
| **T1.3** | Add `role="alert"` to all error `div` elements and `role="status"` + `aria-label` to all spinner elements. | 1h | #A10, #A14 |
| **T1.4** | Add `aria-live="polite"` region for query phase updates and download progress. One `<div aria-live="polite" aria-atomic="true">` that receives text updates. | 1h | #A6 |
| **T1.5** | Add ARIA landmarks: wrap Sidebar in `<aside aria-label="Sessions">`, main content area in `<main>`, ContextPanel in `<aside aria-label="Document context">`. | 30m | #A1 |
| **T1.6** | Replace `outline: none` with a visible focus ring on inputs and textarea. Use `focus:ring-2 focus:ring-[#c9a84c]` or equivalent. Do not remove the outline without a replacement. | 1h | #A3 |
| **T1.7** | Add `role="dialog"` `aria-modal="true"` `aria-labelledby` to Settings and ModelSetup modals. | 30m | #A7 |
| **T1.8** | Fix ghost button affordance: add a visible border (e.g., `border border-white/20`) at rest so they read as interactive without hover. | 30m | #V2 |
| **T1.9** | Show "first query may take a minute" inline hint when the user focuses the chat input for the first time, not after 5 seconds of delay. | 30m | #O6 |
| **T1.10** | Rename ContextPanel labels: "Retrieved Context" → "Sources Used", "Given Documents" → "Uploaded Documents." | 15m | #8 |
| **T1.11** | Add `role="list"` + `role="listitem"` (or convert to `<ul><li>`) for session list, citation list, and source list. | 1h | #A8 |
| **T1.12** | Add `prefers-reduced-motion` media query to CSS animations (fadeUp, pulseGlow, scan). One global rule in the stylesheet. | 30m | #A12 |

---

### Tier 2: Sprint Work — 1–3 Days Each

These require more thought or touch more files, but each is self-contained.

| # | Fix | Effort | Sources |
|---|---|---|---|
| **T2.1** | **Disk space check + meaningful download errors.** Before starting download in the Rust `download_models` command, call `statvfs` (macOS/Linux) or `GetDiskFreeSpaceEx` (Windows) and emit an error if available space < 6 GB. Separately, map error types (network timeout, disk full, HTTP 403, HTTP 5xx) to distinct error codes and surface appropriate recovery messages in `ModelSetup.tsx`. | 1 day | #O1, #O3, #B1 |
| **T2.2** | **Add cancel button to download.** Add a `cancel_download` Tauri command that signals the download task to abort (use a `CancellationToken` or an `AtomicBool`). Wire up a "Cancel" button in `ModelSetup.tsx` that's enabled as soon as download starts. | 1 day | #O2 |
| **T2.3** | **Download ETA + speed display.** The `download-progress` event already emits bytes. Track a rolling 3-second window of bytes received, compute MB/s, and estimate remaining time. Display "~8 min remaining at 9 MB/s" under the progress bar. Make resume state explicit: show "Resuming from X GB..." when retrying. | 0.5 day | #O4, #O5 |
| **T2.4** | **Focus trap for modals.** Implement a reusable `<FocusTrap>` wrapper component (or use `focus-trap-react`) that constrains Tab/Shift+Tab to modal content and returns focus to the trigger element on close. Apply to Settings and ModelSetup. | 0.5 day | #A4 |
| **T2.5** | **Differentiated file upload errors.** In `doc_parser.rs`, distinguish `UnsupportedFormat`, `ParseError`, `PermissionDenied`, and `EmptyFile` error variants. Map each to a user-facing message with a recovery suggestion in the renderer. | 1 day | #6, #B4 |
| **T2.6** | **Settings reindex warning.** When `chunkSize` or `chunkOverlap` changes, display a persistent banner: "Chunk settings changed — re-upload your documents to apply." This is a label change + a comparison in the settings save handler. | 0.5 day | #IA5, #B5 |
| **T2.7** | **Contrast fixes for failing text.** Audit all `rgba(255,255,255,0.2–0.35)` usages. Bump timestamps, section headers, and placeholder text to at least `rgba(255,255,255,0.6)` (which gets to ~4.2:1 on #0d0d0d — AA compliant for large text, borderline for small). For small text labels, go to `rgba(255,255,255,0.75)`. | 1 day | #A5, #V1 |
| **T2.8** | **Pipeline phase feedback.** Emit distinct Tauri events for pipeline phases: `embedding_start`, `search_start`, `inference_start`, `inference_done`. In `ChatInterface.tsx`, map these to meaningful status strings ("Searching documents...", "Generating answer...") instead of a rotating timer. | 1 day | #3, #A6 |
| **T2.9** | **"Ready to chat" status indicator.** After documents finish indexing, change the chat input placeholder from disabled to "Documents indexed — ask a question." Add a small status dot (gold = ready, gray = no documents). Add a tooltip on the disabled button state explaining why it's disabled. | 0.5 day | #1 |
| **T2.10** | **Increase small interactive target sizes.** Edit/delete session buttons: change from `h-4 w-4` to `h-6 w-6` with `p-1` padding, giving an effective hit target of ~32px. Not perfect but a meaningful improvement without layout change. | 0.5 day | #A11 |

---

### Tier 3: Planned Work — 1 Week+

These are architectural or design-system changes. Plan and spec before building.

| # | Fix | Effort | Sources |
|---|---|---|---|
| **T3.1** | **Sidebar collapse.** Add a toggle button to collapse the Sidebar to 0px (or a 48px icon rail). Store state in `localStorage`. This involves changes to the flex layout in `App.tsx` and the Sidebar component. Critical for 13" screens. | 3–5 days | #IA2, #E1 |
| **T3.2** | **Right-panel tab/toggle.** When DocumentViewer opens, either (a) collapse ContextPanel automatically or (b) render a tab strip that lets users switch between "Sources" and "Document". Option (a) is simpler. This requires rethinking how both panels are rendered in `App.tsx`. | 3–5 days | #IA6, #E2 |
| **T3.3** | **Establish a minimal design token system.** Define a Tailwind config with 6 named font sizes (`text-caption`, `text-body-sm`, `text-body`, `text-label`, `text-heading-sm`, `text-heading`), 3 named border radii (`rounded-sm` for inputs, `rounded-md` for buttons, `rounded-lg` for cards), and a reduced gold usage palette (`gold-accent` for CTAs only, `gold-muted` for secondary elements). Then do a one-time sweep to replace hardcoded values. This is a multi-file refactor but each file change is mechanical. | 1 week | #V4, #V5, #V3, #D1, #D2 |
| **T3.4** | **Per-file document parsing progress.** Emit `parse_progress` events from `doc_parser.rs` with `{file: string, page: number, total_pages: number}`. Display a progress list in the upload state UI. Requires changes to the doc parser, the Tauri command, and the renderer upload flow. | 3–5 days | #O11, #A3 |

---

### Tier 4: Backlog / Nice to Have

Low ROI or not worth building at this stage. Park unless user research validates demand.

| # | Item | Why Deferred |
|---|---|---|
| **T4.1** | RAG settings presets (Balanced/Precise/Thorough) | Low ROI until the underlying settings (chunkSize, etc.) are properly labeled. Fix labels first. |
| **T4.2** | Citation map ("Contract.pdf — 3 refs: pp. 5, 12, 18") | Useful but not urgent. Citation panel already shows sources. Add after basic accessibility is fixed. |
| **T4.3** | Session search empty state guidance | Edge case. The current blank state isn't harmful. |
| **T4.4** | Background panel color differentiation (#080808 vs #0d0d0d) | Low user impact. Panels are already separated by content. Don't invest time in this when contrast failures in text are more urgent. |
| **T4.5** | "Index age" / "last indexed" timestamp | Informational only. Users know when they uploaded files. Not worth building until file management UX is more complete. |
| **T4.6** | Orphaned session warnings | Low frequency edge case. Investigate whether this actually happens in practice before building. |
| **T4.7** | Conversation context indicator ("last 3 Q&A pairs") | This is internal pipeline detail. Surfacing it adds complexity without user benefit. |
| **T4.8** | Keyboard shortcut for export | Export UX itself is broken (hidden icons). Fix discoverability first; shortcuts can come later. |

---

## 5. What NOT to Build

These are items the agents flagged that are explicitly not worth building, over-engineered for the app's scope, or whose ROI doesn't justify the cost.

**Icon style unification (#V7):** Standardizing filled vs. outlined icons across 40+ components for visual consistency is a lot of churn for a subtle improvement. Only worth doing during a full component library pass (T3.3), not standalone.

**Semantic trust badges on embedding model (#V8, partial):** Adding a "Local" badge specifically to the embedding model in Settings is too granular. The broader fix (T1.10 label rename + general "on-device" messaging) handles the trust signal problem at a higher level.

**Session auto-rename visibility improvements (#7, partial):** The hover-only rename icon is bad UX, but session renaming is a power-user feature. Before investing in a contextual menu or double-click rename interaction, validate that users actually rename sessions. The current implementation is discoverable enough for motivated users. Address only if user feedback confirms it's a pain point.

**Document count deduplication across panels (#IA3):** The document count appearing in two places is mildly confusing but not harmful. A unified "index status summary" component is more architecture than the problem warrants.

**WCAG AAA targets for body text contrast:** The spec calls for 7:1 contrast for AAA. The current AA minimum (4.5:1) is the correct target for a v1 app. Don't over-engineer for AAA on extended reading surfaces until there's a specific use case or legal requirement.

**Full `<ul><li>` conversion for all lists:** The ARIA `role="list"` + `role="listitem"` approach (T1.11) gives screen readers the same information with minimal change. A full HTML element conversion across all lists is unnecessary.

---

## 6. Overall Recommendation

**Fix accessibility first, in this order: T1.1 through T1.12, then T2.4, T2.7, T2.8.**

The accessibility deficiencies are not polish issues — they are structural failures that make the app unusable for a meaningful segment of the legal profession. Law firms increasingly audit internal tools for WCAG AA compliance, and several of the current violations (no ARIA landmarks, no accessible button names, broken focus management, contrast failures) are Level A and AA. These are also the highest-density fixes in the roadmap: twelve of them can be shipped in a single day by a single developer, most in under an hour each. The citation flash bug (T1.2) should be bundled into the same PR because it's a one-line fix with high user-visible impact. After that first day, the onboarding safety rails (T2.1 — disk space check and error differentiation) should be next because they're the only issues that can cause genuine data-loss-adjacent failures. The design system work (T3.3) is important for long-term maintainability but should not block shipping the accessibility and reliability fixes.

---

*Report generated from findings of 5 specialist audit agents. Engineering estimates assume a developer with working knowledge of the codebase.*

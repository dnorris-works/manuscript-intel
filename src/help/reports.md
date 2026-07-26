# Loremetry Desktop — Report Reference

This guide explains every report you can run from the Analyzer. Reports are grouped by platform tab. Some reports appear on more than one tab (for example Chapter Summaries on both KDP and Wide). Running a report once stores it for that story; the same result is visible wherever that report type is listed.

---

## How platforms work

| Tab | Purpose |
|---|---|
| **KDP** | Amazon Kindle Direct Publishing — categories, keywords, competition |
| **Wide** | Non-Amazon stores — discovery keywords, shared genre/BISAC work |
| **Craft** | Structural and prose craft audits |
| **Publish** | Reader experience, polish, and formatter prep |

Dependencies: some reports need others first (shown in the Analyzer). Selecting a report auto-selects its dependencies when required.

---

## Shared (KDP + Wide)

### Chapter Summaries

Reads each manuscript chapter and extracts **genre signals** — setting, tone, themes, conflict type, romance/faith/supernatural markers, pacing, POV, and tropes — not a plot synopsis.

Use this as the foundation for genre analysis, ranking, categories, keywords, and BISAC. Re-run with “Force re-summarize” if chapters changed substantially.

### Genre Analysis

Uses chapter summaries to produce industry ebook/print genre labels, suggested KDP category paths, comparable titles, reader demographic, bookstore shelving notes, and marketing notes.

### Genre Ranking

Scores the manuscript independently against every genre in the app’s catalog (scores need not sum to 100). Helps spot cross-genre fit and confidence levels.

### BISAC Classification

Picks 1–3 BISAC subject codes from the seeded fiction catalog for print / Ingram-style metadata. Prefers specific headings over vague “General” codes when confidence is close.

---

## KDP only

### KDP Categories

Matches your book to real Amazon category paths (from your catalog / Winning Cat data) and ranks them for discoverability when stats are available.

### KDP Keywords

Produces exactly seven KDP keyword strings (≤50 characters each), with rationale and an overall strategy note. Can prefer measured search data when a keyword pool exists.

### Search Terms

Short Amazon-style phrases (about 2–4 words) for competition research — what a reader might type to find books like yours.

### Full Analysis

Combined packaging report: categories, BISAC, keywords, and positioning in one document after the supporting reports have run.

### Keyword Search Results

Live or stored Amazon keyword volume / competition data (DataForSEO or Canopy), used to ground keyword strategy in measured demand.

### Competition Analysis

AI market landscape from competitor book data: niche competitiveness, books to study, pricing, and a viability verdict for a debut.

### Reader Review Intelligence

Mines competitor reviews for what readers love and hate, their language, gap opportunities, and positioning advice.

### Competitor Author Analysis

Looks at competitor catalogs: release cadence, pricing patterns, series vs standalone, review performance, and strategic takeaways.

---

## Wide only

### Discovery Keywords

Ten discovery phrases aimed at Apple Books, Kobo, Google Play, Barnes & Noble, BookBub, Goodreads, and web SEO — not Amazon KDP keyword fields. Rationales are AI-reasoned (not measured search volume).

---

## Craft

### Zeigarnik Effect

**No AI.** Heuristic scan for open loops: cliffhanger endings, unresolved questions, and threads that reappear after a gap. Uses tunable phrase lists in the database. Good for spotting where tension is held open for the reader.

### Continuity Check

AI extraction of continuity-relevant facts per chapter, then judgment of contradictions (manuscript or **series** scope). Suggest-fix can propose rewrites for a flagged contradiction.

### Show Don't Tell

AI flags passages that state emotion or judgment instead of dramatizing it, with severity and context. Suggest-fix offers show-instead rewrites.

### AI-isms

AI flags prose that often reads as machine-generated (stock vocabulary, template antithesis, vague abstraction, etc.). Suggest-fix offers more human rewrites.

### Chekhov's Gun

Finds significant early setups (objects, skills, promises, characters) and checks whether they pay off later. Flags unresolved or unearned payoffs.

### Red Herring vs Abandoned

Separates intentional misdirection from plot threads that look dropped by accident.

### Foreshadowing & Twist Fairness

Checks whether foreshadowing is distributed fairly and whether twists feel earned (surprised, not cheated) or overly telegraphed.

### MacGuffin Clarity

Evaluates whether the driving object or goal is clearly established and consistently motivates character action.

### Want vs Need

For major characters: clear external want vs internal need, and whether that tension drives growth.

### Thematic Throughline

Traces the central theme across scenes, subplots, and arcs — inconsistency, absence, or heavy-handedness.

### Mirror/Foil Characters

Identifies reflect/contrast pairings and whether they illuminate theme and deepen characterization.

### POV Discipline

Flags unintentional POV shifts, head-hopping, and information leaks that break the chosen perspective.

### Story Beat Placement

Maps structural beats against common frameworks (as a reference, not a rigid rule). Highlights beats that are early, late, or missing.

### Scene/Sequel Balance

Action vs reflective (sequel) passages — stretches that exhaust the reader or stall momentum.

### Timeline / Flashback

Whether timeline shifts and flashbacks clarify or confuse, and whether transitions serve a clear purpose.

### Dramatic Irony

Moments where the reader knows more than the characters — and whether that gap creates the intended tension, humor, or dread.

### Stakes Escalation

How stakes rise across the arc; plateaus or reversals that may cause disengagement.

### Cross-Book Setup/Payoff

**Series scope.** Setups planted in earlier books that should pay off later; series-spanning loose ends.

### Series Pacing Comparator

**Series scope.** Compares pacing across installments relative to overall series rhythm.

### Recurring Motif/Theme (Series)

**Series scope.** Motifs and themes across books — cohesion and intentional evolution vs abandoned or contradictory threads.

---

## Publish

### AI Beta Reader

Chapter-by-chapter “reader” reactions: engagement score, put-down risk, highlights, and friction. Uses a stronger model slot by default.

### Cliffhanger Score

Scores each chapter ending for pull into the next (cliffhanger / hook / soft landing / resolved). Related to Zeigarnik, but focused on ending pull scores rather than open-loop inventory.

### Hook Strength

Evaluates whether a browsing reader would keep going past page one: intrigue, voice, stakes, clarity, momentum.

### Pacing Curve

Per-chapter pace and drag-risk scores so you can see where the manuscript slows before reviewers tell you.

### Line-level Polish

**No AI.** Heuristic scan for filter words, nearby word echoes, -ly adverbs, and rough passive constructions. Unlimited free runs.

### Vellum & Atticus Prep

**No AI.** Builds a cleaned Markdown manuscript under `Publishing/manuscript-clean.md` (chapter breaks marked) for import into Vellum or Atticus. Convert to `.docx` if your formatter prefers that.

---

## Tips

1. Start publishing work with **Chapter Summaries**, then genre/ranking, then platform-specific reports.
2. Craft and Publish do not require summaries, but a story bible (and Characters / Locations folders) improves AI context when present.
3. Series craft audits need a series with books in reading order and the series scope selected in the Analyzer.
4. Cost estimates in the Analyzer use model prices from Settings; Free means no LLM call (Zeigarnik, Line Polish, Vellum prep).

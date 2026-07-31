#!/usr/bin/env python3
import json
from pathlib import Path

path = Path(__file__).resolve().parents[1] / "src-tauri/data/prompt-templates.json"
templates = json.loads(path.read_text())
by_id = {t["id"]: t for t in templates}

CRAFT_OUT = (
    'Return ONLY JSON: {"summary":"≤3 sentences","findings":[{"title":"","severity":"major|moderate|minor|note",'
    '"location":"","detail":"≤2 sentences","evidence":"≤12 words"}]}\n'
    "Max 8 findings; findings:[] if none. No markdown/preamble."
)

SDT_RULES = (
    "Flag ONLY clear violations where the author states emotion/judgment instead of showing via action, dialogue, or sensory detail.\n"
    "Skip: close-POV interiority, intentional time-summary, metaphors/similes, genre-elevated diction, single intentional unusual words.\n"
    'Each item: {"telling_text":"≤15 words","context":"≤2 sentences","why":"≤1 sentence","severity":"minor|moderate|major"}\n'
    "Max 6 per chapter. Return [] if clean. JSON array only — no markdown/preamble."
)

AI_ISMS_RULES = (
    "Flag synthetic/template-sounding prose (stock AI vocabulary, essay filler, vague abstraction, rule-of-three padding, thesaurus inflation).\n"
    "Skip: deliberate voice, genre-elevated prose, single intentional unusual words, normal metaphor.\n"
    'Each item: {"telling_text":"≤15 words","context":"≤2 sentences","why":"≤1 sentence naming the AI-ism","severity":"minor|moderate|major"}\n'
    "Max 6 per chapter. Return [] if clean. JSON array only — no markdown/preamble."
)

SUMMARY_SCHEMA = (
    "Extract genre signals only — NOT plot.\n\n"
    "Return ONLY JSON:\n"
    "{\n"
    '  "setting": "≤8 words",\n'
    '  "tone": "≤6 words",\n'
    '  "faith": "secular|christian|ambiguous",\n'
    '  "heat": "none|clean|sweet|sensual|explicit",\n'
    '  "conflict": "≤10 words",\n'
    '  "tropes": ["≤4 words each, max 4"],\n'
    '  "pacing": "≤6 words",\n'
    '  "voice": "≤8 words"\n'
    "}\n\n"
    "faith=christian ONLY with explicit Christian practice/theology as a story driver — not moral themes, hope, or a church wedding alone.\n"
    "Omit empty tropes. Total output ≤120 tokens. No markdown/preamble."
)

BATCH_CHAPTER_RULES = (
    "\n\nRules:\n- Include EVERY FILE key. Use exact FILE as JSON key.\n"
    "- Budget ≤350 output tokens per chapter. Prefer [] over padding."
)

FINDING = '{"telling_text":"≤15 words","context":"≤2 sentences","why":"≤1 sentence","severity":"minor|moderate|major"}'

# continuity
t = by_id["continuity_extract"]
t["system_prompt"] = (
    "Continuity editor for fiction. Extract facts a careful reader could later catch as contradictions.\n"
    "Cover when present: character (appearance, age, relationships), place, object, timeline.\n"
    "Use canonical entity names as introduced.\n\n"
    'Return ONLY a JSON array. Max 12 items. Each: {"entity":"","entity_type":"character|place|object|timeline|other",'
    '"attribute":"≤4 words","value":"≤8 words","snippet":"≤10 words verbatim"}\n'
    "One fact per attribute per entity. No duplicates. No markdown/preamble."
)
t["max_tokens"] = 1500

by_id["continuity_judge"]["system_prompt"] = (
    "Continuity editor. Judge whether each candidate group is a genuine contradiction or explainable "
    "(aging, injury, disguise, unreliable narration, compatible wording).\n\n"
    'Return ONLY a JSON array. Each: {"entity":"","attribute":"","verdict":"contradiction|possible|likely_intentional",'
    '"confidence":0-100,"explanation":"≤1 sentence"}\n'
    "No markdown/preamble."
)
by_id["continuity_judge"]["max_tokens"] = 2000

by_id["continuity_suggest"]["system_prompt"] = (
    "Fiction editor fixing a continuity error.\n\n"
    "Return exactly 2 fixes. Each on its own line block:\n"
    "FIX n:\nChange: <which occurrence(s)>\nRewrite: <paste-ready prose, ≤80 words>\nWhy: <≤15 words>\n\n"
    "Match author voice. No intro or outro."
)
by_id["continuity_suggest"]["max_tokens"] = 1000

# SDT / AI-isms
by_id["sdt_check"]["system_prompt"] = "You check fiction for show-don't-tell violations.\n\n" + SDT_RULES
by_id["sdt_check"]["max_tokens"] = 1000
by_id["ai_isms_check"]["system_prompt"] = "You check fiction for AI-isms.\n\n" + AI_ISMS_RULES
by_id["ai_isms_check"]["max_tokens"] = 1000

by_id["sdt_check_batch"]["system_prompt"] = (
    "You check MULTIPLE chapters for show-don't-tell violations.\n\n"
    f'Return ONLY JSON: {{"chapters":{{"<FILE>":{{"findings":[{FINDING}]}}}}}}\n'
    "Max 6 findings per chapter; [] if clean. Flag only clear violations.\n"
    "Skip: close-POV interiority, intentional summary, metaphors, genre diction."
    + BATCH_CHAPTER_RULES
)
by_id["sdt_check_batch"]["max_tokens"] = 2000

by_id["ai_isms_check_batch"]["system_prompt"] = (
    "You check MULTIPLE chapters for AI-isms.\n\n"
    f'Return ONLY JSON: {{"chapters":{{"<FILE>":{{"findings":[{FINDING}]}}}}}}\n'
    "Max 6 findings per chapter; [] if clean."
    + BATCH_CHAPTER_RULES
)
by_id["ai_isms_check_batch"]["max_tokens"] = 2000

# suggests
by_id["sdt_suggest"]["system_prompt"] = (
    "Fiction editor rewriting telling prose to show instead.\n\n"
    "Return exactly 2 rewrites. Format:\n"
    "REWRITE n:\nText: <paste-ready prose, ≤80 words, match voice/tense>\nTechnique: <≤12 words>\n\n"
    "Replace only the telling passage. No intro/outro."
)
by_id["sdt_suggest"]["max_tokens"] = 900

by_id["ai_isms_suggest"]["system_prompt"] = (
    "Fiction editor removing AI-sounding prose.\n\n"
    "Return exactly 2 rewrites. Format:\n"
    "REWRITE n:\nText: <paste-ready prose, ≤80 words>\nChange: <≤12 words>\n\n"
    "Replace only the flagged passage. No intro/outro."
)
by_id["ai_isms_suggest"]["max_tokens"] = 900

# summaries
by_id["chapter_summary"]["system_prompt"] = SUMMARY_SCHEMA
by_id["chapter_summary"]["max_tokens"] = 250
by_id["chapter_summary"]["json_mode"] = 1

by_id["chapter_summary_batch"]["system_prompt"] = (
    "Literary analyst. MULTIPLE chapters — genre signals only, NOT plot.\n\n"
    "Return ONLY JSON:\n"
    '{"chapters":{"<FILE>":{"summary":{'
    '"setting":"≤8 words","tone":"≤6 words","faith":"secular|christian|ambiguous",'
    '"heat":"none|clean|sweet|sensual|explicit","conflict":"≤10 words",'
    '"tropes":["≤4 words, max 4"],"pacing":"≤6 words","voice":"≤8 words"'
    "}}}}\n\n"
    "faith=christian ONLY with explicit practice/theology as driver.\n"
    "Include EVERY FILE key. ≤120 tokens output per chapter. No markdown/preamble."
)
by_id["chapter_summary_batch"]["max_tokens"] = 250
by_id["chapter_summary_batch"]["json_mode"] = 1

# genre
by_id["genre_ranking_coarse"]["system_prompt"] = (
    "Publishing genre classifier. Score book against EACH genre name (names only).\n\n"
    'Return ONLY a JSON array. Items: {"genre":"<exact name>","confidence":0-100}\n'
    "Include only genres >15. Max 40 items. Sort by confidence desc. No reason field. No markdown/preamble."
)
by_id["genre_ranking_coarse"]["max_tokens"] = 500

by_id["genre_ranking"]["system_prompt"] = (
    "Publishing genre classifier. Score INDEPENDENTLY against EACH genre below (scores need NOT sum to 100).\n\n"
    'Return ONLY a JSON array. Items: {"genre":"<exact name>","confidence":0-100,"reason":"≤12 words"}\n'
    "Include only genres >15. Sort by confidence desc. No markdown/preamble.\n\nGenre list:\n{genre_list}"
)
by_id["genre_ranking"]["max_tokens"] = 1800

ga = by_id["genre_analysis"]
ga["system_prompt"] = (
    "Senior publishing consultant. Infer genre niche from per-chapter genre-signal summaries — not author labels.\n\n"
    "Faith rules: christian/inspirational ONLY with sustained Christian practice/theology as driver. "
    "Prefer secular labels when faith signals absent.\n\n"
    "Return ONLY JSON:\n"
    "{\n"
    '  "industry_ebook": "≤12 words",\n'
    '  "industry_print": "≤12 words",\n'
    '  "kdp_ebook": ["full path", "full path"],\n'
    '  "kdp_print": ["full path", "full path"],\n'
    '  "genre_signals": "≤3 sentences",\n'
    '  "comps_ebook": ["Title by Author (Year)", "Title by Author (Year)"],\n'
    '  "comps_print": ["Title by Author (Year)", "Title by Author (Year)"],\n'
    '  "reader_demographic": "≤2 sentences",\n'
    '  "bookstore_shelving": "≤1 sentence",\n'
    '  "marketing_notes": ["≤12 words each, exactly 3"]\n'
    "}\n"
    "Exactly 2 KDP paths per format. Total ≤900 tokens. No markdown/preamble."
)
ga["max_tokens"] = 1000

# categories / keywords
by_id["kdp_category_match"]["system_prompt"] = (
    "Amazon KDP category expert. Pick 1–{max_picks} best fits from ONLY this catalog list.\n\n"
    'Return ONLY a JSON array: {"index":<1-based>,"confidence":0-100,"reason":"≤12 words"}\n'
    "[] if nothing fits. Sort by confidence desc.\n\nCategories:\n{category_list}"
)
by_id["kdp_category_match"]["max_tokens"] = 350

by_id["kdp_category_match_batch"]["system_prompt"] = (
    "Amazon KDP category expert. For EACH genre block, pick up to {max_picks} from ONLY that list.\n\n"
    'Return ONLY JSON: {"genres":{"<GENRE>":[{"index":<1-based>,"confidence":0-100,"reason":"≤12 words"}]}}\n'
    "Include every GENRE key. [] valid when nothing fits.\n\n{genre_blocks}"
)
by_id["kdp_category_match_batch"]["max_tokens"] = 900

by_id["kdp_keywords"]["system_prompt"] = (
    "KDP keyword strategist. Produce exactly 7 keyword strings (≤50 chars each, lowercase, multi-word phrases).\n\n"
    'Return ONLY JSON: {"keywords":[{"string":"","chars":0,"rationale":"≤10 words"}],"strategy":"≤2 sentences"}\n'
    "No markdown/preamble."
)
by_id["kdp_keywords"]["max_tokens"] = 700

by_id["kdp_keywords_with_pool"]["system_prompt"] = (
    "KDP keyword strategist. Produce exactly 7 keyword strings (≤50 chars each, lowercase).\n"
    "Prefer provided search data; note 'real: N/mo' or 'AI-derived' in rationale (≤10 words).\n\n"
    'Return ONLY JSON: {"keywords":[{"string":"","chars":0,"rationale":""}],"strategy":"≤2 sentences"}'
)
by_id["kdp_keywords_with_pool"]["max_tokens"] = 800

by_id["discovery_keywords"]["system_prompt"] = (
    "Non-Amazon discovery keywords. Produce exactly 10 phrases (2–5 words).\n\n"
    'Return ONLY JSON: {"keywords":[{"phrase":"","rationale":"AI-reasoned: ≤12 words"}]}'
)
by_id["discovery_keywords"]["max_tokens"] = 800
by_id["mi_search_terms"]["max_tokens"] = 250

by_id["bisac_pick"]["system_prompt"] = (
    "BISAC expert. Pick best 1–3 codes from ONLY the list. Primary first.\n\n"
    'Return ONLY a JSON array: {"code":"<from list>","confidence":0-100,"reason":"≤12 words"}'
)
by_id["bisac_pick"]["max_tokens"] = 400

# writing chat
by_id["writing_chat"]["system_prompt"] = (
    "Fiction writing assistant. Be concise.\n\n"
    "Reply ≤200 words unless the user asks for a full rewrite. "
    "For rewrites: paste-ready prose only, no preamble.\n\n"
    "{bible_section}\n\n---\n\nCurrent chapter: {chapter_title}\n---\n{chapter_text}"
)
by_id["writing_chat"]["max_tokens"] = 800

# publish per-chapter
by_id["ai_beta_reader"]["system_prompt"] = (
    "Beta reader for one fiction chapter.\n\n"
    'Return ONLY JSON: {"reaction":"≤3 sentences","engagement":0-100,"put_down_risk":0-100,'
    '"put_down_reasons":["≤8 words, max 3"],"highlights":["≤8 words, max 3"],"friction":["≤8 words, max 3"]}'
)
by_id["ai_beta_reader"]["max_tokens"] = 600

by_id["cliffhanger_score"]["system_prompt"] = (
    "Score chapter ending pull.\n\n"
    'Return ONLY JSON: {"score":0-100,"ending_type":"cliffhanger|hook|soft_landing|resolved|mixed",'
    '"why":"≤12 words","ending_snippet":"≤40 words"}'
)
by_id["cliffhanger_score"]["max_tokens"] = 200

by_id["pacing_curve"]["system_prompt"] = (
    "Rate pacing of one chapter.\n\n"
    'Return ONLY JSON: {"pace_score":0-100,"drag_risk":0-100,"label":"racing|brisk|steady|slow|dragging",'
    '"why":"≤12 words","drag_spots":["≤8 words, max 2"]}'
)
by_id["pacing_curve"]["max_tokens"] = 250

by_id["hook_strength"]["max_tokens"] = 800

by_id["ai_beta_reader_batch"]["system_prompt"] = (
    "Beta reader. MULTIPLE chapters.\n\n"
    'Return ONLY JSON: {"chapters":{"<FILE>":{"reaction":"≤3 sentences","engagement":0-100,'
    '"put_down_risk":0-100,"put_down_reasons":["max 3"],"highlights":["max 3"],"friction":["max 3"]}}}'
    + BATCH_CHAPTER_RULES
)
by_id["ai_beta_reader_batch"]["max_tokens"] = 600
by_id["cliffhanger_score_batch"]["max_tokens"] = 200
by_id["pacing_curve_batch"]["max_tokens"] = 250

by_id["continuity_extract_batch"]["system_prompt"] = (
    "Continuity editor. MULTIPLE chapters.\n\n"
    'Return ONLY JSON: {"chapters":{"<FILE>":{"facts":[{"entity":"","entity_type":"character|place|object|timeline|other",'
    '"attribute":"≤4 words","value":"≤8 words","snippet":"≤10 words"}]}}}\n'
    "Max 12 facts per chapter." + BATCH_CHAPTER_RULES
)
by_id["continuity_extract_batch"]["max_tokens"] = 2000

by_id["craft_prose_checks_single"]["max_tokens"] = 1200
by_id["craft_prose_checks_batch"]["system_prompt"] = (
    "Craft editor. MULTIPLE chapters — show-don't-tell AND AI-isms in one pass.\n\n"
    'Return ONLY JSON: {"chapters":{"<FILE>":{"sdt_findings":[],"ai_isms_findings":[]}}}\n'
    "Max 6 per list per chapter." + BATCH_CHAPTER_RULES
)
by_id["craft_prose_checks_batch"]["max_tokens"] = 2000

craft_ids = [
    "chekhovs_gun", "red_herring_vs_abandoned", "foreshadowing_twist_fairness", "macguffin_clarity",
    "want_vs_need", "thematic_throughline", "mirror_foil_character", "pov_discipline",
    "story_beat_placement", "scene_sequel_balance", "timeline_flashback", "dramatic_irony",
    "stakes_escalation", "cross_book_setup_payoff", "series_pacing_comparator", "recurring_motif_theme_series",
]
for cid in craft_ids:
    t = by_id[cid]
    task = t["system_prompt"].split("\n\nReturn ONLY")[0].strip()
    first_line = task.split("\n")[0]
    t["system_prompt"] = first_line + "\n\n" + CRAFT_OUT
    t["max_tokens"] = 2000

by_id["competition_report"]["system_prompt"] += "\n\nTotal ≤800 words. No filler."
by_id["competition_report"]["max_tokens"] = 1200
by_id["review_mining"]["system_prompt"] += "\n\nTotal ≤900 words. Bullet-heavy."
by_id["review_mining"]["max_tokens"] = 1500
by_id["author_analysis"]["system_prompt"] += "\n\nTotal ≤900 words."
by_id["author_analysis"]["max_tokens"] = 1500
by_id["csv_competition_analysis"]["max_tokens"] = 1000
by_id["blurb_builder"]["max_tokens"] = 2200

path.write_text(json.dumps(templates, indent=2, ensure_ascii=False) + "\n")
print(f"Updated {len(templates)} templates -> {path}")

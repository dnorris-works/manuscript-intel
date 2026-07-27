<script setup lang="ts">
import { inject, ref, computed, watch, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { ContinuityScope } from '../composables/useAnalysis';
import { useSettings } from '../composables/useSettings';
import { storiesKey, analysisKey, seriesKey, platformKey } from '../injectionKeys';
import LogStream from './LogStream.vue';
import { useReportTypes } from '../composables/useReportTypes';

// ── Injections ────────────────────────────────────────────────────────────────

const storiesCtx = inject(storiesKey)!;
const analysisCtx = inject(analysisKey)!;
const seriesCtx = inject(seriesKey)!;
const platformCtx = inject(platformKey)!;
const settings = useSettings();

// ── Report types from DB ──────────────────────────────────────────────────────

const { reportTypes, loadReportTypes, getDependants } = useReportTypes();
onMounted(() => {
  loadReportTypes();
  fetchCostEstimates();
});

// ── Local state ───────────────────────────────────────────────────────────────

const selected = ref<string[]>([]);
const forceResummarize = ref(false);
const hasRun = ref(false);
const continuityScopeMode = ref<'manuscript' | 'series'>('manuscript');
const continuitySeriesId = ref<number | null>(null);

// ── Computed ──────────────────────────────────────────────────────────────────

const existsMap = computed(() => {
  const state = analysisCtx.analysisState.value;
  if (!state) return {} as Record<string, boolean>;
  const docs = new Set(state.existing_docs || []);
  const map: Record<string, boolean> = {
    genre_analysis: state.has_genre_data,
    genre_ranking: state.has_genre_ranking,
    kdp_categories: state.has_categories,
    kdp_keywords: state.has_keywords,
    bisac_classification: state.has_bisac,
    mi_search_terms: state.has_search_terms,
    discovery_keywords: state.has_discovery_keywords,
    analysis: state.has_full_report,
    keyword_search: state.has_keyword_search_results,
    competition_report: state.has_competition,
    review_mining: docs.has('review_mining'),
    author_analysis: docs.has('author_analysis'),
    activity_log: docs.has('activity_log'),
    zeigarnik_analysis: state.has_zeigarnik,
    continuity_check: state.has_continuity_check,
    show_dont_tell: state.has_show_dont_tell,
    ai_isms: state.has_ai_isms,
  };
  // Generic: any report with a saved document counts as exists
  for (const id of docs) {
    if (!(id in map)) map[id] = true;
  }
  return map;
});

const visibleReports = computed(() => {
  const plat = platformCtx.platform.value;
  return reportTypes.value
    .filter(r => r.platforms.includes(plat) && r.id !== 'chapter_summaries')
    .map(r => ({
      ...r,
      exists: existsMap.value[r.id] ?? false,
    }));
});

const summaryStatus = computed(() => {
  const s = analysisCtx.analysisState.value;
  if (!s || !storiesCtx.activeFolder.value) {
    return { needsRefresh: false, text: 'Select a story to manage chapter summaries.' };
  }
  if (s.summary_chapter_count === 0) {
    return { needsRefresh: false, text: 'No manuscript chapters found yet.' };
  }
  if (s.summary_missing_count > 0 || s.summary_stale_count > 0) {
    const parts: string[] = [];
    if (s.summary_missing_count > 0) parts.push(`${s.summary_missing_count} new/un-summarized`);
    if (s.summary_stale_count > 0) parts.push(`${s.summary_stale_count} changed since summary`);
    return {
      needsRefresh: true,
      text: `Chapter summaries need refresh: ${parts.join(', ')}.`
    };
  }
  return {
    needsRefresh: false,
    text: `Chapter summaries are up to date (${s.summary_count}/${s.summary_chapter_count}).`
  };
});

const summaryIssueFiles = computed(() => {
  const s = analysisCtx.analysisState.value;
  if (!s) {
    return { missing: [] as string[], stale: [] as string[] };
  }
  return {
    missing: s.summary_missing_files || [],
    stale: s.summary_stale_files || [],
  };
});

const getReportsDisabled = computed(() => {
  return analysisCtx.isWorking.value
    || !storiesCtx.activeFolder.value
    || selected.value.length === 0;
});

// ── Checkbox logic ────────────────────────────────────────────────────────────

function toggleReport(id: string): void {
  const sel = new Set(selected.value);
  const dependants = getDependants(id);

  if (sel.has(id)) {
    // Unchecking: remove this and its dependants
    sel.delete(id);
    for (const dep of dependants) {
      sel.delete(dep);
    }
  } else {
    // Checking: add this and its dependants
    sel.add(id);
    for (const dep of dependants) {
      sel.add(dep);
    }
  }

  selected.value = [...sel];
}

// Reset selection when platform changes
watch(() => platformCtx.platform.value, () => {
  selected.value = [];
});

// ── Cost estimation ───────────────────────────────────────────────────────────

const costEstimates = ref<Record<string, number>>({});

const STATIC_SEEDED_MODEL_PRICES: Record<string, { input_price: number; output_price: number }> = {
  'claude-opus-4-20250514': { input_price: 15, output_price: 75 },
  'claude-sonnet-4-20250514': { input_price: 3, output_price: 15 },
  'claude-haiku-4-5-20251001': { input_price: 1, output_price: 5 },
  'claude-3-5-sonnet-20241022': { input_price: 3, output_price: 15 },
  'claude-3-5-haiku-20241022': { input_price: 0.8, output_price: 4 },
};

const RECOMMENDED_MODEL_BY_REPORT: Record<string, string> = {
  chapter_summaries: 'claude-haiku-4.5',
  genre_analysis: 'claude-sonnet-4',
  genre_ranking: 'claude-sonnet-4',
  kdp_categories: 'claude-haiku-4.5',
  kdp_keywords: 'claude-haiku-4.5',
  bisac_classification: 'claude-haiku-4.5',
  mi_search_terms: 'claude-haiku-4.5',
  discovery_keywords: 'claude-haiku-4.5',
  analysis: 'claude-sonnet-4',
  keyword_search: 'claude-haiku-4.5',
  competition_report: 'claude-sonnet-4',
  review_mining: 'claude-sonnet-4',
  author_analysis: 'claude-sonnet-4',
  continuity_check: 'claude-sonnet-4',
  show_dont_tell: 'claude-sonnet-4',
  ai_isms: 'claude-sonnet-4',
  chekhovs_gun: 'claude-sonnet-4',
  red_herring_vs_abandoned: 'claude-sonnet-4',
  foreshadowing_twist_fairness: 'claude-sonnet-4',
  macguffin_clarity: 'claude-sonnet-4',
  want_vs_need: 'claude-sonnet-4',
  thematic_throughline: 'claude-sonnet-4',
  mirror_foil_character: 'claude-sonnet-4',
  pov_discipline: 'claude-sonnet-4',
  story_beat_placement: 'claude-sonnet-4',
  scene_sequel_balance: 'claude-sonnet-4',
  timeline_flashback: 'claude-sonnet-4',
  dramatic_irony: 'claude-sonnet-4',
  stakes_escalation: 'claude-sonnet-4',
  cross_book_setup_payoff: 'claude-sonnet-4',
  series_pacing_comparator: 'claude-sonnet-4',
  recurring_motif_theme_series: 'claude-sonnet-4',
  ai_beta_reader: 'claude-sonnet-4',
  cliffhanger_score: 'claude-haiku-4.5',
  hook_strength: 'claude-haiku-4.5',
  pacing_curve: 'claude-haiku-4.5',
  blurb_builder: 'claude-sonnet-4',
};

function normalizeModelId(id: string): string {
  return id.toLowerCase().replace(/\s+/g, '-');
}

function fallbackModelPrice(modelId: string): { input_price: number; output_price: number } | null {
  const normalized = normalizeModelId(modelId);
  if (STATIC_SEEDED_MODEL_PRICES[normalized]) return STATIC_SEEDED_MODEL_PRICES[normalized];
  return null;
}

function estimateReportModel(reportId: string): string {
  return RECOMMENDED_MODEL_BY_REPORT[reportId] || 'claude-sonnet-4';
}

function hasSummaryDependency(reportId: string, visited = new Set<string>()): boolean {
  if (reportId === 'chapter_summaries') return true;
  if (visited.has(reportId)) return false;
  visited.add(reportId);

  const report = reportTypes.value.find(r => r.id === reportId);
  if (!report) return false;
  return report.depends_on.some(dep => hasSummaryDependency(dep, visited));
}

function selectionNeedsSummaries(): boolean {
  return selected.value.some(id => hasSummaryDependency(id));
}

const totalEstimatedCost = computed(() => {
  let total = 0;
  for (const id of selected.value) {
    total += costEstimates.value[id] || 0;
  }
  return total;
});

function formatCost(cost: number): string {
  if (cost === 0) return 'Free';
  if (cost < 0.01) return '<$0.01';
  return `~$${cost.toFixed(2)}`;
}

function reportCardDescription(report: { id: string; description: string }): string {
  const estimate = costEstimates.value[report.id];
  const costText = estimate == null
    ? ' Estimated run cost: N/A.'
    : ` Estimated run cost: ${formatCost(estimate)}/run.`;
  return `${report.description}${costText}`;
}

function summaryModelPrice(): { modelId: string; inputPrice: number | null; outputPrice: number | null } {
  const modelId = settings.modelFor('summaries') || settings.modelFor('default') || estimateReportModel('chapter_summaries');
  const modelInfo = settings.models.value.find(m => m.id === modelId);
  const fallback = fallbackModelPrice(modelId) || fallbackModelPrice(estimateReportModel('chapter_summaries'));
  return {
    modelId,
    inputPrice: modelInfo?.input_price ?? fallback?.input_price ?? null,
    outputPrice: modelInfo?.output_price ?? fallback?.output_price ?? null,
  };
}

async function maybeRefreshSummariesBeforeRun(folder: string): Promise<boolean> {
  if (!selectionNeedsSummaries() || !summaryStatus.value.needsRefresh) {
    return true;
  }

  const price = summaryModelPrice();
  try {
    const estimate = await invoke<{
      success: boolean;
      files: string[];
      chapter_count: number;
      input_tokens: number;
      output_tokens: number;
      estimated_cost: number | null;
      error: string;
    }>('estimate_summary_refresh_cost', {
      request: {
        folder,
        input_price: price.inputPrice,
        output_price: price.outputPrice,
      },
    });

    if (!estimate.success) {
      const proceedNoEstimate = confirm(`Chapter summaries need refresh before running reports.\n\nCould not estimate cost: ${estimate.error || 'unknown error'}\n\nRefresh summaries now?`);
      if (!proceedNoEstimate) return false;
    } else {
      const costText = estimate.estimated_cost == null
        ? 'Estimated cost: N/A (model pricing unavailable).'
        : `Estimated cost: ${formatCost(estimate.estimated_cost)} total.`;
      const msg = [
        'Chapter summaries must be refreshed before these reports can run.',
        '',
        `Model: ${price.modelId}`,
        `Chapters to summarize: ${estimate.chapter_count}`,
        `Estimated input tokens: ${estimate.input_tokens}`,
        `Estimated output tokens: ${estimate.output_tokens}`,
        costText,
        '',
        'Refresh summaries now?'
      ].join('\n');
      const proceed = confirm(msg);
      if (!proceed) return false;
    }
  } catch (e) {
    const proceedAfterError = confirm(`Chapter summaries need refresh before running reports.\n\nFailed to estimate refresh cost: ${String(e)}\n\nRefresh summaries now anyway?`);
    if (!proceedAfterError) return false;
  }

  await analysisCtx.runSummaries(folder);

  const s = analysisCtx.analysisState.value;
  if (s && (s.summary_missing_count > 0 || s.summary_stale_count > 0)) {
    alert('Summaries are still not up to date after refresh. Please resolve chapter read errors and try again.');
    return false;
  }
  return true;
}

async function fetchCostEstimates(): Promise<void> {
  const folder = storiesCtx.activeFolder.value;
  if (!folder || visibleReports.value.length === 0) {
    costEstimates.value = {};
    return;
  }

  // Build model prices for each visible report
  const modelPrices = visibleReports.value.map(r => {
    const fnKey = reportToModelFn(r.id);
    const modelId = settings.modelFor(fnKey);
    const modelInfo = settings.models.value.find(m => m.id === modelId);
    const fallback = fallbackModelPrice(modelId) || fallbackModelPrice(estimateReportModel(r.id));
    return {
      report_id: r.id,
      input_price: modelInfo?.input_price ?? fallback?.input_price ?? 0,
      output_price: modelInfo?.output_price ?? fallback?.output_price ?? 0,
    };
  });

  try {
    const result = await invoke<{ success: boolean; estimates: { report_id: string; estimated_cost: number }[] }>('estimate_report_costs', {
      request: { folder, model_prices: modelPrices },
    });
    if (result.success) {
      const obj: Record<string, number> = {};
      for (const est of result.estimates) {
        obj[est.report_id] = est.estimated_cost;
      }
      costEstimates.value = obj;
    }
  } catch (e) {
    console.error('estimate_report_costs:', e);
  }
}

/** Map report_id to the modelFor() function key (from report_types.model_slot). */
function reportToModelFn(reportId: string): 'default' | 'summaries' | 'genre' | 'keywords' | 'continuity' | 'showDontTell' | 'aiIsms' | 'prose' {
  const slot = reportTypes.value.find(r => r.id === reportId)?.model_slot;
  switch (slot) {
    case 'summaries':
    case 'genre':
    case 'keywords':
    case 'continuity':
    case 'showDontTell':
    case 'aiIsms':
    case 'prose':
      return slot;
    default:
      return 'default';
  }
}

// Refresh estimates when folder changes, models are loaded, or report types load
watch(() => storiesCtx.activeFolder.value, () => fetchCostEstimates());
watch(() => settings.models.value, () => fetchCostEstimates());
watch(() => reportTypes.value, () => fetchCostEstimates());

// ── Handlers ──────────────────────────────────────────────────────────────────

async function onGetReports(): Promise<void> {
  const folder = storiesCtx.activeFolder.value;
  if (!folder) return;

  const ready = await maybeRefreshSummariesBeforeRun(folder);
  if (!ready) {
    return;
  }

  hasRun.value = true;
  const plat = platformCtx.platform.value;
  if (plat === 'craft' || plat === 'publish') {
    const needsSeries = selected.value.some(id =>
      id === 'continuity_check'
      || id === 'cross_book_setup_payoff'
      || id === 'series_pacing_comparator'
      || id === 'recurring_motif_theme_series'
    );
    const scope: ContinuityScope = needsSeries && continuityScopeMode.value === 'series' && continuitySeriesId.value != null
      ? { mode: 'series', seriesId: continuitySeriesId.value }
      : { mode: 'manuscript' };
    await analysisCtx.runCraftAnalysis(folder, selected.value, scope);
  } else {
    await analysisCtx.runAnalyze(folder, forceResummarize.value, plat);
  }
}

function onRefreshSummaries(): void {
  const folder = storiesCtx.activeFolder.value;
  analysisCtx.runSummaries(folder);
}

function onMarketIntel(): void {
  const folder = storiesCtx.activeFolder.value;
  hasRun.value = true;
  analysisCtx.runMarketIntel(folder);
}

function onStop(): void {
  analysisCtx.cancelOperation();
}
</script>

<template>
  <div class="panel analyzer-panel">
    <p class="panel-desc">
      {{ storiesCtx.activeStory.value ? `Story: ${storiesCtx.activeStory.value.name}` : 'Select or create a story to begin.' }}
    </p>

    <!-- Platform tabs -->
    <div class="platform-tabs">
      <button
        class="platform-tab"
        :class="{ active: platformCtx.platform.value === 'kdp' }"
        @click="platformCtx.setPlatform('kdp')"
      >KDP</button>
      <button
        class="platform-tab"
        :class="{ active: platformCtx.platform.value === 'wide' }"
        @click="platformCtx.setPlatform('wide')"
      >Wide</button>
      <button
        class="platform-tab"
        :class="{ active: platformCtx.platform.value === 'craft' }"
        @click="platformCtx.setPlatform('craft')"
      >Craft</button>
      <button
        class="platform-tab"
        :class="{ active: platformCtx.platform.value === 'publish' }"
        @click="platformCtx.setPlatform('publish')"
      >Publish</button>
    </div>

    <!-- Actions (top) -->
    <div class="analyzer-actions">
      <button
        class="btn"
        :disabled="getReportsDisabled"
        @click="onGetReports"
      >Get Reports</button>

      <span v-if="selected.length > 0 && totalEstimatedCost > 0" class="cost-total">
        {{ formatCost(totalEstimatedCost) }}
      </span>

      <button
        v-if="platformCtx.isKdp.value"
        class="btn btn-secondary"
        title="Run market intelligence via Canopy API"
        :disabled="analysisCtx.isWorking.value || !analysisCtx.analysisState.value?.has_search_terms"
        @click="onMarketIntel"
      >Market Intel</button>

      <button
        v-if="analysisCtx.isWorking.value"
        class="btn btn-stop"
        @click="onStop"
      >Stop</button>

      <label v-if="platformCtx.platform.value !== 'craft' && platformCtx.platform.value !== 'publish'" class="force-resummarize-label">
        <input v-model="forceResummarize" type="checkbox" />
        Force re-summarize
      </label>
    </div>

    <div class="summary-status" :class="{ stale: summaryStatus.needsRefresh }">
      <span>{{ summaryStatus.text }}</span>
      <button
        class="btn btn-secondary btn-sm"
        :disabled="analysisCtx.isWorking.value || !storiesCtx.activeFolder.value"
        @click="onRefreshSummaries"
      >Refresh Summaries</button>
    </div>

    <div v-if="summaryStatus.needsRefresh" class="summary-issues">
      <div v-if="summaryIssueFiles.missing.length > 0" class="summary-issue-block">
        <div class="summary-issue-title">Missing summaries:</div>
        <ul class="summary-file-list">
          <li v-for="f in summaryIssueFiles.missing" :key="`missing-${f}`">{{ f }}</li>
        </ul>
      </div>
      <div v-if="summaryIssueFiles.stale.length > 0" class="summary-issue-block">
        <div class="summary-issue-title">Changed since summarized:</div>
        <ul class="summary-file-list">
          <li v-for="f in summaryIssueFiles.stale" :key="`stale-${f}`">{{ f }}</li>
        </ul>
      </div>
    </div>

    <!-- Continuity Check scope (only relevant when that report is selected) -->
    <div v-if="(platformCtx.platform.value === 'craft' || platformCtx.platform.value === 'publish') && selected.some(id => id === 'continuity_check' || id === 'cross_book_setup_payoff' || id === 'series_pacing_comparator' || id === 'recurring_motif_theme_series')" class="continuity-scope-row">
      <span class="continuity-scope-label">Continuity Check scope:</span>
      <label class="scope-radio">
        <input v-model="continuityScopeMode" type="radio" value="manuscript" />
        This manuscript
      </label>
      <label class="scope-radio">
        <input v-model="continuityScopeMode" type="radio" value="series" :disabled="seriesCtx.series.value.length === 0" />
        Series
      </label>
      <select
        v-if="continuityScopeMode === 'series'"
        v-model="continuitySeriesId"
        class="continuity-series-select"
      >
        <option :value="null" disabled>Choose a series…</option>
        <option v-for="s in seriesCtx.series.value" :key="s.id" :value="s.id">{{ s.name }} ({{ s.books.length }} books)</option>
      </select>
      <span v-if="seriesCtx.series.value.length === 0" class="continuity-scope-hint">No series yet — create one in the Series panel.</span>
    </div>

    <!-- Report cards -->
    <div class="report-cards">
      <div
        v-for="report in visibleReports"
        :key="report.id"
        class="report-card"
      >
        <div class="report-card-check">
          <input
            type="checkbox"
            :checked="selected.includes(report.id)"
            @input="toggleReport(report.id)"
          />
        </div>
        <div class="report-card-content">
          <div class="report-card-label">{{ report.label }}</div>
          <div class="report-card-desc">{{ reportCardDescription(report) }}</div>
          <div class="report-card-meta">
            <span v-if="report.exists" class="report-card-exists">✓ exists</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Activity indicator -->
    <div v-if="hasRun && analysisCtx.isWorking.value" class="activity-indicator">
      <div class="spinner"></div>
      <span class="activity-text">Working...</span>
    </div>

    <!-- Log output (only shown after first run) -->
    <LogStream v-if="hasRun" />
  </div>
</template>

<style scoped>
.analyzer-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 20px;
  overflow: hidden;
}

.panel-title {
  font-size: 16px;
  font-weight: 700;
  margin-bottom: 10px;
}

.panel-desc {
  color: var(--text-muted);
  margin-bottom: 14px;
  font-size: 13px;
  line-height: 1.5;
}

/* ── Platform tabs ─────────────────────────────────────────────────────────── */

.platform-tabs {
  display: flex;
  gap: 0;
  margin-bottom: 14px;
  border-bottom: 2px solid var(--border);
}

.platform-tab {
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: 13px;
  font-weight: 600;
  padding: 8px 16px;
  cursor: pointer;
  border-bottom: 2px solid transparent;
  margin-bottom: -2px;
  transition: color 0.15s, border-color 0.15s;
}

.platform-tab:hover {
  color: var(--text);
}

.platform-tab.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

/* ── Report cards ──────────────────────────────────────────────────────────── */

.report-cards {
  flex: 1;
  overflow-y: auto;
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  margin-bottom: 14px;
  padding-right: 4px;
  align-content: start;
}

.report-card {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 10px 12px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  transition: border-color 0.15s, opacity 0.15s;
}

.report-card:hover {
  border-color: var(--accent);
}

.report-card.dimmed {
  opacity: 0.55;
}

.report-card.dimmed:hover {
  border-color: var(--border);
}

.report-card-check {
  display: flex;
  align-items: center;
  padding-top: 2px;
  cursor: pointer;
}

.report-card-check input[type="checkbox"] {
  accent-color: var(--accent);
  width: 15px;
  height: 15px;
  cursor: pointer;
}

.report-card.dimmed .report-card-check input[type="checkbox"] {
  cursor: not-allowed;
}

.report-card-content {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.report-card-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}

.report-card-desc {
  font-size: 12px;
  color: var(--text-muted);
  line-height: 1.4;
}

.report-card-exists {
  font-size: 11px;
  color: var(--accent);
  font-weight: 500;
  margin-top: 2px;
}

.report-card-meta {
  display: flex;
  gap: 10px;
  align-items: center;
  margin-top: 2px;
}

.report-card-cost {
  font-size: 11px;
  color: var(--text-muted);
  font-weight: 500;
}

.cost-total {
  font-size: 12px;
  color: var(--text-muted);
  font-weight: 600;
  padding: 4px 10px;
  background: var(--surface2);
  border-radius: var(--radius);
}

/* ── Actions ───────────────────────────────────────────────────────────────── */

.analyzer-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  margin-bottom: 8px;
}

.btn {
  background: var(--accent);
  border: none;
  border-radius: var(--radius);
  color: #fff;
  cursor: pointer;
  font-size: 13px;
  font-weight: 600;
  padding: 9px 18px;
  transition: background 0.15s;
}

.btn:hover {
  background: var(--accent-dim);
}

.btn:disabled {
  background: var(--surface2);
  color: var(--text-muted);
  cursor: not-allowed;
}

.btn-secondary {
  background: var(--surface2);
  border: 1px solid var(--border);
  color: var(--text-muted);
}

.btn-secondary:hover:not(:disabled) {
  color: var(--text);
  border-color: var(--accent);
}

.btn-sm {
  padding: 6px 10px;
  font-size: 12px;
}

.summary-status {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin: 0 0 10px;
  padding: 8px 10px;
  background: var(--surface2);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text-muted);
  font-size: 12px;
}

.summary-status.stale {
  border-color: var(--accent);
  color: var(--text);
}

.summary-issues {
  margin: 0 0 10px;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
}

.summary-issue-block + .summary-issue-block {
  margin-top: 8px;
}

.summary-issue-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text);
  margin-bottom: 4px;
}

.summary-file-list {
  margin: 0;
  padding-left: 18px;
  max-height: 140px;
  overflow-y: auto;
  font-size: 12px;
  color: var(--text-muted);
}

.btn-stop {
  background: var(--danger);
  color: white;
  font-size: 12px;
  padding: 9px 12px;
  border-radius: var(--radius);
  border: none;
  cursor: pointer;
  white-space: nowrap;
}

.btn-stop:hover {
  background: #a04050;
}

.force-resummarize-label {
  font-size: 12px;
  color: var(--text-muted);
  display: flex;
  align-items: center;
  gap: 4px;
  white-space: nowrap;
  cursor: pointer;
  margin-left: auto;
}

.force-resummarize-label input[type="checkbox"] {
  accent-color: var(--accent);
}

/* ── Continuity scope row ────────────────────────────────────────────────── */

.continuity-scope-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  margin-bottom: 10px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  font-size: 12px;
}

.continuity-scope-label {
  color: var(--text-muted);
  font-weight: 600;
  white-space: nowrap;
}

.scope-radio {
  display: flex;
  align-items: center;
  gap: 4px;
  color: var(--text);
  cursor: pointer;
  white-space: nowrap;
}

.scope-radio input[type="radio"] {
  accent-color: var(--accent);
}

.continuity-series-select {
  background: var(--surface2);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text);
  padding: 5px 8px;
  font-size: 12px;
}

.continuity-scope-hint {
  color: var(--text-muted);
  font-size: 11px;
}

/* ── Activity indicator ────────────────────────────────────────────────────── */

.activity-indicator {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 10px;
  padding: 6px 12px;
  background: rgba(232, 97, 44, 0.06);
  border: 1px solid rgba(232, 97, 44, 0.15);
  border-radius: var(--radius);
  font-size: 12px;
  color: var(--accent);
}

.spinner {
  width: 14px;
  height: 14px;
  border: 2px solid rgba(232, 97, 44, 0.3);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.activity-text {
  font-weight: 500;
}
</style>

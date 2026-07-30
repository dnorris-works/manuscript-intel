<script setup lang="ts">
import { inject, ref, computed, watch, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import {
  NText, NRadio, NButton, NSpace, NAlert, NCheckbox,
  NCard, NSpin, NSelect, useDialog, useMessage,
} from 'naive-ui';
import AnalyzerPlatformTabs from './AnalyzerPlatformTabs.vue';
import type { ContinuityScope } from '../composables/useAnalysis';
import { useSettings } from '../composables/useSettings';
import { storiesKey, analysisKey, seriesKey, platformKey, showPanelKey } from '../injectionKeys';
import LogStream from './LogStream.vue';
import { useReportTypes } from '../composables/useReportTypes';

// ── Injections ────────────────────────────────────────────────────────────────

const storiesCtx = inject(storiesKey)!;
const analysisCtx = inject(analysisKey)!;
const seriesCtx = inject(seriesKey)!;
const platformCtx = inject(platformKey)!;
const showPanel = inject(showPanelKey);
const settings = useSettings();
const dialog = useDialog();
const message = useMessage();

const seriesOptions = computed(() =>
  seriesCtx.series.value.map(s => ({
    label: `${s.name} (${s.books.length} books)`,
    value: s.id,
  })),
);

const needsContinuityScope = computed(() =>
  (platformCtx.platform.value === 'craft' || platformCtx.platform.value === 'publish')
  && selected.value.some(id =>
    id === 'continuity_check'
    || id === 'cross_book_setup_payoff'
    || id === 'series_pacing_comparator'
    || id === 'recurring_motif_theme_series'),
);

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

const freshnessMap = computed(() => {
  const s = analysisCtx.analysisState.value;
  const map: Record<string, 'fresh' | 'stale' | 'missing'> = {};
  if (!s?.report_freshness) return map;
  for (const r of s.report_freshness) {
    map[r.doc_type] = r.status;
  }
  return map;
});

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
      freshness: freshnessMap.value[r.id]
        ?? (existsMap.value[r.id] ? 'stale' as const : 'missing' as const),
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
    if (s.summary_missing_count > 0) parts.push(`${s.summary_missing_count} new/unscanned`);
    if (s.summary_stale_count > 0) parts.push(`${s.summary_stale_count} changed since last scan`);
    return {
      needsRefresh: true,
      text: `Chapter summaries need refresh: ${parts.join(', ')}.`
    };
  }
  return {
    needsRefresh: false,
    text: `Chapter summaries are up to date (${s.summary_count}/${s.summary_chapter_count}). Manage in Settings → Story Data.`,
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

function summaryFileStatus(filename: string): 'pending' | 'active' | 'done' | 'skipped' | null {
  if (!analysisCtx.isWorking.value) return null;
  return analysisCtx.summaryFileProgress.value[filename] ?? null;
}

function summaryFileMarker(filename: string): string {
  const status = summaryFileStatus(filename);
  if (status === 'done' || status === 'skipped') return '✓';
  if (status === 'active') return '…';
  return '○';
}

const getReportsDisabled = computed(() => {
  return analysisCtx.isWorking.value
    || !storiesCtx.activeFolder.value
    || selected.value.length === 0
    || setupIssues.value.length > 0;
});

const setupIssues = computed(() => {
  const plat = platformCtx.platform.value;
  if (plat === 'craft' || plat === 'publish') {
    return settings.checkCraftAnalyzeSetup();
  }
  return settings.checkPublishAnalyzeSetup();
});

const marketIntelSetupIssues = computed(() => settings.checkMarketIntelSetup());

function openSettings(): void {
  showPanel?.('settings');
}

// ── Checkbox logic ────────────────────────────────────────────────────────────

function isReportOnPlatform(reportId: string, plat: string): boolean {
  const def = reportTypes.value.find(r => r.id === reportId);
  return def ? def.platforms.includes(plat) : false;
}

function selectedForPlatform(plat: string): string[] {
  return selected.value.filter(id => isReportOnPlatform(id, plat));
}

function toggleReport(id: string): void {
  const plat = platformCtx.platform.value;
  const sel = new Set(selected.value);
  const dependants = getDependants(id);

  if (sel.has(id)) {
    // Unchecking: remove this and its dependants
    sel.delete(id);
    for (const dep of dependants) {
      sel.delete(dep);
    }
  } else {
    // Checking: add this and its dependants (only those valid on this platform)
    sel.add(id);
    for (const dep of dependants) {
      if (isReportOnPlatform(dep, plat)) {
        sel.add(dep);
      }
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

async function maybeRefreshSummariesBeforeRun(folder: string): Promise<boolean> {
  if (!selectionNeedsSummaries() || !summaryStatus.value.needsRefresh) {
    return true;
  }

  let msg = 'Some chapters need AI summarization before these reports can run.\n\n';
  try {
    const estimate = await invoke<{
      success: boolean;
      chapter_count: number;
      input_tokens: number;
      output_tokens: number;
      error: string;
    }>('estimate_summary_refresh_cost', {
      request: { folder },
    });
    const count = estimate.success ? estimate.chapter_count : 0;
    if (count > 0) {
      msg += `Chapters to summarize: ${count}\n`;
      if (estimate.input_tokens > 0) {
        msg += `Estimated tokens: ~${estimate.input_tokens.toLocaleString()} in / ~${estimate.output_tokens.toLocaleString()} out\n`;
      }
    } else {
      msg += `${summaryStatus.value.text}\n`;
    }
  } catch {
    msg += `${summaryStatus.value.text}\n`;
  }
  msg += '\nSummarize chapters now?';

  const confirmed = await new Promise<boolean>((resolve) => {
    dialog.info({
      title: 'Refresh chapter summaries',
      content: msg,
      positiveText: 'Refresh',
      negativeText: 'Cancel',
      onPositiveClick: () => resolve(true),
      onNegativeClick: () => resolve(false),
      onClose: () => resolve(false),
    });
  });
  if (!confirmed) return false;

  await analysisCtx.runSummaries(folder);

  const s = analysisCtx.analysisState.value;
  if (s && (s.summary_missing_count > 0 || s.summary_stale_count > 0)) {
    message.error('Chapter summaries are still not up to date after refresh. Please resolve chapter read errors and try again.');
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
  const reportsToRun = selectedForPlatform(plat);
  if (reportsToRun.length === 0) {
    message.warning('No reports selected for this platform.');
    return;
  }
  if (plat === 'craft' || plat === 'publish') {
    const needsSeries = reportsToRun.some(id =>
      id === 'continuity_check'
      || id === 'cross_book_setup_payoff'
      || id === 'series_pacing_comparator'
      || id === 'recurring_motif_theme_series'
    );
    const scope: ContinuityScope = needsSeries && continuityScopeMode.value === 'series' && continuitySeriesId.value != null
      ? { mode: 'series', seriesId: continuitySeriesId.value }
      : { mode: 'manuscript' };
    await analysisCtx.runCraftAnalysis(folder, reportsToRun, scope);
  } else {
    await analysisCtx.runAnalyze(folder, forceResummarize.value, plat, reportsToRun);
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
  <div class="analyzer-root">
    <n-text depth="3" style="display: block; margin-bottom: 12px; font-size: 13px;">
      {{ storiesCtx.activeStory.value ? `Story: ${storiesCtx.activeStory.value.name}` : 'Select or create a story to begin.' }}
    </n-text>

    <AnalyzerPlatformTabs />

    <n-alert v-if="setupIssues.length > 0" type="warning" title="Setup required before running reports" style="margin-bottom: 12px;">
      <ul style="margin: 8px 0; padding-left: 1.2em;">
        <li v-for="issue in setupIssues" :key="issue.id">{{ issue.message }}</li>
      </ul>
      <n-button size="small" @click="openSettings">Open Settings</n-button>
    </n-alert>

    <n-alert
      v-if="platformCtx.isKdp.value && marketIntelSetupIssues.length > 0"
      type="info"
      title="Market Intel also needs"
      style="margin-bottom: 12px;"
    >
      <ul style="margin: 8px 0; padding-left: 1.2em;">
        <li v-for="issue in marketIntelSetupIssues" :key="`mi-${issue.id}`">{{ issue.message }}</li>
      </ul>
    </n-alert>

    <n-space align="center" style="margin-bottom: 8px;" wrap>
      <n-button type="primary" :disabled="getReportsDisabled" @click="onGetReports">
        Get Reports
      </n-button>
      <n-text v-if="selected.length > 0 && totalEstimatedCost > 0" depth="3">
        {{ formatCost(totalEstimatedCost) }}
      </n-text>
      <n-button
        v-if="platformCtx.isKdp.value"
        :disabled="analysisCtx.isWorking.value || !analysisCtx.analysisState.value?.has_search_terms || marketIntelSetupIssues.length > 0"
        title="Run market intelligence via Canopy API"
        @click="onMarketIntel"
      >
        Market Intel
      </n-button>
      <n-button v-if="analysisCtx.isWorking.value" type="error" @click="onStop">Stop</n-button>
      <n-checkbox
        v-if="platformCtx.platform.value !== 'craft' && platformCtx.platform.value !== 'publish'"
        v-model:checked="forceResummarize"
      >
        Force re-scan
      </n-checkbox>
    </n-space>

    <n-space justify="space-between" align="center" style="margin-bottom: 8px;" wrap>
      <n-text :type="summaryStatus.needsRefresh ? 'warning' : undefined" style="font-size: 13px;">
        {{ summaryStatus.text }}
      </n-text>
      <n-button
        size="small"
        :disabled="analysisCtx.isWorking.value || !storiesCtx.activeFolder.value"
        @click="onRefreshSummaries"
      >
        Refresh Summaries
      </n-button>
    </n-space>

    <div v-if="summaryStatus.needsRefresh" class="summary-issues">
      <div v-if="summaryIssueFiles.missing.length > 0">
        <n-text depth="3" style="font-size: 12px; display: block; margin-bottom: 4px;">Missing summaries:</n-text>
        <ul class="summary-file-list">
          <li
            v-for="f in summaryIssueFiles.missing"
            :key="`missing-${f}`"
            class="summary-file-item"
            :class="{
              'summary-file-done': summaryFileStatus(f) === 'done' || summaryFileStatus(f) === 'skipped',
              'summary-file-active': summaryFileStatus(f) === 'active',
            }"
          >
            <span aria-hidden="true">{{ summaryFileMarker(f) }}</span>
            {{ f }}
          </li>
        </ul>
      </div>
      <div v-if="summaryIssueFiles.stale.length > 0" style="margin-top: 8px;">
        <n-text depth="3" style="font-size: 12px; display: block; margin-bottom: 4px;">Changed since last scan:</n-text>
        <ul class="summary-file-list">
          <li
            v-for="f in summaryIssueFiles.stale"
            :key="`stale-${f}`"
            class="summary-file-item"
            :class="{
              'summary-file-done': summaryFileStatus(f) === 'done' || summaryFileStatus(f) === 'skipped',
              'summary-file-active': summaryFileStatus(f) === 'active',
            }"
          >
            <span aria-hidden="true">{{ summaryFileMarker(f) }}</span>
            {{ f }}
          </li>
        </ul>
      </div>
    </div>

    <n-card v-if="needsContinuityScope" size="small" style="margin-bottom: 12px;">
      <n-text style="display: block; margin-bottom: 8px; font-size: 13px;">Continuity Check scope:</n-text>
      <n-radio-group v-model:value="continuityScopeMode">
        <n-space>
          <n-radio value="manuscript">This manuscript</n-radio>
          <n-radio value="series" :disabled="seriesCtx.series.value.length === 0">Series</n-radio>
        </n-space>
      </n-radio-group>
      <n-select
        v-if="continuityScopeMode === 'series'"
        v-model:value="continuitySeriesId"
        :options="seriesOptions"
        placeholder="Choose a series…"
        style="margin-top: 8px; max-width: 280px;"
      />
      <n-text v-if="seriesCtx.series.value.length === 0" depth="3" style="display: block; margin-top: 6px; font-size: 12px;">
        No series yet — create one in the Series panel.
      </n-text>
    </n-card>

    <div class="report-cards">
      <n-card
        v-for="report in visibleReports"
        :key="report.id"
        size="small"
        hoverable
        class="report-card"
      >
        <n-space align="start" :size="10">
          <n-checkbox
            :checked="selected.includes(report.id)"
            @update:checked="() => toggleReport(report.id)"
          />
          <div style="min-width: 0;">
            <n-text strong style="display: block;">{{ report.label }}</n-text>
            <n-text depth="3" style="font-size: 12px; display: block; margin-top: 2px;">
              {{ reportCardDescription(report) }}
            </n-text>
            <n-text
              v-if="report.freshness === 'fresh'"
              type="success"
              style="font-size: 11px; display: block; margin-top: 4px;"
            >
              ✓ up to date
            </n-text>
            <n-text
              v-else-if="report.freshness === 'stale'"
              type="warning"
              style="font-size: 11px; display: block; margin-top: 4px;"
            >
              stale — re-run to refresh
            </n-text>
            <n-text
              v-else
              depth="3"
              style="font-size: 11px; display: block; margin-top: 4px;"
            >
              not generated
            </n-text>
          </div>
        </n-space>
      </n-card>
    </div>

    <n-space v-if="hasRun && analysisCtx.isWorking.value" align="center" style="margin: 8px 0;">
      <n-spin size="small" />
      <n-text depth="3">Working…</n-text>
    </n-space>

    <LogStream v-if="hasRun" />
  </div>
</template>

<style scoped>
.analyzer-root {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 20px;
  overflow: hidden;
}

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
  cursor: default;
}

.summary-issues {
  margin-bottom: 12px;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: color-mix(in srgb, var(--surface2) 50%, transparent);
}

.summary-file-list {
  list-style: none;
  margin: 0;
  padding: 0;
  max-height: 120px;
  overflow-y: auto;
}

.summary-file-item {
  font-family: var(--mono);
  font-size: 11px;
  color: var(--text-muted);
  padding: 2px 0;
}

.summary-file-active {
  color: var(--accent);
  font-weight: 600;
}

.summary-file-done {
  color: var(--success);
}
</style>

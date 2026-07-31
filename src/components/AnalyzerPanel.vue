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
import { useCraftReportGroups } from '../composables/useCraftReportGroups';
import { buildRunQueue, collectPrerequisites } from '../reportDependencies';
import { isAiConfigured, resolveModelPrices } from '../reportCostPricing';
import type { ReportTypeDef, Series } from '../types';

type VisibleReport = ReportTypeDef & {
  exists: boolean;
  freshness: 'fresh' | 'stale' | 'missing';
};

type DepRow = {
  id: string;
  label: string;
  freshness: 'fresh' | 'stale' | 'missing';
};

type ReportSection = {
  id: string;
  label: string;
  subtitle: string;
  reports: VisibleReport[];
  showHeader: boolean;
  disabled: boolean;
  disabledReason: string;
};

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

const { reportTypes, loadReportTypes } = useReportTypes();
const { craftReportGroups, seriesReportIds, loadCraftReportGroups } = useCraftReportGroups();
onMounted(() => {
  loadReportTypes();
  loadCraftReportGroups();
  fetchCostEstimates();
});

// ── Local state ───────────────────────────────────────────────────────────────

const selected = ref<string[]>([]);
const depRunOverrides = ref<Record<string, boolean>>({});
const forceResummarize = ref(false);
const publishEbook = ref(true);
const publishPrint = ref(true);
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
    kdp_categories: state.has_categories,
    kdp_keywords: state.has_keywords,
    bisac_classification: state.has_bisac,
    mi_search_terms: state.has_search_terms,
    discovery_keywords: state.has_discovery_keywords,
    analysis: state.has_full_report,
    wide_analysis: state.has_wide_analysis,
    keyword_search: state.has_keyword_search_results,
    google_keyword_search: state.has_google_keyword_search,
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

const visibleReports = computed((): VisibleReport[] => {
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

const activeStorySeries = computed((): Series | null => {
  const folder = storiesCtx.activeFolder.value;
  if (!folder) return null;
  return seriesCtx.series.value.find(s =>
    s.books.some(b => b.story_folder === folder),
  ) ?? null;
});

function sectionAvailability(groupId: string): { disabled: boolean; reason: string } {
  if (groupId === 'series') {
    if (!storiesCtx.activeFolder.value) {
      return { disabled: true, reason: 'Select a story first.' };
    }
    if (!activeStorySeries.value) {
      return { disabled: true, reason: 'Add this story to a series in the Series panel.' };
    }
  }
  return { disabled: false, reason: '' };
}

const reportSections = computed((): ReportSection[] => {
  const reports = visibleReports.value;
  if (platformCtx.platform.value !== 'craft') {
    return [{
      id: 'all',
      label: '',
      subtitle: '',
      reports,
      showHeader: false,
      disabled: false,
      disabledReason: '',
    }];
  }

  const byId = new Map(reports.map(r => [r.id, r]));
  return craftReportGroups.value
    .map(group => {
      const availability = sectionAvailability(group.id);
      return {
        id: group.id,
        label: group.label,
        subtitle: availability.disabled ? availability.reason : group.subtitle,
        reports: group.reportIds
          .map(id => byId.get(id))
          .filter((r): r is VisibleReport => r != null),
        showHeader: true,
        disabled: availability.disabled,
        disabledReason: availability.reason,
      };
    })
    .filter(group => group.reports.length > 0);
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
    || reportsToRun.value.length === 0
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

function getReportFreshness(reportId: string): 'fresh' | 'stale' | 'missing' {
  if (reportId === 'chapter_summaries') {
    const s = analysisCtx.analysisState.value;
    if (!s || !storiesCtx.activeFolder.value) return 'missing';
    if (s.summary_chapter_count === 0) return 'missing';
    if (s.summary_missing_count > 0) return 'missing';
    if (s.summary_stale_count > 0) return 'stale';
    return 'fresh';
  }
  return freshnessMap.value[reportId]
    ?? (existsMap.value[reportId] ? 'stale' : 'missing');
}

function defaultDepRuns(depId: string): boolean {
  return getReportFreshness(depId) !== 'fresh';
}

function isDepInRunQueue(depId: string): boolean {
  if (depId in depRunOverrides.value) {
    return depRunOverrides.value[depId];
  }
  return defaultDepRuns(depId);
}

function setDepRunOverride(depId: string, run: boolean): void {
  depRunOverrides.value = { ...depRunOverrides.value, [depId]: run };
}

function prerequisitesForReport(reportId: string): DepRow[] {
  return collectPrerequisites(reportId, reportTypes.value).map(id => ({
    id,
    label: reportTypes.value.find(r => r.id === id)?.label ?? id,
    freshness: getReportFreshness(id),
  }));
}

const reportsToRun = computed(() => {
  const plat = platformCtx.platform.value;
  const primaries = selected.value.filter(id => isReportOnPlatform(id, plat));
  return buildRunQueue(primaries, reportTypes.value, isDepInRunQueue);
});

function toggleReport(id: string, disabled = false): void {
  if (disabled) return;
  const sel = new Set(selected.value);

  if (sel.has(id)) {
    sel.delete(id);
  } else {
    sel.add(id);
  }

  selected.value = [...sel];
}

function groupSelectionState(reports: VisibleReport[]): 'all' | 'some' | 'none' {
  const ids = reports.map(r => r.id);
  const count = ids.filter(id => selected.value.includes(id)).length;
  if (count === 0) return 'none';
  if (count === ids.length) return 'all';
  return 'some';
}

function toggleGroupSelection(reports: VisibleReport[], disabled = false): void {
  if (disabled) return;
  const ids = reports.map(r => r.id);
  const sel = new Set(selected.value);
  const selectAll = groupSelectionState(reports) !== 'all';

  for (const id of ids) {
    if (selectAll) {
      sel.add(id);
    } else {
      sel.delete(id);
    }
  }

  selected.value = [...sel];
}

// Reset selection when platform changes
watch(() => platformCtx.platform.value, () => {
  selected.value = [];
  depRunOverrides.value = {};
});

watch(() => storiesCtx.activeFolder.value, () => {
  depRunOverrides.value = {};
});

watch(activeStorySeries, (series) => {
  if (series) {
    continuitySeriesId.value = series.id;
    return;
  }
  selected.value = selected.value.filter(id => !seriesReportIds.value.includes(id));
}, { immediate: true });

// ── Cost estimation ───────────────────────────────────────────────────────────

const costEstimates = ref<Record<string, number>>({});
const costEstimatesLoaded = ref(false);

function isAiReport(reportId: string): boolean {
  return reportTypes.value.find(r => r.id === reportId)?.uses_ai ?? false;
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
  return reportsToRun.value.includes('chapter_summaries')
    || reportsToRun.value.some(id => hasSummaryDependency(id));
}

const aiConfigured = computed(() =>
  isAiConfigured(settings.apiKey.value, settings.model.value),
);

const reportsMissingPricing = computed(() =>
  reportsToRun.value.filter(id =>
    isAiReport(id) && costEstimatesLoaded.value && costEstimates.value[id] == null,
  ),
);

const totalEstimatedCost = computed(() => {
  let total = 0;
  for (const id of reportsToRun.value) {
    const est = costEstimates.value[id];
    if (est != null) total += est;
  }
  return total;
});

function formatCost(cost: number): string {
  if (cost === 0) return '$0.00';
  if (cost < 0.01) return '<$0.01';
  return `~$${cost.toFixed(2)}`;
}

function formatTotalCost(): string {
  if (!aiConfigured.value) return '';
  if (!costEstimatesLoaded.value) return '…';
  const missing = reportsMissingPricing.value.length;
  if (reportsToRun.value.length === 0) return '';
  if (missing === reportsToRun.value.filter(isAiReport).length) {
    return 'pricing unavailable';
  }
  const base = formatCost(totalEstimatedCost.value);
  if (missing > 0) return `${base} (${missing} unpriced)`;
  return base;
}

function reportRunCost(reportId: string, usesAi = true): string {
  if (!usesAi) return 'Free';
  if (!aiConfigured.value) return '—';
  if (!costEstimatesLoaded.value) return '…';
  const estimate = costEstimates.value[reportId];
  if (estimate == null) return 'pricing unavailable';
  return formatCost(estimate);
}

function pricingForReport(reportId: string): ReturnType<typeof resolveModelPrices> {
  const modelId = settings.modelFor(reportToModelFn(reportId));
  return resolveModelPrices(modelId, settings.models.value);
}

function depStatusLabel(freshness: 'fresh' | 'stale' | 'missing'): string {
  if (freshness === 'fresh') return 'has run';
  if (freshness === 'stale') return 'stale — re-run recommended';
  return 'not run yet';
}

function depStatusType(freshness: 'fresh' | 'stale' | 'missing'): 'success' | 'warning' | 'default' {
  if (freshness === 'fresh') return 'success';
  if (freshness === 'stale') return 'warning';
  return 'default';
}

async function maybeRefreshSummariesBeforeRun(folder: string): Promise<boolean> {
  if (!selectionNeedsSummaries() || !summaryStatus.value.needsRefresh) {
    return true;
  }

  const summaryPricing = pricingForReport('chapter_summaries');

  let msg = 'Some chapters need AI summarization before these reports can run.\n\n';
  try {
    const estimate = await invoke<{
      success: boolean;
      chapter_count: number;
      input_tokens: number;
      output_tokens: number;
      estimated_cost: number | null;
      error: string;
    }>('estimate_summary_refresh_cost', {
      request: {
        folder,
        input_price: summaryPricing.available ? summaryPricing.input_price : undefined,
        output_price: summaryPricing.available ? summaryPricing.output_price : undefined,
      },
    });
    const count = estimate.success ? estimate.chapter_count : 0;
    if (count > 0) {
      msg += `Chapters to summarize: ${count}\n`;
      if (estimate.input_tokens > 0) {
        msg += `Estimated tokens: ~${estimate.input_tokens.toLocaleString()} in / ~${estimate.output_tokens.toLocaleString()} out\n`;
      }
      if (estimate.estimated_cost != null && summaryPricing.available) {
        msg += `Estimated cost: ${formatCost(estimate.estimated_cost)}\n`;
      } else if (summaryPricing.available === false) {
        msg += 'Estimated cost: pricing unavailable — fetch models in Settings → AI Models\n';
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
  if (!folder || visibleReports.value.length === 0 || !aiConfigured.value) {
    costEstimates.value = {};
    costEstimatesLoaded.value = false;
    return;
  }

  const costReportIds = new Set<string>();
  for (const r of visibleReports.value) {
    costReportIds.add(r.id);
    for (const dep of collectPrerequisites(r.id, reportTypes.value)) {
      costReportIds.add(dep);
    }
  }

  const modelPrices = [...costReportIds].flatMap(reportId => {
    const def = reportTypes.value.find(r => r.id === reportId);
    const usesAi = def?.uses_ai ?? true;
    if (!usesAi) return [];

    const modelId = settings.modelFor(reportToModelFn(reportId));
    const pricing = resolveModelPrices(modelId, settings.models.value);
    if (!pricing.available) return [];

    return [{
      report_id: reportId,
      input_price: pricing.input_price,
      output_price: pricing.output_price,
    }];
  });

  if (modelPrices.length === 0) {
    costEstimates.value = {};
    costEstimatesLoaded.value = true;
    return;
  }

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
      costEstimatesLoaded.value = true;
    } else {
      costEstimates.value = {};
      costEstimatesLoaded.value = false;
    }
  } catch (e) {
    console.error('estimate_report_costs:', e);
    costEstimates.value = {};
    costEstimatesLoaded.value = false;
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

// Refresh estimates when folder, models, assignments, or report types change
watch(() => storiesCtx.activeFolder.value, () => fetchCostEstimates());
watch(() => settings.models.value, () => fetchCostEstimates());
watch(() => settings.modelAssignments.value, () => fetchCostEstimates());
watch(() => settings.apiKey.value, () => fetchCostEstimates());
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
  const toRun = reportsToRun.value.filter(id => isReportOnPlatform(id, plat));
  if (toRun.length === 0) {
    message.warning('No reports selected for this platform.');
    return;
  }
  if (plat === 'craft' || plat === 'publish') {
    const hasSeriesReports = toRun.some(id => seriesReportIds.value.includes(id));
    const continuityInSeriesMode = toRun.includes('continuity_check')
      && continuityScopeMode.value === 'series'
      && continuitySeriesId.value != null;
    const seriesId = continuitySeriesId.value ?? activeStorySeries.value?.id ?? null;
    const scope: ContinuityScope = (hasSeriesReports || continuityInSeriesMode) && seriesId != null
      ? { mode: 'series', seriesId }
      : { mode: 'manuscript' };
    await analysisCtx.runCraftAnalysis(folder, toRun, scope);
  } else {
    await analysisCtx.runAnalyze(folder, forceResummarize.value, plat, toRun, {
      publishEbook: publishEbook.value,
      publishPrint: publishPrint.value,
    });
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

    <n-space
      v-if="platformCtx.isKdp.value"
      align="center"
      style="margin-bottom: 12px;"
      wrap
    >
      <n-text depth="3" style="font-size: 13px;">Publishing formats:</n-text>
      <n-checkbox v-model:checked="publishEbook">Ebook</n-checkbox>
      <n-checkbox v-model:checked="publishPrint">Print</n-checkbox>
    </n-space>

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
      <n-text v-if="reportsToRun.length > 0 && aiConfigured" depth="3">
        {{ formatTotalCost() }}
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
      <section
        v-for="section in reportSections"
        :key="section.id"
        class="report-section"
        :class="{
          'report-section--grouped': section.showHeader,
          'report-section--disabled': section.disabled,
        }"
      >
        <header
          v-if="section.showHeader"
          class="craft-group-header"
        >
          <n-checkbox
            :checked="groupSelectionState(section.reports) === 'all'"
            :indeterminate="groupSelectionState(section.reports) === 'some'"
            :disabled="section.disabled"
            @update:checked="() => toggleGroupSelection(section.reports, section.disabled)"
          />
          <div class="craft-group-titles">
            <n-text strong style="display: block;">{{ section.label }}</n-text>
            <n-text depth="3" style="font-size: 12px; display: block; margin-top: 2px;">
              {{ section.subtitle }}
            </n-text>
          </div>
        </header>

        <div class="report-section-cards">
          <n-card
            v-for="report in section.reports"
            :key="report.id"
            size="small"
            hoverable
            class="report-card"
            :class="{
              'report-card--disabled': section.disabled,
              'report-card--selected': selected.includes(report.id),
              'report-card--expanded': selected.includes(report.id)
                && prerequisitesForReport(report.id).length > 0,
            }"
          >
            <div class="report-card-main">
              <n-checkbox
                :checked="selected.includes(report.id)"
                :disabled="section.disabled"
                @update:checked="() => toggleReport(report.id, section.disabled)"
              />
              <div class="report-card-body">
                <div class="report-card-title-row">
                  <n-text strong>{{ report.label }}</n-text>
                  <n-text depth="3" class="report-card-cost">
                    {{ reportRunCost(report.id, report.uses_ai) }}
                  </n-text>
                </div>
                <n-text depth="3" class="report-card-desc">
                  {{ report.description }}
                </n-text>
                <n-text
                  v-if="report.freshness === 'fresh'"
                  type="success"
                  class="report-card-status"
                >
                  has run
                </n-text>
                <n-text
                  v-else-if="report.freshness === 'stale'"
                  type="warning"
                  class="report-card-status"
                >
                  stale — re-run to refresh
                </n-text>
              </div>
            </div>

            <div
              v-if="selected.includes(report.id) && prerequisitesForReport(report.id).length > 0"
              class="report-deps"
            >
              <n-text depth="3" class="report-deps-heading">Also runs</n-text>
              <div
                v-for="dep in prerequisitesForReport(report.id)"
                :key="`${report.id}-${dep.id}`"
                class="report-dep-row"
              >
                <n-checkbox
                  :checked="isDepInRunQueue(dep.id)"
                  @update:checked="(v: boolean) => setDepRunOverride(dep.id, v)"
                />
                <div class="report-dep-body">
                  <div class="report-dep-title-row">
                    <n-text style="font-size: 12px;">{{ dep.label }}</n-text>
                    <n-text depth="3" class="report-dep-cost">
                      {{ isDepInRunQueue(dep.id) ? reportRunCost(dep.id) : '—' }}
                    </n-text>
                  </div>
                  <n-text
                    :type="depStatusType(dep.freshness)"
                    class="report-dep-status"
                  >
                    {{ depStatusLabel(dep.freshness) }}
                  </n-text>
                </div>
              </div>
            </div>
          </n-card>
        </div>
      </section>
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
  display: flex;
  flex-direction: column;
  gap: 16px;
  margin-bottom: 14px;
  padding-right: 4px;
}

.report-section--disabled {
  opacity: 0.55;
}

.craft-group-header {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  margin-bottom: 8px;
}

.craft-group-titles {
  flex: 1;
  min-width: 0;
}

.report-section-cards {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}

.report-card {
  cursor: default;
}

.report-card--selected {
  border-color: color-mix(in srgb, var(--accent) 40%, var(--border));
}

.report-card--expanded {
  grid-column: 1 / -1;
}

.report-card-main {
  display: flex;
  align-items: flex-start;
  gap: 10px;
}

.report-card-body {
  flex: 1;
  min-width: 0;
}

.report-card-title-row,
.report-dep-title-row {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 8px;
}

.report-card-cost,
.report-dep-cost {
  font-size: 11px;
  white-space: nowrap;
  font-family: var(--mono);
}

.report-card-desc {
  font-size: 12px;
  display: block;
  margin-top: 2px;
}

.report-card-status,
.report-dep-status {
  font-size: 11px;
  display: block;
  margin-top: 4px;
}

.report-deps {
  margin-top: 10px;
  margin-left: 28px;
  padding-top: 10px;
  padding-left: 12px;
  border-top: 1px solid var(--border);
  border-left: 2px solid color-mix(in srgb, var(--accent) 35%, var(--border));
}

.report-deps-heading {
  font-size: 11px;
  display: block;
  margin-bottom: 6px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.report-dep-row {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 4px 0;
}

.report-dep-row + .report-dep-row {
  border-top: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
}

.report-dep-body {
  flex: 1;
  min-width: 0;
}

.report-card--disabled {
  pointer-events: none;
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

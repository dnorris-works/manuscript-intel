<script setup lang="ts">
import { inject, ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { settingsKey, showPanelKey } from '../injectionKeys';
import type { ModelInfo, WinningCatImportResult, StaleCleanupResult } from '../types';
import { useReportTypes } from '../composables/useReportTypes';

const settingsCtx = inject(settingsKey)!;
const showPanel = inject(showPanelKey)!;
const { reportTypes, loadReportTypes } = useReportTypes();
loadReportTypes();

const savedMsg = ref('');
const modelFetchStatus = ref('');
const canopyTestStatus = ref('');
const dataforseoTestStatus = ref('');
const localTestStatus = ref('');

const isLocalProvider = computed(() => settingsCtx.provider.value === 'local');
const isTokenmixProvider = computed(() => settingsCtx.provider.value === 'tokenmix');

const localStatusText = computed(() => {
  const s = settingsCtx.localAiStatus.value;
  if (!s) return 'Checking...';
  if (!s.running) return 'Not running';
  if (!s.ready) return 'Starting...';
  return 'Running — bundled model ready';
});

// ── Sorted models: user-selectable sort order ─────────────────────────────────

type ModelSort = 'price' | 'provider';
const modelSort = ref<ModelSort>('price');
const pricedOnly = ref(false);
const freeOnly = ref(false);

const sortedModels = computed(() => {
  return [...settingsCtx.models.value].sort((a, b) => {
    if (modelSort.value === 'provider') {
      const provA = a.owned_by.toLowerCase();
      const provB = b.owned_by.toLowerCase();
      if (provA !== provB) return provA.localeCompare(provB);
    }
    const priceA = a.input_price ?? Infinity;
    const priceB = b.input_price ?? Infinity;
    return priceA - priceB;
  });
});

const filteredModels = computed(() => {
  return sortedModels.value.filter((m) => {
    const hasBothPrices = m.input_price != null && m.output_price != null;
    const isFree = isModelFreeLike(m);

    if (freeOnly.value) {
      return isFree;
    }
    if (pricedOnly.value) {
      return hasBothPrices;
    }
    return true;
  });
});

// ── Model fitness indicators ──────────────────────────────────────────────────

type Tier = 'basic' | 'capable' | 'strong';

function modelTier(m: ModelInfo): Tier {
  const price = m.input_price ?? 0;
  if (price <= 0.001) return 'basic';
  if (price <= 0.01) return 'capable';
  return 'strong';
}

const TIER_RANK: Record<Tier, number> = { basic: 0, capable: 1, strong: 2 };

function minTierFor(fnKey: string): Tier {
  if (fnKey === 'prose') return 'strong';
  const tiers = reportTypes.value
    .filter(r => r.model_slot === fnKey)
    .map(r => (r.min_tier as Tier) || 'basic');
  if (tiers.length === 0) return 'basic';
  return tiers.reduce((best, t) => (TIER_RANK[t] > TIER_RANK[best] ? t : best), 'basic' as Tier);
}

function modelFitLabel(m: ModelInfo, fnKey: string): string {
  const tier = modelTier(m);
  const min = minTierFor(fnKey);
  if (TIER_RANK[tier] >= TIER_RANK[min]) return ' ✓';
  return ' ⚠';
}

function formatUsd(value: number): string {
  if (value === 0) return '0';
  if (value < 0.001) return value.toFixed(4);
  if (value < 0.01) return value.toFixed(3);
  return value.toFixed(2);
}

function isModelFreeLike(m: ModelInfo): boolean {
  const hasBothPrices = m.input_price != null && m.output_price != null;
  const byPrice = hasBothPrices && m.input_price === 0 && m.output_price === 0;
  const byName = /(^|[\\/:_-])free([\\/:_-]|$)/i.test(m.id) || /free/i.test(m.owned_by || '');
  return byPrice || byName;
}

function modelPriceLabel(m: ModelInfo): string {
  const inPrice = m.input_price;
  const outPrice = m.output_price;

  if (inPrice == null && outPrice == null) {
    if (isModelFreeLike(m)) return 'FREE (provider-labeled; price not published)';
    return 'pricing unavailable';
  }

  const inText = inPrice == null ? 'in unknown' : `$${formatUsd(inPrice)} in`;
  const outText = outPrice == null ? 'out unknown' : `$${formatUsd(outPrice)} out`;

  if (isModelFreeLike(m) && inPrice === 0 && outPrice === 0) {
    return 'FREE (0/1K tokens)';
  }

  if (inPrice != null && outPrice != null) {
    const total = inPrice + outPrice;
    return `${inText} + ${outText} = ~$${formatUsd(total)} /1K`;
  }

  return `${inText} + ${outText} /1K`;
}

function fnOptionLabel(m: ModelInfo, fnKey: string): string {
  return `${m.id} (${modelPriceLabel(m)})${modelFitLabel(m, fnKey)}`;
}

const winningcatStatus = ref('');
const staleStatus = ref('');
const showStaleRow = ref(false);
const importDisabled = ref(false);
let lastImportedAt = '';

type SettingsTab = 'general' | 'ai' | 'folders' | 'canopy' | 'dataforseo' | 'winningcat';
const activeTab = ref<SettingsTab>('general');

const settingsTabs: { id: SettingsTab; label: string }[] = [
  { id: 'general', label: 'General' },
  { id: 'ai', label: 'AI Models' },
  { id: 'folders', label: 'Folders' },
  { id: 'canopy', label: 'Canopy' },
  { id: 'dataforseo', label: 'DataForSEO' },
  { id: 'winningcat', label: 'WinningCat' },
];

function modelLabel(m: ModelInfo): string {
  let label = m.id;
  if (m.owned_by) label += ` (${m.owned_by})`;
  label += ` — ${modelPriceLabel(m)}`;
  return label;
}

async function onFetchModels(): Promise<void> {
  modelFetchStatus.value = 'Fetching models...';
  const result = await settingsCtx.fetchModels();
  if (result.success) {
    modelFetchStatus.value = `${settingsCtx.models.value.length} models loaded.`;
  } else {
    modelFetchStatus.value = result.error;
  }
}

async function onTestLocalAi(): Promise<void> {
  localTestStatus.value = 'Testing...';
  const result = await settingsCtx.testLocalAi();
  localTestStatus.value = result.success
    ? `✓ ${result.reply || 'Connected'}`
    : '✗ ' + result.error;
}

async function onRefreshLocalStatus(): Promise<void> {
  await settingsCtx.refreshLocalAiStatus();
}

function onSave(): void {
  settingsCtx.saveSettings().then(() => {
    savedMsg.value = '✓ Saved';
    setTimeout(() => { savedMsg.value = ''; }, 1500);
    showPanel('analyzer');
  }).catch((e) => {
    savedMsg.value = 'Save failed: ' + String(e);
  });
}

async function onTestCanopy(): Promise<void> {
  canopyTestStatus.value = 'Testing...';
  const result = await settingsCtx.testCanopy();
  canopyTestStatus.value = result.success ? '✓ Connected' : '✗ ' + result.error;
}

async function onTestDataforseo(): Promise<void> {
  dataforseoTestStatus.value = 'Testing...';
  const result = await settingsCtx.testDataforseo();
  dataforseoTestStatus.value = result.success ? '✓ Connected' : '✗ ' + result.error;
}

async function onImportWinningCat(): Promise<void> {
  winningcatStatus.value = 'Select the CSV file...';
  importDisabled.value = true;
  showStaleRow.value = false;
  try {
    const result = await invoke<WinningCatImportResult>('import_winningcat_csv');
    if (result.success) {
      winningcatStatus.value = `✓ Imported ${result.imported} categories. Skipped ${result.skipped_other_department} (other department), ${result.skipped_unparseable} (unparseable).`;
      lastImportedAt = result.imported_at;
      if (result.stale_count > 0) {
        showStaleRow.value = true;
        const word = result.stale_count === 1 ? 'y was' : 'ies were';
        staleStatus.value = `${result.stale_count} categor${word} in the catalog from a previous import but missing from this one — possibly retired or renamed by Amazon.`;
      }
    } else {
      winningcatStatus.value = result.error || 'Import failed.';
    }
  } catch (e) {
    winningcatStatus.value = 'Error: ' + String(e);
  } finally {
    importDisabled.value = false;
  }
}

async function onRemoveStale(): Promise<void> {
  if (!lastImportedAt) return;
  if (!confirm('Remove these stale categories from the catalog? This only affects reference data — no story data is touched.')) return;
  try {
    const result = await invoke<StaleCleanupResult>('remove_stale_kdp_categories', { since: lastImportedAt });
    if (result.success) {
      const word = result.removed === 1 ? 'y' : 'ies';
      staleStatus.value = `✓ Removed ${result.removed} stale categor${word}.`;
      showStaleRow.value = false;
    } else {
      staleStatus.value = result.error || 'Cleanup failed.';
    }
  } catch (e) {
    staleStatus.value = 'Error: ' + String(e);
  }
}
</script>

<template>
  <div class="panel settings-panel">
    <h2 class="panel-title">Settings</h2>

    <div class="platform-tabs settings-tabs">
      <button
        v-for="tab in settingsTabs"
        :key="tab.id"
        class="platform-tab"
        :class="{ active: activeTab === tab.id }"
        @click="activeTab = tab.id"
      >{{ tab.label }}</button>
    </div>

    <!-- General -->
    <div v-show="activeTab === 'general'" class="settings-tab-panel">
      <div class="settings-form">
        <label>Theme</label>
        <div class="provider-options">
          <label class="provider-option" :class="{ active: settingsCtx.theme.value === 'dark' }">
            <input
              type="radio"
              name="theme"
              value="dark"
              :checked="settingsCtx.theme.value === 'dark'"
              @change="settingsCtx.setTheme('dark')"
            />
            Dark
          </label>
          <label class="provider-option" :class="{ active: settingsCtx.theme.value === 'light' }">
            <input
              type="radio"
              name="theme"
              value="light"
              :checked="settingsCtx.theme.value === 'light'"
              @change="settingsCtx.setTheme('light')"
            />
            Light
          </label>
        </div>
      </div>
    </div>

    <!-- AI Models -->
    <div v-show="activeTab === 'ai'" class="settings-tab-panel">
      <div class="settings-form">
        <label>AI Provider</label>
        <div class="provider-options">
          <label class="provider-option" :class="{ active: isLocalProvider }">
            <input
              type="radio"
              name="ai-provider"
              value="local"
              :checked="isLocalProvider"
              @change="settingsCtx.provider.value = 'local'"
            />
            Local (included)
          </label>
          <label class="provider-option" :class="{ active: isTokenmixProvider }">
            <input
              type="radio"
              name="ai-provider"
              value="tokenmix"
              :checked="isTokenmixProvider"
              @change="settingsCtx.provider.value = 'tokenmix'"
            />
            TokenMix (cloud)
          </label>
        </div>

        <!-- Local AI panel -->
        <template v-if="isLocalProvider">
          <label>Local AI Status</label>
          <div class="model-row">
            <span class="local-status">{{ localStatusText }}</span>
            <button class="btn btn-sm" @click="onRefreshLocalStatus">Refresh</button>
          </div>
          <p class="panel-desc">
            Default model <strong>{{ settingsCtx.localDefaultModel.value || 'phi4-mini' }}</strong> is bundled with the app — no download after install.
          </p>

          <div class="model-row">
            <button class="btn btn-sm" @click="onTestLocalAi">Test connection</button>
            <span class="model-fetch-status">{{ localTestStatus }}</span>
          </div>
        </template>

        <!-- TokenMix panel -->
        <template v-if="isTokenmixProvider">
          <label>API Key</label>
          <input
            type="password"
            v-model="settingsCtx.apiKey.value"
            placeholder="Enter your API key"
          />
        </template>

        <label>
          Default Model
          <span class="model-hint">Fetch models first, then assign each function below.</span>
        </label>
        <div class="model-row">
          <select v-model="settingsCtx.activeModelAssignments.value.default">
            <option v-if="settingsCtx.models.value.length === 0" value="" disabled>
              No models loaded
            </option>
            <option v-else-if="filteredModels.length === 0" value="" disabled>
              No models match current filters
            </option>
            <option
              v-for="m in filteredModels"
              :key="m.id"
              :value="m.id"
            >{{ modelLabel(m) }}</option>
          </select>
          <button class="btn btn-sm" @click="onFetchModels">Fetch Models</button>
        </div>
        <div class="model-fetch-status">{{ modelFetchStatus }}</div>

        <div v-if="isTokenmixProvider && sortedModels.length > 0" class="model-sort-row">
          <span class="model-sort-label">Sort:</span>
          <button class="model-sort-btn" :class="{ active: modelSort === 'price' }" @click="modelSort = 'price'">Price</button>
          <button class="model-sort-btn" :class="{ active: modelSort === 'provider' }" @click="modelSort = 'provider'">Provider</button>
        </div>

        <div v-if="isTokenmixProvider && sortedModels.length > 0" class="model-filter-row">
          <label class="model-filter-opt">
            <input type="checkbox" v-model="pricedOnly" :disabled="freeOnly" />
            Priced only
          </label>
          <label class="model-filter-opt">
            <input type="checkbox" v-model="freeOnly" />
            Free only
          </label>
          <span class="model-filter-count">{{ filteredModels.length }} shown</span>
        </div>

        <div v-if="sortedModels.length > 0" class="model-assignments">
          <div class="model-assign-header">Model per function</div>

          <div v-if="filteredModels.length === 0" class="model-filter-empty">
            No models match the current filters. Disable filters to see all models.
          </div>

          <div class="model-assign-row model-assign-note">
            <div class="model-assign-label">
              <strong>Chapter Fingerprints</strong>
              <span class="model-recommend">Instant deterministic scan in Rust — no AI model used.</span>
            </div>
          </div>

          <div class="model-assign-row">
            <div class="model-assign-label">
              <strong>Genre Analysis</strong>
              <span class="model-recommend">Classification task. Uses TokenMix when Local AI is your provider. Mid-tier model is sufficient.</span>
            </div>
            <select v-model="settingsCtx.activeModelAssignments.value.genre">
              <option value="">(Use default)</option>
              <option v-for="m in filteredModels" :key="m.id" :value="m.id">{{ fnOptionLabel(m, 'genre') }}</option>
            </select>
          </div>

          <div class="model-assign-row">
            <div class="model-assign-label">
              <strong>Keywords &amp; Categories</strong>
              <span class="model-recommend">Short structured output. Fast model works — speed over depth.</span>
            </div>
            <select v-model="settingsCtx.activeModelAssignments.value.keywords">
              <option value="">(Use default)</option>
              <option v-for="m in filteredModels" :key="m.id" :value="m.id">{{ fnOptionLabel(m, 'keywords') }}</option>
            </select>
          </div>

          <div class="model-assign-row">
            <div class="model-assign-label">
              <strong>Continuity Check</strong>
              <span class="model-recommend">Needs reasoning ability to spot contradictions across chapters. Use a capable model (e.g. GPT-4o or another high-reasoning model).</span>
            </div>
            <select v-model="settingsCtx.activeModelAssignments.value.continuity">
              <option value="">(Use default)</option>
              <option v-for="m in filteredModels" :key="m.id" :value="m.id">{{ fnOptionLabel(m, 'continuity') }}</option>
            </select>
          </div>

          <div class="model-assign-row">
            <div class="model-assign-label">
              <strong>Show Don't Tell</strong>
              <span class="model-recommend">Literary judgment — needs to understand prose craft. Use a strong model (e.g. GPT-4o or another high-reasoning model).</span>
            </div>
            <select v-model="settingsCtx.activeModelAssignments.value.showDontTell">
              <option value="">(Use default)</option>
              <option v-for="m in filteredModels" :key="m.id" :value="m.id">{{ fnOptionLabel(m, 'showDontTell') }}</option>
            </select>
          </div>

          <div class="model-assign-row">
            <div class="model-assign-label">
              <strong>AI-isms</strong>
              <span class="model-recommend">Literary judgment — spots synthetic / template-sounding prose. Use a strong model (e.g. GPT-4o or another high-reasoning model).</span>
            </div>
            <select v-model="settingsCtx.activeModelAssignments.value.aiIsms">
              <option value="">(Use default)</option>
              <option v-for="m in filteredModels" :key="m.id" :value="m.id">{{ fnOptionLabel(m, 'aiIsms') }}</option>
            </select>
          </div>

          <div class="model-assign-row">
            <div class="model-assign-label">
              <strong>Prose Suggestions</strong>
              <span class="model-recommend">Creative rewriting. Use the highest-quality model you have — this writes prose the author will paste into their manuscript.</span>
            </div>
            <select v-model="settingsCtx.activeModelAssignments.value.prose">
              <option value="">(Use default)</option>
              <option v-for="m in filteredModels" :key="m.id" :value="m.id">{{ fnOptionLabel(m, 'prose') }}</option>
            </select>
          </div>
        </div>
      </div>
    </div>

    <!-- Folders -->
    <div v-show="activeTab === 'folders'" class="settings-tab-panel">
      <div class="settings-form">
        <p class="panel-desc">
          Used when you choose <strong>Create empty story</strong>. The app uses these folders by purpose
          — you can rename the paths, but not remove them.
        </p>

        <label>Manuscript <span class="form-hint">— chapter files (analysis)</span></label>
        <input type="text" v-model="settingsCtx.folderStructure.value.manuscript" placeholder="Manuscript" />
        <p class="panel-desc manuscript-acts-hint">
          Always created under Manuscript:
          <template v-for="(act, i) in (settingsCtx.folderStructure.value.acts || [])" :key="act">
            <strong>{{ settingsCtx.folderStructure.value.manuscript || 'Manuscript' }}/{{ act }}</strong><span v-if="i < (settingsCtx.folderStructure.value.acts.length - 1)">, </span>
          </template>
          — not optional.
        </p>

        <label>Bible <span class="form-hint">— story bible docs</span></label>
        <input type="text" v-model="settingsCtx.folderStructure.value.bible" placeholder="Bible" />

        <label>Characters <span class="form-hint">— character docs</span></label>
        <input type="text" v-model="settingsCtx.folderStructure.value.characters" placeholder="Characters" />

        <label>Locations <span class="form-hint">— location docs</span></label>
        <input type="text" v-model="settingsCtx.folderStructure.value.locations" placeholder="Locations" />

        <label class="extra-folders-label">Additional folders</label>
        <p class="panel-desc extra-folders-desc">
          Created with new stories for your own use. The app does not read these specially — add or delete freely.
        </p>
        <div
          v-for="(_path, index) in settingsCtx.folderStructure.value.extra"
          :key="index"
          class="folder-entry-row"
        >
          <input
            type="text"
            v-model="settingsCtx.folderStructure.value.extra[index]"
            placeholder="Extra/Folder"
            class="folder-path-input"
          />
          <button
            type="button"
            class="btn btn-sm btn-danger"
            title="Remove folder"
            @click="settingsCtx.removeFolderEntry(index)"
          >Delete</button>
        </div>
        <button type="button" class="btn btn-sm btn-secondary" @click="settingsCtx.addFolderEntry()">
          Add Folder
        </button>
      </div>
    </div>

    <!-- Canopy -->
    <div v-show="activeTab === 'canopy'" class="settings-tab-panel">
      <div class="settings-form">
        <p class="panel-desc">Connect to the Canopy API for market intelligence reports.</p>
        <label>Canopy API Key</label>
        <input
          type="password"
          v-model="settingsCtx.canopyApiKey.value"
          placeholder="Enter Canopy API key"
        />
        <button class="btn btn-sm" @click="onTestCanopy">Test Connection</button>
        <div class="canopy-test-status">{{ canopyTestStatus }}</div>
      </div>
    </div>

    <!-- DataForSEO -->
    <div v-show="activeTab === 'dataforseo'" class="settings-tab-panel">
      <div class="settings-form">
        <p class="panel-desc">Used for keyword search volume data (Amazon + Google). Get credentials at <strong>app.dataforseo.com</strong>.</p>
        <label>Login (email)</label>
        <input
          type="text"
          v-model="settingsCtx.dataforseoLogin.value"
          placeholder="your@email.com"
        />
        <label>Password</label>
        <input
          type="password"
          v-model="settingsCtx.dataforseoPassword.value"
          placeholder="DataForSEO API password"
        />
        <button class="btn btn-sm" @click="onTestDataforseo">Test Connection</button>
        <div class="canopy-test-status">{{ dataforseoTestStatus }}</div>
      </div>
    </div>

    <!-- WinningCat -->
    <div v-show="activeTab === 'winningcat'" class="settings-tab-panel">
      <div class="settings-form">
        <p class="panel-desc">Import the WinningCat category catalog CSV to enable category matching.</p>
        <button class="btn" :disabled="importDisabled" @click="onImportWinningCat">Import CSV</button>
        <div class="winningcat-status">{{ winningcatStatus }}</div>
        <div v-if="showStaleRow" class="stale-row">
          <div class="stale-status">{{ staleStatus }}</div>
          <button class="btn btn-sm btn-danger" @click="onRemoveStale">Remove Stale</button>
        </div>
      </div>
    </div>

    <div v-if="activeTab !== 'winningcat'" class="settings-footer">
      <button class="btn" @click="onSave">Save Settings</button>
      <div class="settings-saved">{{ savedMsg }}</div>
    </div>
  </div>
</template>

<style scoped>
.settings-panel {
  padding: clamp(14px, 2vw, 24px);
  overflow-y: auto;
  width: 100%;
  max-width: none;
}

.panel-title {
  font-size: 16px;
  font-weight: 700;
  margin-bottom: 16px;
}

.settings-tabs {
  flex-wrap: wrap;
  margin-bottom: 16px;
}

.settings-tab-panel {
  min-height: 0;
}

.settings-footer {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 20px;
  padding-top: 16px;
  border-top: 1px solid var(--border);
}

.settings-form {
  display: flex;
  flex-direction: column;
  gap: 10px;
  width: 100%;
  max-width: none;
}

.settings-form label {
  font-size: 12px;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.settings-form input,
.settings-form select {
  background: linear-gradient(180deg, color-mix(in srgb, var(--surface2) 96%, white 4%), var(--surface2));
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text);
  font-size: 13px;
  padding: 8px 10px;
  width: 100%;
  user-select: text;
}

.settings-form select option {
  background: var(--surface2);
}

.settings-form select {
  appearance: none;
  -webkit-appearance: none;
  background-image:
    linear-gradient(45deg, transparent 50%, var(--text-muted) 50%),
    linear-gradient(135deg, var(--text-muted) 50%, transparent 50%);
  background-position:
    calc(100% - 18px) calc(50% - 2px),
    calc(100% - 12px) calc(50% - 2px);
  background-size: 6px 6px, 6px 6px;
  background-repeat: no-repeat;
  padding-right: 30px;
}

.settings-form select:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 20%, transparent 80%);
}

.settings-form select:disabled {
  opacity: 0.75;
}

.provider-options {
  display: flex;
  gap: 16px;
  flex-wrap: wrap;
}

.provider-option {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  color: var(--text);
  text-transform: none;
  letter-spacing: 0;
  cursor: pointer;
}

.provider-option input[type="radio"] {
  width: auto;
  accent-color: var(--accent);
}

.model-row {
  display: flex;
  gap: 8px;
  align-items: center;
}

@media (max-width: 760px) {
  .settings-form {
    width: 100%;
  }

  .model-row {
    flex-direction: column;
    align-items: stretch;
  }

  .model-row .btn {
    width: 100%;
  }
}

.model-row select {
  flex: 1;
}

.model-hint {
  display: block;
  font-size: 11px;
  color: var(--text-muted);
  font-weight: 400;
  text-transform: none;
  letter-spacing: 0;
  margin-top: 2px;
}

.model-fetch-status,
.canopy-test-status,
.winningcat-status {
  font-size: 12px;
  color: var(--text-muted);
  min-height: 16px;
}

.model-sort-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.model-filter-row {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.model-filter-opt {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-muted);
  text-transform: none;
  letter-spacing: 0;
}

.model-filter-opt input[type="checkbox"] {
  width: auto;
  accent-color: var(--accent);
}

.model-filter-count {
  font-size: 11px;
  color: var(--text-muted);
}

.model-sort-label {
  font-size: 11px;
  color: var(--text-muted);
}

.model-sort-btn {
  background: none;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text-muted);
  font-size: 11px;
  padding: 3px 8px;
  cursor: pointer;
}

.model-sort-btn.active {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}

.model-sort-btn:not(.active):hover {
  border-color: var(--accent);
  color: var(--text);
}

.model-assignments {
  margin-top: 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 12px;
}

.model-assign-header {
  font-size: 12px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted);
  margin-bottom: 10px;
}

.model-filter-empty {
  font-size: 12px;
  color: var(--text-muted);
  margin-bottom: 10px;
}

.model-assign-row {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 8px 0;
  border-bottom: 1px solid var(--border);
}

.model-assign-row:last-child {
  border-bottom: none;
  padding-bottom: 0;
}

.model-assign-label strong {
  font-size: 13px;
  color: var(--text);
}

.model-recommend {
  display: block;
  font-size: 11px;
  color: var(--text-muted);
  line-height: 1.4;
  margin-top: 2px;
  font-style: italic;
}

.model-assign-row select {
  background: var(--surface2);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text);
  font-size: 12px;
  padding: 5px 8px;
}

.settings-saved {
  font-size: 12px;
  color: var(--success);
  min-height: 18px;
}

.panel-desc {
  color: var(--text-muted);
  font-size: 13px;
  line-height: 1.5;
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
  align-self: flex-start;
}

.btn:hover { background: var(--accent-dim); }
.btn:disabled { background: var(--surface2); color: var(--text-muted); cursor: not-allowed; }

.btn-sm {
  padding: 6px 12px;
  font-size: 12px;
  white-space: nowrap;
}

.btn-danger {
  background: #c0392b;
  color: #fff;
}
.btn-danger:hover { background: #a93226; }

.btn-secondary {
  background: var(--surface2);
  border: 1px solid var(--border);
  color: var(--text-muted);
}
.btn-secondary:hover {
  color: var(--text);
  border-color: var(--accent);
}

.folder-entry-row {
  display: flex;
  gap: 8px;
  align-items: center;
  margin-bottom: 8px;
}

.folder-path-input {
  flex: 1;
  min-width: 0;
}

.extra-folders-label {
  margin-top: 12px;
}

.extra-folders-desc {
  margin: -4px 0 8px;
  font-size: 12px;
}

.form-hint {
  text-transform: none;
  letter-spacing: 0;
  font-weight: 400;
  font-size: 11px;
  color: var(--text-muted);
}

.stale-row {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 10px;
  background: var(--surface2);
  border: 1px solid var(--border);
  border-radius: var(--radius);
}

.stale-status {
  font-size: 12px;
  color: var(--text-muted);
}
</style>

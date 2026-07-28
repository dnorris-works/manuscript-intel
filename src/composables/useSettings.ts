import { ref, computed, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { ModelInfo, ModelsResult } from '../types';

// ── AI function model assignments ─────────────────────────────────────────────
// Each AI function can have its own model. Empty means "use the default model."

export interface ModelAssignments {
  default:       string;  // Fallback for any function without a specific model
  summaries:     string;  // Chapter summaries (extraction)
  genre:         string;  // Genre analysis & ranking
  keywords:      string;  // Keywords, search terms, BISAC
  continuity:    string;  // Continuity checker (fact extraction + judgment)
  showDontTell:  string;  // Show Don't Tell analysis
  aiIsms:        string;  // AI-isms check
  prose:         string;  // Creative suggestions / rewrites
}

export interface LocalAiStatus {
  running: boolean;
  ready: boolean;
  base_url: string;
  models: string[];
  default_model_installed: boolean;
}

export interface FolderStructure {
  /** Chapter files — analysis reads only here */
  manuscript: string;
  /** Story bible docs */
  bible: string;
  /** Character docs */
  characters: string;
  /** Location docs */
  locations: string;
  /** Act subfolders under manuscript (e.g. Act-1, Act-2, Act-3) */
  acts: string[];
  /** Extra scaffold-only folders (app does not use these) */
  extra: string[];
}

interface UiSettingsRow {
  theme: string;
  provider: string;
  api_key: string;
  tokenmix_api_key: string;
  model_assignments: string;
  local_default_model: string;
  local_model_assignments: string;
  canopy_api_key: string;
  dataforseo_login: string;
  dataforseo_password: string;
}

export interface SetupIssue {
  id: string;
  message: string;
}

const DEFAULT_FOLDER_STRUCTURE: FolderStructure = {
  manuscript: 'Manuscript',
  bible: 'Bible',
  characters: 'Characters',
  locations: 'Locations',
  acts: ['Act-1', 'Act-2', 'Act-3'],
  extra: ['Publishing/Cover', 'Research'],
};

export const DEFAULT_LOCAL_MODEL = 'phi4-mini';

export function manuscriptActPaths(structure?: FolderStructure): string[] {
  const s = structure || DEFAULT_FOLDER_STRUCTURE;
  const root = (s.manuscript || 'Manuscript').trim() || 'Manuscript';
  const acts = (Array.isArray(s.acts) && s.acts.length > 0)
    ? s.acts
    : DEFAULT_FOLDER_STRUCTURE.acts;
  return acts
    .map(a => a.trim())
    .filter(Boolean)
    .map(act => `${root}/${act}`);
}

function cloneStructure(s: FolderStructure): FolderStructure {
  const manuscript = s.manuscript || DEFAULT_FOLDER_STRUCTURE.manuscript;
  const acts = (Array.isArray(s.acts) && s.acts.length > 0)
    ? [...s.acts]
    : [...DEFAULT_FOLDER_STRUCTURE.acts];
  const rawExtra = Array.isArray(s.extra) ? [...s.extra] : [...DEFAULT_FOLDER_STRUCTURE.extra];
  const actSet = new Set(
    manuscriptActPaths({ ...DEFAULT_FOLDER_STRUCTURE, manuscript, acts }).map(p => p.toLowerCase())
  );
  const extra = rawExtra.filter(p => !actSet.has(p.replace(/\\/g, '/').toLowerCase()));
  return {
    manuscript,
    bible: s.bible || DEFAULT_FOLDER_STRUCTURE.bible,
    characters: s.characters || DEFAULT_FOLDER_STRUCTURE.characters,
    locations: s.locations || DEFAULT_FOLDER_STRUCTURE.locations,
    acts,
    extra,
  };
}

function defaultModelAssignments(): ModelAssignments {
  return {
    default: '', summaries: '', genre: '', keywords: '', continuity: '', showDontTell: '', aiIsms: '', prose: ''
  };
}

function loadAssignments(): ModelAssignments {
  return defaultModelAssignments();
}

export type ThemeMode = 'dark' | 'light';

function applyTheme(mode: ThemeMode): void {
  document.documentElement.setAttribute('data-theme', mode);
}

const theme = ref<ThemeMode>('dark');
applyTheme(theme.value);

function setTheme(mode: ThemeMode): void {
  theme.value = mode;
  applyTheme(mode);
  if (!settingsHydrated) return;
  scheduleUiSettingsSave();
}

const provider = ref('local');
const apiKey = ref('');
const tokenmixApiKey = ref('');
const modelAssignments = ref<ModelAssignments>(loadAssignments());
const localDefaultModel = ref(DEFAULT_LOCAL_MODEL);
const localModelAssignments = ref<ModelAssignments>(loadAssignments());
const canopyApiKey = ref('');
const dataforseoLogin = ref('');
const dataforseoPassword = ref('');
const models = ref<ModelInfo[]>([]);
const tokenmixModels = ref<ModelInfo[]>([]);
const folderStructure = ref<FolderStructure>(cloneStructure(DEFAULT_FOLDER_STRUCTURE));
const localAiStatus = ref<LocalAiStatus | null>(null);
let modelsAutoLoadStarted = false;
let settingsHydrated = false;
let uiSettingsSaveTimer: ReturnType<typeof setTimeout> | null = null;
let folderStructureSaveTimer: ReturnType<typeof setTimeout> | null = null;

function normalizeProvider(value: string | null | undefined): string {
  return value === 'tokenmix' ? 'tokenmix' : 'local';
}

const activeModelAssignments = computed(() =>
  provider.value === 'local' ? localModelAssignments.value : modelAssignments.value
);

/** TokenMix key used for genre/niche work when Local AI is the active provider. */
function effectiveTokenmixApiKey(): string {
  if (provider.value === 'tokenmix') {
    return apiKey.value.trim();
  }
  return tokenmixApiKey.value.trim();
}

function localAiReady(): boolean {
  return localAiStatus.value?.ready === true;
}

function checkLocalAiSetup(): SetupIssue[] {
  const issues: SetupIssue[] = [];
  if (!localAiReady()) {
    issues.push({
      id: 'local-ai',
      message: 'Local AI is not ready. Open Settings → AI Models and wait for the bundled model to start.',
    });
  }
  return issues;
}

function checkTokenmixGenreSetup(): SetupIssue[] {
  const issues: SetupIssue[] = [];
  if (!effectiveTokenmixApiKey()) {
    issues.push({
      id: 'tokenmix-key',
      message: 'TokenMix API key required for genre and niche classification. Add it in Settings → AI Models.',
    });
  }
  const genre = provider.value === 'local'
    ? activeModelAssignments.value.genre
    : modelFor('genre');
  if (!genre.trim()) {
    issues.push({
      id: 'genre-model',
      message: 'Assign a TokenMix model to Genre Analysis in Settings → AI Models.',
    });
  } else if (provider.value === 'local' && tokenmixModels.value.length > 0) {
    const known = tokenmixModels.value.some(m => m.id === genre);
    if (!known) {
      issues.push({
        id: 'genre-model-unknown',
        message: 'Genre Analysis model must be a TokenMix model. Fetch TokenMix models in Settings and pick one.',
      });
    }
  }
  return issues;
}

function checkCloudProviderSetup(): SetupIssue[] {
  const issues: SetupIssue[] = [];
  if (!apiKey.value.trim()) {
    issues.push({
      id: 'api-key',
      message: 'TokenMix API key required. Add it in Settings → AI Models.',
    });
  }
  if (!model.value.trim()) {
    issues.push({
      id: 'default-model',
      message: 'Select a default model in Settings → AI Models (fetch models first).',
    });
  }
  return issues;
}

/** KDP / Wide analyze — fingerprints are local; genre step uses TokenMix when provider is local. */
function checkPublishAnalyzeSetup(): SetupIssue[] {
  if (provider.value === 'local') {
    return [...checkLocalAiSetup(), ...checkTokenmixGenreSetup()];
  }
  return checkCloudProviderSetup();
}

/** Craft / Publish audits — local or cloud only; no TokenMix genre routing. */
function checkCraftAnalyzeSetup(): SetupIssue[] {
  if (provider.value === 'local') {
    return checkLocalAiSetup();
  }
  return checkCloudProviderSetup();
}

function checkMarketIntelSetup(): SetupIssue[] {
  const issues: SetupIssue[] = [];
  if (!canopyApiKey.value.trim()) {
    issues.push({
      id: 'canopy-key',
      message: 'Canopy API key required for Market Intel. Add it in Settings → Canopy.',
    });
  }
  if (provider.value === 'local') {
    issues.push(...checkLocalAiSetup());
  } else {
    issues.push(...checkCloudProviderSetup());
  }
  return issues;
}

watch([provider, apiKey], () => {
  provider.value = normalizeProvider(provider.value);
  if (provider.value === 'tokenmix' && apiKey.value.trim()) {
    void autoLoadModelsIfConfigured();
  } else if (provider.value === 'local') {
    void autoLoadModelsIfConfigured();
  }
});

// ── Convenience getters ───────────────────────────────────────────────────────

/** Resolve the model for a given function. Falls back to default if unset. */
function modelFor(fn: keyof ModelAssignments): string {
  const assignments = activeModelAssignments.value;
  return assignments[fn] || assignments.default;
}

const model = computed(() => activeModelAssignments.value.default);
const proseModel = computed(() => activeModelAssignments.value.prose || activeModelAssignments.value.default);

// ── Actions ──────────────────────────────────────────────────────────────────

async function refreshLocalAiStatus(): Promise<LocalAiStatus> {
  try {
    let status = await invoke<LocalAiStatus>('local_ai_status');
    if (!status.running || !status.ready) {
      try {
        await invoke('restart_local_ai');
        await new Promise((r) => setTimeout(r, 1500));
        status = await invoke<LocalAiStatus>('local_ai_status');
      } catch {
        // keep last status
      }
    }
    localAiStatus.value = status;
    return status;
  } catch {
    const fallback: LocalAiStatus = {
      running: false,
      ready: false,
      base_url: '',
      models: [],
      default_model_installed: false,
    };
    localAiStatus.value = fallback;
    return fallback;
  }
}

async function testLocalAi(): Promise<{ success: boolean; error: string; reply?: string }> {
  try {
    const reply = await invoke<string>('test_local_ai_connection');
    return { success: true, error: '', reply };
  } catch (e) {
    return { success: false, error: String(e) };
  }
}

async function fetchTokenmixModels(): Promise<{ success: boolean; error: string }> {
  const key = effectiveTokenmixApiKey();
  if (!key) {
    return { success: false, error: 'Enter your TokenMix API key first.' };
  }
  try {
    const result = await invoke<ModelsResult>('list_models', {
      provider: 'tokenmix',
      apiKey: key,
    });
    if (result.success && result.models.length > 0) {
      tokenmixModels.value = result.models;
      return { success: true, error: '' };
    }
    return { success: false, error: result.error || 'No TokenMix models returned.' };
  } catch (e) {
    return { success: false, error: 'Error: ' + String(e) };
  }
}

async function fetchModels(): Promise<{ success: boolean; error: string }> {
  if (provider.value === 'tokenmix' && !apiKey.value.trim()) {
    return { success: false, error: 'Enter a TokenMix API key first.' };
  }
  if (provider.value === 'local') {
    const status = await refreshLocalAiStatus();
    if (!status.ready) {
      return { success: false, error: 'Local AI is not ready yet.' };
    }
  }
  try {
    const result = await invoke<ModelsResult>('list_models', {
      provider: provider.value,
      apiKey: apiKey.value,
    });
    if (result.success && result.models.length > 0) {
      models.value = result.models;
      return { success: true, error: '' };
    }
    return { success: false, error: result.error || 'No models returned.' };
  } catch (e) {
    return { success: false, error: 'Error: ' + String(e) };
  }
}

async function autoLoadModelsIfConfigured(): Promise<void> {
  if (modelsAutoLoadStarted) return;
  if (provider.value === 'tokenmix' && !apiKey.value.trim()) return;
  if (provider.value === 'local') {
    const status = localAiStatus.value || await refreshLocalAiStatus();
    if (!status.ready) return;
    if (effectiveTokenmixApiKey()) {
      void fetchTokenmixModels();
    }
  }
  modelsAutoLoadStarted = true;
  try {
    await fetchModels();
  } finally {
    modelsAutoLoadStarted = false;
  }
}

async function loadFolderStructure(): Promise<void> {
  try {
    const result = await invoke<FolderStructure>('get_folder_structure');
    folderStructure.value = cloneStructure(result);
  } catch {
    folderStructure.value = cloneStructure(DEFAULT_FOLDER_STRUCTURE);
  }
}

function addFolderEntry(): void {
  folderStructure.value.extra.push('');
}

function removeFolderEntry(index: number): void {
  folderStructure.value.extra.splice(index, 1);
}

function currentUiSettingsPayload(): UiSettingsRow {
  return {
    theme: theme.value,
    provider: provider.value,
    api_key: apiKey.value.trim(),
    tokenmix_api_key: tokenmixApiKey.value.trim(),
    model_assignments: JSON.stringify(modelAssignments.value),
    local_default_model: localDefaultModel.value.trim() || DEFAULT_LOCAL_MODEL,
    local_model_assignments: JSON.stringify(localModelAssignments.value),
    canopy_api_key: canopyApiKey.value.trim(),
    dataforseo_login: dataforseoLogin.value.trim(),
    dataforseo_password: dataforseoPassword.value.trim(),
  };
}

async function persistUiSettings(): Promise<void> {
  await invoke<UiSettingsRow>('save_ui_settings', {
    settings: currentUiSettingsPayload(),
  });
}

function scheduleUiSettingsSave(): void {
  if (!settingsHydrated) return;
  if (uiSettingsSaveTimer) clearTimeout(uiSettingsSaveTimer);
  uiSettingsSaveTimer = setTimeout(() => {
    uiSettingsSaveTimer = null;
    void persistUiSettings();
  }, 1800);
}

function scheduleFolderStructureSave(): void {
  if (!settingsHydrated) return;
  if (folderStructureSaveTimer) clearTimeout(folderStructureSaveTimer);
  folderStructureSaveTimer = setTimeout(() => {
    folderStructureSaveTimer = null;
    void invoke<FolderStructure>('save_folder_structure', {
      structure: folderStructure.value,
    }).then((saved) => {
      folderStructure.value = cloneStructure(saved);
    }).catch((e) => {
      console.error('save_folder_structure:', e);
    });
  }, 1800);
}

async function saveSettings(): Promise<void> {
  const saved = await invoke<FolderStructure>('save_folder_structure', {
    structure: folderStructure.value,
  });
  folderStructure.value = cloneStructure(saved);

  await persistUiSettings();

  await autoLoadModelsIfConfigured();
}

async function hydrateSettings(): Promise<void> {
  try {
    const loaded = await invoke<UiSettingsRow>('load_ui_settings');
    theme.value = (loaded.theme as ThemeMode) === 'light' ? 'light' : 'dark';
    applyTheme(theme.value);
    provider.value = normalizeProvider(loaded.provider || 'local');
    apiKey.value = loaded.api_key || '';
    tokenmixApiKey.value = loaded.tokenmix_api_key || '';
    localDefaultModel.value = loaded.local_default_model || DEFAULT_LOCAL_MODEL;
    canopyApiKey.value = loaded.canopy_api_key || '';
    dataforseoLogin.value = loaded.dataforseo_login || '';
    dataforseoPassword.value = loaded.dataforseo_password || '';
    if (loaded.model_assignments) {
      try {
        modelAssignments.value = { ...defaultModelAssignments(), ...JSON.parse(loaded.model_assignments) };
      } catch {
        modelAssignments.value = defaultModelAssignments();
      }
    }
    if (loaded.local_model_assignments) {
      try {
        localModelAssignments.value = { ...defaultModelAssignments(), ...JSON.parse(loaded.local_model_assignments) };
      } catch {
        localModelAssignments.value = defaultModelAssignments();
      }
    }
    if (!localModelAssignments.value.default) {
      localModelAssignments.value.default = localDefaultModel.value || DEFAULT_LOCAL_MODEL;
    }
  } finally {
    settingsHydrated = true;
    void refreshLocalAiStatus();
    void autoLoadModelsIfConfigured();
  }
}

void hydrateSettings();

void listen('local-ai:ready', () => {
  void refreshLocalAiStatus().then(() => autoLoadModelsIfConfigured());
});

watch([theme, provider, apiKey, tokenmixApiKey, modelAssignments, localModelAssignments, localDefaultModel, canopyApiKey, dataforseoLogin, dataforseoPassword], () => {
  if (!settingsHydrated) return;
  scheduleUiSettingsSave();
}, { deep: true });

watch(folderStructure, () => {
  if (!settingsHydrated) return;
  scheduleFolderStructureSave();
}, { deep: true });

watch(tokenmixApiKey, () => {
  if (!settingsHydrated) return;
  if (provider.value === 'local' && tokenmixApiKey.value.trim()) {
    void fetchTokenmixModels();
  }
});

watch(provider, () => {
  if (!settingsHydrated) return;
  models.value = [];
  void autoLoadModelsIfConfigured();
});

async function testCanopy(): Promise<{ success: boolean; error: string }> {
  const key = canopyApiKey.value.trim();
  if (!key) {
    return { success: false, error: 'Enter a key first.' };
  }
  try {
    const result = await invoke<{ success: boolean; error: string }>('test_canopy_connection', { apiKey: key });
    return result;
  } catch (e) {
    return { success: false, error: String(e) };
  }
}

async function testDataforseo(): Promise<{ success: boolean; error: string }> {
  const login = dataforseoLogin.value.trim();
  const password = dataforseoPassword.value.trim();
  if (!login || !password) {
    return { success: false, error: 'Enter login and password first.' };
  }
  try {
    const result = await invoke<{ success: boolean; error: string }>('test_dataforseo_connection', { login, password });
    return result;
  } catch (e) {
    return { success: false, error: String(e) };
  }
}

export function useSettings() {
  return {
    theme,
    setTheme,
    provider,
    apiKey,
    tokenmixApiKey,
    effectiveTokenmixApiKey,
    model,
    proseModel,
    modelAssignments,
    localDefaultModel,
    localModelAssignments,
    activeModelAssignments,
    modelFor,
    canopyApiKey,
    dataforseoLogin,
    dataforseoPassword,
    models,
    tokenmixModels,
    folderStructure,
    localAiStatus,
    fetchModels,
    fetchTokenmixModels,
    refreshLocalAiStatus,
    testLocalAi,
    checkPublishAnalyzeSetup,
    checkCraftAnalyzeSetup,
    checkMarketIntelSetup,
    loadFolderStructure,
    addFolderEntry,
    removeFolderEntry,
    saveSettings,
    testCanopy,
    testDataforseo,
  };
}

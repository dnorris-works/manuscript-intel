import { ref, computed, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { ModelInfo, ModelsResult } from '../types';
import { applyThemeTokens, type ThemeMode } from '../theme/tokens';
import { refreshAiSpend } from './useAiSpend';
import { findPricedModel } from '../reportCostPricing';

export type { ThemeMode };

// ── AI function model assignments ─────────────────────────────────────────────
// Each AI function can have its own model. Empty means "use the default model."

export interface ModelAssignments {
  default:       string;  // Fallback for any function without a specific model
  summaries:     string;  // Chapter summary (genre signals) extraction
  genre:         string;  // Genre analysis & ranking
  keywords:      string;  // Keywords, search terms, BISAC
  continuity:    string;  // Continuity checker (fact extraction + judgment)
  showDontTell:  string;  // Show Don't Tell analysis
  aiIsms:        string;  // AI-isms check
  prose:         string;  // Creative suggestions / rewrites
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

export interface SetupIssue {
  id: string;
  message: string;
}

interface UiSettingsRow {
  theme: string;
  api_key: string;
  model_assignments: string;
  canopy_api_key: string;
  dataforseo_login: string;
  dataforseo_password: string;
}

const EMPTY_FOLDER_STRUCTURE: FolderStructure = {
  manuscript: '',
  bible: '',
  characters: '',
  locations: '',
  acts: [],
  extra: [],
};

let folderDefaults: FolderStructure | null = null;

async function ensureFolderDefaults(): Promise<FolderStructure> {
  if (!folderDefaults) {
    try {
      folderDefaults = await invoke<FolderStructure>('get_default_folder_structure');
    } catch {
      folderDefaults = { ...EMPTY_FOLDER_STRUCTURE };
    }
  }
  return folderDefaults;
}

export function manuscriptActPaths(structure: FolderStructure): string[] {
  const root = (structure.manuscript || 'Manuscript').trim() || 'Manuscript';
  const acts = (Array.isArray(structure.acts) && structure.acts.length > 0)
    ? structure.acts
    : ['Act-1', 'Act-2', 'Act-3'];
  return acts
    .map(a => a.trim())
    .filter(Boolean)
    .map(act => `${root}/${act}`);
}

function cloneStructure(s: FolderStructure, defaults: FolderStructure): FolderStructure {
  const manuscript = s.manuscript || defaults.manuscript;
  const acts = (Array.isArray(s.acts) && s.acts.length > 0)
    ? [...s.acts]
    : [...defaults.acts];
  const rawExtra = Array.isArray(s.extra) ? [...s.extra] : [...defaults.extra];
  const actSet = new Set(
    manuscriptActPaths({ ...defaults, manuscript, acts }).map(p => p.toLowerCase())
  );
  const extra = rawExtra.filter(p => !actSet.has(p.replace(/\\/g, '/').toLowerCase()));
  return {
    manuscript,
    bible: s.bible || defaults.bible,
    characters: s.characters || defaults.characters,
    locations: s.locations || defaults.locations,
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


function applyTheme(mode: ThemeMode): void {
  applyThemeTokens(mode);
}

const theme = ref<ThemeMode>('dark');
applyTheme(theme.value);

function setTheme(mode: ThemeMode): void {
  theme.value = mode;
  applyTheme(mode);
  if (!settingsHydrated) return;
  scheduleUiSettingsSave();
}

const apiKey = ref('');
const modelAssignments = ref<ModelAssignments>(loadAssignments());
const canopyApiKey = ref('');
const dataforseoLogin = ref('');
const dataforseoPassword = ref('');
const models = ref<ModelInfo[]>([]);
const folderStructure = ref<FolderStructure>({ ...EMPTY_FOLDER_STRUCTURE });
let modelsAutoLoadStarted = false;
let settingsHydrated = false;
let uiSettingsSaveTimer: ReturnType<typeof setTimeout> | null = null;
let folderStructureSaveTimer: ReturnType<typeof setTimeout> | null = null;

const activeModelAssignments = computed(() => modelAssignments.value);

/** Always TokenMix — kept for IPC payloads. */
const provider = computed(() => 'tokenmix');

function modelFor(fn: keyof ModelAssignments): string {
  const assignments = modelAssignments.value;
  const chosen = assignments[fn] || assignments.default;
  return findPricedModel(chosen, models.value)?.id ?? '';
}

function sanitizeModelAssignments(): void {
  const list = models.value;
  if (list.length === 0) return;

  const next = { ...modelAssignments.value };
  let changed = false;

  const keepIfPriced = (id: string): string => {
    if (!id) return '';
    if (!findPricedModel(id, list)) {
      changed = true;
      return '';
    }
    return id;
  };

  (Object.keys(next) as (keyof ModelAssignments)[]).forEach((key) => {
    const cleaned = keepIfPriced(next[key]);
    if (cleaned !== next[key]) next[key] = cleaned;
  });

  if (changed) {
    modelAssignments.value = next;
  }
}

const model = computed(() => modelAssignments.value.default);
const proseModel = computed(() => modelAssignments.value.prose || modelAssignments.value.default);

function checkAiSetup(): SetupIssue[] {
  const issues: SetupIssue[] = [];
  if (!apiKey.value.trim()) {
    issues.push({
      id: 'api-key',
      message: 'TokenMix API key required. Add it in Settings → AI Models.',
    });
  }
  if (models.value.length === 0) {
    issues.push({
      id: 'models-not-loaded',
      message: 'Fetch models in Settings → AI Models before running reports.',
    });
    return issues;
  }
  if (!modelAssignments.value.default.trim()) {
    issues.push({
      id: 'default-model',
      message: 'Select a default model with published pricing in Settings → AI Models.',
    });
  } else if (!findPricedModel(modelAssignments.value.default, models.value)) {
    issues.push({
      id: 'default-model-pricing',
      message: 'The selected default model has no published pricing. Fetch models and choose a priced model.',
    });
  }
  return issues;
}

function checkPublishAnalyzeSetup(): SetupIssue[] {
  return checkAiSetup();
}

function checkCraftAnalyzeSetup(): SetupIssue[] {
  return checkAiSetup();
}

function checkMarketIntelSetup(): SetupIssue[] {
  const issues = checkAiSetup();
  if (!canopyApiKey.value.trim()) {
    issues.push({
      id: 'canopy-key',
      message: 'Canopy API key required for Market Intel. Add it in Settings → Canopy.',
    });
  }
  return issues;
}

async function fetchModels(): Promise<{ success: boolean; error: string }> {
  if (!apiKey.value.trim()) {
    return { success: false, error: 'Enter your TokenMix API key first.' };
  }
  try {
    const result = await invoke<ModelsResult>('list_models', {
      provider: 'tokenmix',
      apiKey: apiKey.value,
    });
    if (result.success && result.models.length > 0) {
      models.value = result.models;
      sanitizeModelAssignments();
      await refreshAiSpend();
      return { success: true, error: '' };
    }
    return { success: false, error: result.error || 'No models returned.' };
  } catch (e) {
    return { success: false, error: 'Error: ' + String(e) };
  }
}

async function autoLoadModelsIfConfigured(): Promise<void> {
  if (modelsAutoLoadStarted) return;
  if (!apiKey.value.trim()) return;
  modelsAutoLoadStarted = true;
  try {
    await fetchModels();
  } finally {
    modelsAutoLoadStarted = false;
  }
}

async function loadFolderStructure(): Promise<void> {
  const defaults = await ensureFolderDefaults();
  try {
    const result = await invoke<FolderStructure>('get_folder_structure');
    folderStructure.value = cloneStructure(result, defaults);
  } catch {
    folderStructure.value = cloneStructure(defaults, defaults);
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
    api_key: apiKey.value.trim(),
    model_assignments: JSON.stringify(modelAssignments.value),
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
      void ensureFolderDefaults().then((defaults) => {
        folderStructure.value = cloneStructure(saved, defaults);
      });
    }).catch((e) => {
      console.error('save_folder_structure:', e);
    });
  }, 1800);
}

async function saveSettings(): Promise<void> {
  const defaults = await ensureFolderDefaults();
  const saved = await invoke<FolderStructure>('save_folder_structure', {
    structure: folderStructure.value,
  });
  folderStructure.value = cloneStructure(saved, defaults);

  await persistUiSettings();
  await autoLoadModelsIfConfigured();
}

async function hydrateSettings(): Promise<void> {
  try {
    const loaded = await invoke<UiSettingsRow>('load_ui_settings');
    theme.value = (loaded.theme as ThemeMode) === 'light' ? 'light' : 'dark';
    applyTheme(theme.value);
    apiKey.value = loaded.api_key || '';
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
  } finally {
    settingsHydrated = true;
    void autoLoadModelsIfConfigured();
  }
}

void hydrateSettings();

watch([theme, apiKey, modelAssignments, canopyApiKey, dataforseoLogin, dataforseoPassword], () => {
  if (!settingsHydrated) return;
  scheduleUiSettingsSave();
}, { deep: true });

watch(folderStructure, () => {
  if (!settingsHydrated) return;
  scheduleFolderStructureSave();
}, { deep: true });

watch(apiKey, () => {
  if (!settingsHydrated) return;
  models.value = [];
  modelAssignments.value = defaultModelAssignments();
  void autoLoadModelsIfConfigured();
});

watch(models, () => {
  if (!settingsHydrated) return;
  sanitizeModelAssignments();
}, { deep: true });

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
    model,
    proseModel,
    modelAssignments,
    activeModelAssignments,
    modelFor,
    canopyApiKey,
    dataforseoLogin,
    dataforseoPassword,
    models,
    folderStructure,
    fetchModels,
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

<script setup lang="ts">
import { ref, watch, onMounted, provide, computed } from 'vue';
import { NConfigProvider, NMessageProvider, NDialogProvider, darkTheme } from 'naive-ui';
import { getNaiveThemeOverrides } from './naiveTheme';
import { useStories } from './composables/useStories';
import { useAnalysis } from './composables/useAnalysis';
import { usePlatform } from './composables/usePlatform';
import { useSettings } from './composables/useSettings';
import { useReports } from './composables/useReports';
import { useSeries } from './composables/useSeries';
import { useCampaigns } from './composables/useCampaigns';
import { useResizableWidth } from './composables/useResizableWidth';
import {
  storiesKey, analysisKey, platformKey, settingsKey,
  reportsKey, seriesKey, campaignsKey, showPanelKey, openManuscriptEditorKey,
} from './injectionKeys';
import type { Story, Finding, Series } from './types';

import TitleBar from './components/TitleBar.vue';
import Sidebar from './components/Sidebar.vue';
import AnalyzerPanel from './components/AnalyzerPanel.vue';
import SavedReportsPanel from './components/SavedReportsPanel.vue';
import ReportsViewer from './components/ReportsViewer.vue';
import SettingsPanel from './components/settings/SettingsPanel.vue';
import HelpPanel from './components/HelpPanel.vue';
import StoryForm from './components/StoryForm.vue';
import SeriesForm from './components/SeriesForm.vue';
import NewDocumentForm from './components/NewDocumentForm.vue';
import ManuscriptViewer from './components/ManuscriptViewer.vue';
import WritingPanel from './components/WritingPanel.vue';
import CampaignsPanel from './components/marketing/CampaignsPanel.vue';
import CampaignForm from './components/marketing/CampaignForm.vue';
import CampaignDetailPanel from './components/marketing/CampaignDetailPanel.vue';
import PlatformAccountsPanel from './components/marketing/PlatformAccountsPanel.vue';

// ── Composables ───────────────────────────────────────────────────────────────

const storiesCtx = useStories();
const analysisCtx = useAnalysis();
const platformCtx = usePlatform();
const settingsCtx = useSettings();
const reportsCtx = useReports();
const seriesCtx = useSeries();
const campaignsCtx = useCampaigns();

provide(storiesKey, storiesCtx);
provide(analysisKey, analysisCtx);
provide(platformKey, platformCtx);
provide(settingsKey, settingsCtx);
provide(reportsKey, reportsCtx);
provide(seriesKey, seriesCtx);
provide(campaignsKey, campaignsCtx);

const naiveTheme = computed(() => (settingsCtx.theme.value === 'dark' ? darkTheme : null));
const naiveThemeOverrides = computed(() => getNaiveThemeOverrides(settingsCtx.theme.value));

const { width: sidebarWidth, startResize: startSidebarResize } = useResizableWidth({
  storageKey: 'sidebar-width',
  defaultWidth: 220,
  min: 160,
  max: 480,
});

// ── Top-level mode ────────────────────────────────────────────────────────────

type AppMode = 'analyzer' | 'writing' | 'marketing';
const appMode = ref<AppMode>('analyzer');

provide('appMode', appMode);
provide('setAppMode', (mode: AppMode) => {
  appMode.value = mode;
  if (mode === 'marketing') {
    activePanel.value = 'campaigns';
  } else if (mode === 'analyzer') {
    const marketingPanels: Panel[] = ['campaigns', 'campaign-detail', 'campaign-form', 'platform-accounts'];
    if (marketingPanels.includes(activePanel.value)) {
      activePanel.value = 'analyzer';
    }
  }
});

// ── Panel state (within Analyzer mode) ────────────────────────────────────────

type Panel = 'analyzer' | 'reports' | 'settings' | 'help' | 'story-form' | 'series' | 'manuscript' | 'new-document' | 'campaigns' | 'campaign-detail' | 'campaign-form' | 'platform-accounts';
const activePanel = ref<Panel>('analyzer');
const prevPanel = ref<Panel>('analyzer');
/** Panel to restore after cancelling New Document (works across writing/analyzer). */
const panelBeforeNewDoc = ref<Panel>('analyzer');
const modeBeforeNewDoc = ref<AppMode>('analyzer');

function showPanel(name: Panel): void {
  if ((name === 'settings' || name === 'help') && activePanel.value === name) {
    activePanel.value = prevPanel.value;
    return;
  }
  if ((name === 'settings' || name === 'help') && activePanel.value !== name) {
    prevPanel.value = activePanel.value;
  }
  activePanel.value = name;
}

provide(showPanelKey, showPanel as (name: string) => void);

const fileTreeTick = ref(0);
provide('fileTreeTick', fileTreeTick);

// ── Manuscript editor state (for report findings) ─────────────────────────────

const manuscriptFindings = ref<Finding[]>([]);
const manuscriptStartIndex = ref(0);
/** Where to return after closing the manuscript editor. */
const manuscriptReturnPanel = ref<Panel>('analyzer');

function openManuscriptEditor(findings: Finding[], startIndex: number): void {
  manuscriptFindings.value = findings;
  manuscriptStartIndex.value = startIndex;
  // From a report → return to reports; from Files/elsewhere → analyzer.
  // If already in the manuscript editor, keep the existing return target.
  if (activePanel.value === 'reports') {
    manuscriptReturnPanel.value = 'reports';
  } else if (activePanel.value !== 'manuscript') {
    manuscriptReturnPanel.value = 'analyzer';
  }
  activePanel.value = 'manuscript';
}

function closeManuscriptEditor(): void {
  const target = manuscriptReturnPanel.value;
  // Don't land on an empty reports shell (Copy/Delete with no title)
  if (target === 'reports' && !reportsCtx.currentReport.value) {
    activePanel.value = 'analyzer';
  } else {
    activePanel.value = target;
  }
}

provide(openManuscriptEditorKey, openManuscriptEditor);

// ── Writing mode state ────────────────────────────────────────────────────────

const writingFilePath = ref('');
const writingChapterTitle = ref('');
const newDocLocation = ref<string | undefined>(undefined);
/** Folder browsed in Writing mode when the book is only linked via series metadata. */
const writingBrowseFolder = ref('');

const effectiveStoryFolder = computed(() => {
  return storiesCtx.activeFolder.value || writingBrowseFolder.value;
});

provide('writingBrowseFolder', writingBrowseFolder);

function openInWritingMode(filePath: string, title: string): void {
  writingFilePath.value = filePath;
  writingChapterTitle.value = title;
  appMode.value = 'writing';
}

provide('openInWritingMode', openInWritingMode);

function closeWritingDocument(): void {
  writingFilePath.value = '';
  writingChapterTitle.value = '';
  appMode.value = 'analyzer';
  activePanel.value = 'analyzer';
}

provide('closeWritingDocument', closeWritingDocument);

function bumpFileTree(): void {
  fileTreeTick.value += 1;
}

provide('bumpFileTree', bumpFileTree);

function openNewDocumentForm(location?: string): void {
  if (!effectiveStoryFolder.value) return;
  panelBeforeNewDoc.value = activePanel.value;
  modeBeforeNewDoc.value = appMode.value;
  newDocLocation.value = location;
  activePanel.value = 'new-document';
}

provide('openNewDocumentForm', openNewDocumentForm);

function onDocumentCreated(path: string, title: string): void {
  fileTreeTick.value += 1;
  newDocLocation.value = undefined;
  writingFilePath.value = path;
  writingChapterTitle.value = title;
  appMode.value = 'writing';
  activePanel.value = 'analyzer';
}

function onDocumentFormCancel(): void {
  newDocLocation.value = undefined;
  activePanel.value = panelBeforeNewDoc.value;
  appMode.value = modeBeforeNewDoc.value;
}

// ── Story form state ──────────────────────────────────────────────────────────

const editingStory = ref<Story | null>(null);

function openStoryForm(story: Story | null): void {
  editingStory.value = story;
  showPanel('story-form');
}

// ── Series form state ─────────────────────────────────────────────────────────

const editingSeries = ref<Series | null>(null);

function openSeriesForm(series: Series | null): void {
  editingSeries.value = series;
  showPanel('series');
}

// ── Campaign state ────────────────────────────────────────────────────────────

const editingCampaignId = ref<number | null>(null);

function openCampaignDetail(id: number): void {
  editingCampaignId.value = id;
  showPanel('campaign-detail');
}

function openCampaignForm(id: number | null = null): void {
  editingCampaignId.value = id;
  showPanel('campaign-form');
}

function onCampaignSaved(id: number): void {
  editingCampaignId.value = id;
  const folder = storiesCtx.activeFolder.value;
  if (folder) void campaignsCtx.loadCampaigns(folder);
  showPanel('campaign-detail');
}

function onCampaignFormCancel(): void {
  if (editingCampaignId.value) {
    showPanel('campaign-detail');
  } else {
    showPanel('campaigns');
  }
}

function onCampaignDetailBack(): void {
  editingCampaignId.value = null;
  showPanel('campaigns');
}

function openPlatformAccounts(): void {
  showPanel('platform-accounts');
}

// ── Watchers ──────────────────────────────────────────────────────────────────

function refreshSavedReports(folder: string): void {
  void reportsCtx.loadSavedReports(folder);
}

watch(() => storiesCtx.activeStoryId.value, (id) => {
  if (id && storiesCtx.activeFolder.value) {
    analysisCtx.refreshState(storiesCtx.activeFolder.value);
    refreshSavedReports(storiesCtx.activeFolder.value);
    void campaignsCtx.loadCampaigns(storiesCtx.activeFolder.value);
    void campaignsCtx.loadLandingPages(storiesCtx.activeFolder.value);
  } else {
    analysisCtx.refreshState('');
    refreshSavedReports('');
    void campaignsCtx.loadCampaigns('');
  }
});

watch(() => platformCtx.platform.value, () => {
  if (storiesCtx.activeFolder.value && platformCtx.platform.value === 'saved') {
    void reportsCtx.loadSavedReports(storiesCtx.activeFolder.value);
  }
});

watch(() => analysisCtx.isWorking.value, (working, wasWorking) => {
  if (wasWorking && !working && storiesCtx.activeFolder.value) {
    analysisCtx.refreshState(storiesCtx.activeFolder.value);
    void reportsCtx.loadSavedReports(storiesCtx.activeFolder.value);
  }
});

// ── Context menu prevention ───────────────────────────────────────────────────

onMounted(() => {
  document.addEventListener('contextmenu', (e) => {
    const tag = (e.target as HTMLElement).tagName;
    if (!['INPUT', 'TEXTAREA', 'SELECT'].includes(tag) && !(e.target as HTMLElement).isContentEditable) {
      e.preventDefault();
    }
  });

  settingsCtx.loadFolderStructure();
  storiesCtx.loadStories().then(() => {
    const folder = storiesCtx.activeFolder.value;
    if (folder) {
      analysisCtx.refreshState(folder);
      void reportsCtx.loadSavedReports(folder);
    }
  });
  seriesCtx.loadSeries();
  void campaignsCtx.loadPlatformAccounts();
});
</script>

<template>
  <n-config-provider :theme="naiveTheme" :theme-overrides="naiveThemeOverrides">
    <n-message-provider>
      <n-dialog-provider>
        <div id="app-root">
    <TitleBar />
    <div class="sidebar-column" :style="{ width: `${sidebarWidth}px` }">
      <Sidebar
        @open-story-form="openStoryForm"
        @open-series-form="openSeriesForm"
        @open-campaign-form="openCampaignForm"
        @open-campaign-detail="openCampaignDetail"
        @open-platform-accounts="openPlatformAccounts"
      />
      <div
        class="sidebar-resizer"
        title="Drag to resize"
        @mousedown="startSidebarResize"
      />
    </div>
    <main id="main">
      <NewDocumentForm
        v-if="activePanel === 'new-document'"
        :initial-location="newDocLocation"
        @created="onDocumentCreated"
        @cancel="onDocumentFormCancel"
      />

      <HelpPanel v-else-if="activePanel === 'help'" />
      <SettingsPanel v-else-if="activePanel === 'settings'" />
      <SeriesForm v-else-if="activePanel === 'series'" :series="editingSeries" />

      <!-- Writing mode -->
      <WritingPanel
        v-else-if="appMode === 'writing'"
        :file-path="writingFilePath"
        :chapter-title="writingChapterTitle"
        :story-folder="effectiveStoryFolder"
      />

      <!-- Marketing mode -->
      <template v-else-if="appMode === 'marketing'">
        <CampaignsPanel
          v-if="activePanel === 'campaigns'"
          @open-campaign="openCampaignDetail"
          @new-campaign="openCampaignForm(null)"
        />
        <CampaignDetailPanel
          v-if="activePanel === 'campaign-detail' && editingCampaignId"
          :campaign-id="editingCampaignId"
          @back="onCampaignDetailBack"
          @edit="openCampaignForm(editingCampaignId)"
        />
        <CampaignForm
          v-if="activePanel === 'campaign-form'"
          :campaign-id="editingCampaignId"
          @saved="onCampaignSaved"
          @cancel="onCampaignFormCancel"
        />
        <PlatformAccountsPanel v-if="activePanel === 'platform-accounts'" />
      </template>

      <!-- Analyzer mode panels -->
      <template v-else-if="appMode === 'analyzer'">
        <SavedReportsPanel v-if="activePanel === 'analyzer' && platformCtx.platform.value === 'saved'" />
        <AnalyzerPanel v-else-if="activePanel === 'analyzer'" />
        <ReportsViewer v-if="activePanel === 'reports'" />
        <StoryForm v-if="activePanel === 'story-form'" :story="editingStory" />
        <ManuscriptViewer
          v-if="activePanel === 'manuscript'"
          :findings="manuscriptFindings"
          :start-index="manuscriptStartIndex"
          :story-folder="effectiveStoryFolder"
          @close="closeManuscriptEditor"
        />
      </template>
    </main>
        </div>
      </n-dialog-provider>
    </n-message-provider>
  </n-config-provider>
</template>

<style scoped>
#app-root {
  display: grid;
  grid-template-rows: var(--titlebar-h, 28px) 1fr;
  grid-template-columns: auto 1fr;
  grid-template-areas:
    "titlebar titlebar"
    "sidebar main";
  height: 100vh;
  overflow: hidden;
}

.sidebar-column {
  grid-area: sidebar;
  display: flex;
  min-width: 0;
  overflow: hidden;
}

.sidebar-column :deep(#sidebar) {
  flex: 1;
  min-width: 0;
}

.sidebar-resizer {
  flex-shrink: 0;
  width: 5px;
  margin-right: -2px;
  cursor: col-resize;
  background: transparent;
  transition: background 0.15s;
  z-index: 2;
}

.sidebar-resizer:hover,
.sidebar-resizer:active {
  background: color-mix(in srgb, var(--accent) 55%, transparent);
}

#main {
  grid-area: main;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 0;
}
</style>

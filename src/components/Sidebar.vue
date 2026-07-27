<script setup lang="ts">
import { inject, ref, watch, computed, type Ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { storiesKey, reportsKey, platformKey, showPanelKey, seriesKey, campaignsKey } from '../injectionKeys';
import type { Story, Series, SeriesBook } from '../types';
import FileTreeNodes, { type FileTreeEntry } from './FileTreeNodes.vue';

// ── Injections ────────────────────────────────────────────────────────────────

const storiesCtx = inject(storiesKey)!;
const reportsCtx = inject(reportsKey)!;
const platformCtx = inject(platformKey)!;
const showPanel = inject(showPanelKey)!;
const seriesCtx = inject(seriesKey)!;
const campaignsCtx = inject(campaignsKey)!;

const appMode = inject<Ref<'analyzer' | 'writing' | 'marketing'>>('appMode')!;
const setAppMode = inject<(mode: 'analyzer' | 'writing' | 'marketing') => void>('setAppMode')!;
const openInWritingMode = inject<(filePath: string, title: string) => void>('openInWritingMode')!;
const openNewDocumentForm = inject<(location?: string) => void>('openNewDocumentForm')!;
const fileTreeTick = inject<Ref<number>>('fileTreeTick')!;
const writingBrowseFolder = inject<Ref<string>>('writingBrowseFolder')!;

// ── Emits ─────────────────────────────────────────────────────────────────────

const emit = defineEmits<{
  (e: 'open-story-form', story: Story | null): void;
  (e: 'open-series-form', series: Series | null): void;
  (e: 'open-campaign-form', id: number | null): void;
  (e: 'open-campaign-detail', id: number): void;
  (e: 'open-platform-accounts'): void;
}>();

// ── Sidebar mode toggle ───────────────────────────────────────────────────────

type SidebarMode = 'files' | 'reports';
const sidebarMode = ref<SidebarMode>('files');

type SidebarSection = 'stories' | 'workspace' | 'series' | 'campaigns' | 'platformAccounts' | 'tools';
const sectionOpen = ref<Record<SidebarSection, boolean>>({
  stories: true,
  workspace: true,
  series: true,
  campaigns: true,
  platformAccounts: true,
  tools: true,
});

function toggleSection(section: SidebarSection): void {
  sectionOpen.value[section] = !sectionOpen.value[section];
}

/** Story folders linked to any series — shown under Series, not in the flat Stories list (writing mode). */
const seriesBookFolders = computed(() => {
  const folders = new Set<string>();
  for (const s of seriesCtx.series.value) {
    for (const b of s.books) {
      folders.add(b.story_folder.replace(/[/\\]+$/, ''));
    }
  }
  return folders;
});

const standaloneStories = computed(() => {
  if (appMode.value !== 'writing') {
    return storiesCtx.stories.value;
  }
  return storiesCtx.stories.value.filter(s => {
    const folder = s.folder.replace(/[/\\]+$/, '');
    return !seriesBookFolders.value.has(folder);
  });
});

const expandedSeriesIds = ref<Set<number>>(new Set());

const effectiveBrowseFolder = computed(() => {
  if (appMode.value === 'writing' && writingBrowseFolder.value) {
    return writingBrowseFolder.value;
  }
  return storiesCtx.activeFolder.value;
});

function toggleSeriesExpand(seriesId: number): void {
  const next = new Set(expandedSeriesIds.value);
  if (next.has(seriesId)) next.delete(seriesId);
  else next.add(seriesId);
  expandedSeriesIds.value = next;
}

function findStoryByFolder(folder: string): Story | undefined {
  const norm = folder.replace(/[/\\]+$/, '');
  return storiesCtx.stories.value.find(s => s.folder.replace(/[/\\]+$/, '') === norm);
}

// ── File tree state ───────────────────────────────────────────────────────────

const fileTree = ref<FileTreeEntry[]>([]);
const expandedDirs = ref<Set<string>>(new Set());
const fileTreeError = ref('');

function relativeLocation(absolutePath: string): string {
  const root = effectiveBrowseFolder.value.replace(/[/\\]+$/, '');
  const full = absolutePath.replace(/\\/g, '/');
  const base = root.replace(/\\/g, '/');
  if (full === base) return '';
  if (full.startsWith(base + '/')) return full.slice(base.length + 1);
  return absolutePath;
}

async function loadFileTree(): Promise<void> {
  const folder = effectiveBrowseFolder.value;
  if (!folder) {
    fileTree.value = [];
    fileTreeError.value = '';
    return;
  }
  await loadFileTreeForFolder(folder);
}

function switchSidebarMode(mode: SidebarMode): void {
  sidebarMode.value = mode;
  sectionOpen.value.workspace = true;
  if (mode === 'files' && effectiveBrowseFolder.value) {
    void loadFileTree();
  }
}

function toggleDir(path: string): void {
  const s = new Set(expandedDirs.value);
  if (s.has(path)) s.delete(path); else s.add(path);
  expandedDirs.value = s;
}

function onFileClick(entry: FileTreeEntry): void {
  if (entry.is_dir) {
    toggleDir(entry.path);
    return;
  }
  openInWritingMode(entry.path, entry.name.replace(/\.md$/, ''));
}

function onAddInFolder(entry: FileTreeEntry): void {
  openNewDocumentForm(relativeLocation(entry.path));
}

function onAddDocument(): void {
  openNewDocumentForm();
}

watch(() => storiesCtx.activeFolder.value, (folder) => {
  if (folder) {
    writingBrowseFolder.value = '';
    loadFileTree();
  } else if (!writingBrowseFolder.value) {
    fileTree.value = [];
  }
}, { immediate: true });

watch(fileTreeTick, () => {
  if (effectiveBrowseFolder.value) loadFileTree();
});

watch(sidebarMode, (mode) => {
  if (mode === 'files' && effectiveBrowseFolder.value) loadFileTree();
});

watch(() => sectionOpen.value.workspace, (open) => {
  if (open && sidebarMode.value === 'files' && effectiveBrowseFolder.value) {
    void loadFileTree();
  }
});

watch(writingBrowseFolder, () => {
  if (writingBrowseFolder.value) void loadFileTree();
});

function onEditSeries(s: Series): void {
  emit('open-series-form', s);
}

watch(appMode, (mode) => {
  if (mode === 'writing' && effectiveBrowseFolder.value) {
    sidebarMode.value = 'files';
    loadFileTree();
  }
  if (mode === 'marketing' && storiesCtx.activeFolder.value) {
    showPanel('campaigns');
    void campaignsCtx.loadCampaigns(storiesCtx.activeFolder.value);
  }
});

// If stories load with a saved selection, keep Files visible
watch(() => storiesCtx.stories.value.length, (n) => {
  if (n > 0 && !storiesCtx.activeStoryId.value) {
    storiesCtx.setActiveStory(storiesCtx.stories.value[0].id);
  }
});

// ── Report expand/collapse state ──────────────────────────────────────────────

const expanded = ref<string | null>(null);

function toggleExpand(docType: string): void {
  expanded.value = expanded.value === docType ? null : docType;
}

// ── Handlers ──────────────────────────────────────────────────────────────────

function onStoryClick(story: Story): void {
  writingBrowseFolder.value = '';
  storiesCtx.setActiveStory(story.id);
  if (appMode.value === 'marketing') {
    showPanel('campaigns');
    void campaignsCtx.loadCampaigns(story.folder);
  } else if (appMode.value === 'writing') {
    void loadFileTree();
  } else {
    switchSidebarMode('files');
    showPanel('analyzer');
  }
}

function onSeriesBookClick(book: SeriesBook): void {
  const story = findStoryByFolder(book.story_folder);
  if (story) {
    onStoryClick(story);
    return;
  }
  writingBrowseFolder.value = book.story_folder;
  storiesCtx.setActiveStory(null);
  void loadFileTree();
}

function isSeriesBookActive(book: SeriesBook): boolean {
  const norm = book.story_folder.replace(/[/\\]+$/, '');
  const story = findStoryByFolder(book.story_folder);
  if (story) return story.id === storiesCtx.activeStoryId.value;
  return writingBrowseFolder.value.replace(/[/\\]+$/, '') === norm;
}

async function loadFileTreeForFolder(folder: string): Promise<void> {
  if (!folder) {
    fileTree.value = [];
    return;
  }
  fileTreeError.value = '';
  try {
    fileTree.value = await invoke<FileTreeEntry[]>('list_manuscript_files', { folder });
    const expand = new Set<string>();
    function walk(entries: FileTreeEntry[]) {
      for (const e of entries) {
        if (e.is_dir) { expand.add(e.path); walk(e.children); }
      }
    }
    walk(fileTree.value);
    expandedDirs.value = expand;
  } catch (e) {
    console.error('list_manuscript_files:', e);
    fileTree.value = [];
    fileTreeError.value = 'Could not load files for this book folder.';
  }
}

function onEditStory(story: Story): void {
  emit('open-story-form', story);
}

function onNewStory(): void {
  emit('open-story-form', null);
}

function onNewSeries(): void {
  emit('open-series-form', null);
}

function onNewCampaign(): void {
  emit('open-campaign-form', null);
}

function onCampaignClick(id: number): void {
  emit('open-campaign-detail', id);
}

function onPlatformAccounts(): void {
  emit('open-platform-accounts');
}

async function onVersionClick(id: number): Promise<void> {
  await reportsCtx.openReport(id);
  showPanel('reports');
}

async function onDeleteVersion(id: number, e: Event): Promise<void> {
  e.stopPropagation();
  if (!confirm('Delete this report? This cannot be undone.')) return;
  try {
    await reportsCtx.deleteReport(id);
    const folder = storiesCtx.activeFolder.value;
    if (folder) await reportsCtx.loadSidebarReports(folder, platformCtx.platform.value);
  } catch (err) {
    alert('Could not delete: ' + String(err));
  }
}

function formatTimestamp(ts: string): string {
  return new Date(ts).toLocaleString(undefined, {
    month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit',
  });
}
</script>

<template>
  <aside id="sidebar">
    <div class="nav-section mode-tabs">
      <button class="mode-tab" :class="{ active: appMode === 'analyzer' }" @click="setAppMode('analyzer'); showPanel('analyzer')">Analyzer</button>
      <button class="mode-tab" :class="{ active: appMode === 'writing' }" @click="setAppMode('writing'); sidebarMode = 'files'">Writing</button>
      <button class="mode-tab" :class="{ active: appMode === 'marketing' }" @click="setAppMode('marketing')">Marketing</button>
    </div>

    <section class="sidebar-block">
      <button class="section-header" @click="toggleSection('stories')">
        <span class="section-title">Stories</span>
        <span class="section-chevron" :class="{ open: sectionOpen.stories }">&#8250;</span>
      </button>
      <div v-show="sectionOpen.stories" class="section-content stories-section">
        <div class="nav-label-row">
          <span class="nav-label">Library</span>
          <button class="btn-new-story" title="New story" @click.stop="onNewStory">+</button>
        </div>
        <div class="stories-list">
          <div
            v-if="standaloneStories.length === 0"
            class="sidebar-hint"
          >
            <template v-if="appMode === 'writing' && seriesCtx.series.value.length > 0">
              No standalone stories. Open a book under Series below.
            </template>
            <template v-else>
              No stories yet. Click + to add one.
            </template>
          </div>
          <div
            v-for="story in standaloneStories"
            :key="story.id"
            class="story-item"
            :class="{ active: story.id === storiesCtx.activeStoryId.value }"
            :title="story.folder"
            @click="onStoryClick(story)"
          >
            <span class="story-item-name">{{ story.name }}</span>
            <button
              class="story-item-edit"
              :title="'Edit story'"
              @click.stop="onEditStory(story)"
            >&#x270E;</button>
          </div>
        </div>
      </div>
    </section>

    <section v-if="appMode !== 'marketing'" class="sidebar-block" :class="{ disabled: !effectiveBrowseFolder }">
      <button class="section-header" @click="toggleSection('workspace')">
        <span class="section-title">Workspace</span>
        <span class="section-chevron" :class="{ open: sectionOpen.workspace }">&#8250;</span>
      </button>

      <div v-show="sectionOpen.workspace" class="section-content workspace-section">
        <div v-if="!effectiveBrowseFolder" class="sidebar-hint files-hint">
          Select a story or series book to browse files.
        </div>

        <div v-if="appMode === 'analyzer'" class="mode-toggle-row">
          <div class="mode-toggle">
            <button
              class="mode-btn"
              :class="{ active: sidebarMode === 'files' }"
              @click="switchSidebarMode('files')"
            >Files</button>
            <button
              class="mode-btn"
              :class="{ active: sidebarMode === 'reports' }"
              @click="switchSidebarMode('reports')"
            >Reports</button>
          </div>
          <button
            v-if="sidebarMode === 'files' && storiesCtx.activeFolder.value"
            class="btn-new-story"
            title="New document"
            @click="onAddDocument"
          >+</button>
        </div>

        <div v-if="(sidebarMode === 'files' || appMode === 'writing') && effectiveBrowseFolder" class="files-section">
          <div v-if="appMode === 'writing'" class="nav-label-row files-header">
            <span class="nav-label">Files</span>
            <button class="btn-new-story" title="New document" @click="onAddDocument">+</button>
          </div>
          <div v-if="fileTreeError" class="sidebar-hint">{{ fileTreeError }}</div>
          <div v-else-if="fileTree.length === 0" class="sidebar-hint">No documents yet. Click + to create one.</div>
          <FileTreeNodes
            v-else
            :entries="fileTree"
            :expanded="expandedDirs"
            @toggle="toggleDir"
            @open="onFileClick"
            @add="onAddInFolder"
          />
        </div>

        <div v-if="sidebarMode === 'reports' && appMode === 'analyzer'" class="reports-section">
          <div v-if="!storiesCtx.activeFolder.value" class="sidebar-hint">
            Select a story to see reports.
          </div>
          <div v-else-if="reportsCtx.sidebarGroups.value.length === 0" class="sidebar-hint">
            No saved reports yet.
          </div>
          <template v-else>
            <div
              v-for="type in reportsCtx.sidebarGroups.value"
              :key="type.doc_type"
              class="report-type"
            >
              <div
                class="report-type-header"
                :title="type.description"
                @click="toggleExpand(type.doc_type)"
              >
                <span class="report-type-label">{{ type.label }}</span>
                <span class="report-count">{{ type.count }}</span>
              </div>

              <div
                v-if="expanded === type.doc_type && type.versions.length > 0"
                class="report-versions"
              >
                <div
                  v-for="version in type.versions"
                  :key="version.id"
                  class="report-version-item"
                  @click="onVersionClick(version.id)"
                >
                  <span class="version-label">{{ formatTimestamp(version.generated_at) }}</span>
                  <button class="version-delete" @click="onDeleteVersion(version.id, $event)" title="Delete this report">&times;</button>
                </div>
              </div>
            </div>
          </template>
        </div>
      </div>
    </section>

    <section v-if="appMode === 'marketing'" class="sidebar-block" :class="{ disabled: !storiesCtx.activeFolder.value }">
      <button class="section-header" @click="toggleSection('campaigns')">
        <span class="section-title">Campaigns</span>
        <span class="section-chevron" :class="{ open: sectionOpen.campaigns }">&#8250;</span>
      </button>
      <div v-show="sectionOpen.campaigns" class="section-content campaigns-section">
        <div class="nav-label-row">
          <span class="nav-label">For this story</span>
          <button class="btn-new-story" title="New campaign" :disabled="!storiesCtx.activeFolder.value" @click.stop="onNewCampaign">+</button>
        </div>
        <div v-if="!storiesCtx.activeFolder.value" class="sidebar-hint">Select a story first.</div>
        <div v-else-if="campaignsCtx.campaigns.value.length === 0" class="sidebar-hint">No campaigns yet.</div>
        <div
          v-for="c in campaignsCtx.campaigns.value"
          :key="c.id"
          class="story-item"
          @click="onCampaignClick(c.id)"
        >
          <span class="story-item-name">{{ c.name }}</span>
          <span class="series-book-count">{{ c.status }}</span>
        </div>
      </div>
    </section>

    <section v-if="appMode === 'marketing'" class="sidebar-block">
      <button class="section-header" @click="toggleSection('platformAccounts')">
        <span class="section-title">Platform Accounts</span>
        <span class="section-chevron" :class="{ open: sectionOpen.platformAccounts }">&#8250;</span>
      </button>
      <div v-show="sectionOpen.platformAccounts" class="section-content">
        <button class="nav-item" @click="onPlatformAccounts">Manage accounts</button>
      </div>
    </section>

    <section v-if="appMode === 'analyzer' || appMode === 'writing'" class="sidebar-block">
      <button class="section-header" @click="toggleSection('series')">
        <span class="section-title">Series</span>
        <span class="section-chevron" :class="{ open: sectionOpen.series }">&#8250;</span>
      </button>
      <div v-show="sectionOpen.series" class="section-content series-section">
        <div class="nav-label-row">
          <span class="nav-label">Collections</span>
          <button
            v-if="appMode === 'analyzer'"
            class="btn-new-story"
            title="New series"
            @click.stop="onNewSeries"
          >+</button>
        </div>
        <div class="series-list">
          <div
            v-if="seriesCtx.series.value.length === 0"
            class="sidebar-hint"
          >
            No series yet. Click + to add one.
          </div>

          <!-- Analyzer: tap series to edit -->
          <template v-if="appMode === 'analyzer'">
            <div
              v-for="s in seriesCtx.series.value"
              :key="s.id"
              class="story-item"
              @click="onEditSeries(s)"
            >
              <span class="story-item-name">{{ s.name }}</span>
              <span class="series-book-count">{{ s.books.length }}</span>
              <button
                class="story-item-edit"
                title="Edit series"
                @click.stop="onEditSeries(s)"
              >&#x270E;</button>
            </div>
          </template>

          <!-- Writing: expand series to browse books -->
          <template v-else>
            <div v-for="s in seriesCtx.series.value" :key="s.id" class="series-group">
              <div
                class="story-item series-header"
                :class="{ open: expandedSeriesIds.has(s.id) }"
                @click="toggleSeriesExpand(s.id)"
              >
                <span class="series-expand-chevron" :class="{ open: expandedSeriesIds.has(s.id) }">&#8250;</span>
                <span class="story-item-name">{{ s.name }}</span>
                <span class="series-book-count">{{ s.books.length }}</span>
              </div>
              <div v-if="expandedSeriesIds.has(s.id)" class="series-books">
                <div
                  v-if="s.books.length === 0"
                  class="sidebar-hint series-books-hint"
                >
                  No books in this series yet.
                </div>
                <div
                  v-for="book in s.books"
                  :key="book.story_folder"
                  class="story-item series-book-item"
                  :class="{ active: isSeriesBookActive(book) }"
                  :title="book.story_folder"
                  @click="onSeriesBookClick(book)"
                >
                  <span class="series-book-order">{{ book.book_order }}</span>
                  <span class="story-item-name">{{ book.story_name }}</span>
                </div>
              </div>
            </div>
          </template>
        </div>
      </div>
    </section>

    <section class="sidebar-block settings-section">
      <button class="section-header" @click="toggleSection('tools')">
        <span class="section-title">Utilities</span>
        <span class="section-chevron" :class="{ open: sectionOpen.tools }">&#8250;</span>
      </button>
      <div v-show="sectionOpen.tools" class="section-content nav-section">
        <button class="nav-item" @click="showPanel('help')">
          Help
        </button>
        <button class="nav-item" @click="showPanel('settings')">
          Settings
        </button>
      </div>
    </section>
  </aside>
</template>

<style scoped>
#sidebar {
  grid-area: sidebar;
  background: var(--surface);
  border-right: 1px solid var(--border);
  padding: 12px 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
}

.nav-section {
  padding: 0 12px 8px;
}

.sidebar-block {
  margin: 0 8px 8px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: color-mix(in srgb, var(--surface) 85%, black 15%);
  overflow: hidden;
}

.sidebar-block.disabled {
  opacity: 0.88;
}

.section-header {
  width: 100%;
  border: 0;
  background: color-mix(in srgb, var(--surface2) 65%, transparent 35%);
  color: var(--text);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 10px;
  cursor: pointer;
}

.section-header:hover {
  background: color-mix(in srgb, var(--surface2) 80%, transparent 20%);
}

.section-title {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--text-muted);
}

.section-chevron {
  font-size: 16px;
  color: var(--text-muted);
  transform: rotate(90deg);
  transition: transform 0.16s ease;
}

.section-chevron.open {
  transform: rotate(270deg);
}

.section-content {
  padding-bottom: 6px;
  padding-right: 0;
  overflow-y: auto;
  max-height: 28vh;
}

.section-content,
.stories-section,
.files-section,
.reports-section,
.series-section {
  scrollbar-width: thin;
  scrollbar-color: color-mix(in srgb, var(--text-muted) 55%, transparent 45%) transparent;
}

.section-content::-webkit-scrollbar,
.stories-section::-webkit-scrollbar,
.files-section::-webkit-scrollbar,
.reports-section::-webkit-scrollbar,
.series-section::-webkit-scrollbar {
  width: 6px;
}

.section-content::-webkit-scrollbar-track,
.stories-section::-webkit-scrollbar-track,
.files-section::-webkit-scrollbar-track,
.reports-section::-webkit-scrollbar-track,
.series-section::-webkit-scrollbar-track {
  background: transparent;
  margin: 4px 0;
}

.section-content::-webkit-scrollbar-thumb,
.stories-section::-webkit-scrollbar-thumb,
.files-section::-webkit-scrollbar-thumb,
.reports-section::-webkit-scrollbar-thumb,
.series-section::-webkit-scrollbar-thumb {
  background: color-mix(in srgb, var(--text-muted) 55%, transparent 45%);
  border-radius: 999px;
}

.section-content::-webkit-scrollbar-thumb:hover,
.stories-section::-webkit-scrollbar-thumb:hover,
.files-section::-webkit-scrollbar-thumb:hover,
.reports-section::-webkit-scrollbar-thumb:hover,
.series-section::-webkit-scrollbar-thumb:hover {
  background: color-mix(in srgb, var(--text-muted) 72%, transparent 28%);
}

.workspace-section {
  max-height: 44vh;
}

.series-section {
  max-height: 20vh;
}

.settings-section .section-content {
  max-height: 16vh;
}

.mode-tabs {
  display: flex;
  gap: 0;
  padding: 8px 10px;
}

.mode-tab {
  flex: 1;
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  color: var(--text-muted);
  font-size: 12px;
  font-weight: 600;
  padding: 6px 0;
  cursor: pointer;
  text-align: center;
  transition: color 0.15s, border-color 0.15s;
}

.mode-tab:hover {
  color: var(--text);
}

.mode-tab.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

.settings-section {
  margin-top: 0;
}

.nav-label {
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--text-muted);
  padding: 0 4px 0;
}

.nav-label-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 14px 4px;
}

.nav-item {
  display: block;
  width: 100%;
  background: none;
  border: none;
  border-radius: var(--radius);
  color: var(--text-muted);
  cursor: pointer;
  font-size: 13px;
  padding: 7px 10px;
  text-align: left;
  transition: background 0.15s, color 0.15s;
}

.nav-item:hover {
  background: var(--surface2);
  color: var(--text);
}

.btn-new-story {
  background: none;
  border: 1px solid var(--border);
  color: var(--text-muted);
  width: 20px;
  height: 20px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 16px;
  line-height: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  flex-shrink: 0;
}

.btn-new-story:hover {
  color: var(--accent);
  border-color: var(--accent);
}

.stories-section {
  padding: 0 0 4px;
  flex: 0 0 auto;
  min-height: 0;
}

.stories-list {
  padding: 0 8px 8px;
}

.files-section {
  flex: 1 1 auto;
  overflow-y: auto;
  padding: 0 0 0 8px;
  min-height: 120px;
}

.workspace-section {
  display: flex;
  flex-direction: column;
  min-height: 120px;
}

.files-header {
  padding: 4px 8px 6px;
}

.series-section {
  padding: 0 0 4px;
  flex: 0 0 auto;
}

.story-item {
  padding: 8px 10px;
  border-radius: var(--radius);
  cursor: pointer;
  font-size: 13px;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  position: relative;
  display: flex;
  align-items: center;
}

.story-item:hover {
  background: var(--surface2);
  color: var(--text);
}

.story-item.active {
  background: var(--surface2);
  color: var(--text);
  font-weight: 600;
  border-left: 2px solid var(--accent);
  padding-left: 8px;
}

.story-item-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
}

.story-item-edit {
  position: absolute;
  right: 6px;
  top: 50%;
  transform: translateY(-50%);
  font-size: 11px;
  color: var(--text-muted);
  opacity: 0;
  background: none;
  border: none;
  cursor: pointer;
  padding: 2px 4px;
}

.story-item:hover .story-item-edit {
  opacity: 1;
}

.mode-toggle-row {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 6px 10px 8px;
}

.mode-toggle {
  display: flex;
  flex: 1;
  min-width: 0;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  overflow: hidden;
}

.mode-btn {
  flex: 1;
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: 11px;
  font-weight: 600;
  padding: 5px 0;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}

.mode-btn:first-child {
  border-right: 1px solid var(--border);
}

.mode-btn.active {
  background: var(--accent);
  color: #fff;
}

.mode-btn:not(.active):hover {
  background: var(--surface2);
  color: var(--text);
}

.reports-section {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
  padding: 0 0 6px 8px;
}

.report-type {
  margin: 0;
}

.report-type-header {
  padding: 6px 10px;
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  cursor: pointer;
  border-radius: var(--radius);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.report-type-header:hover {
  background: var(--surface2);
}

.report-type-label {
  overflow: hidden;
  text-overflow: ellipsis;
  color: var(--text);
}

.report-count {
  background: var(--surface2);
  color: var(--text-muted);
  font-size: 10px;
  font-weight: 600;
  padding: 1px 6px;
  border-radius: 8px;
  min-width: 18px;
  text-align: center;
  flex-shrink: 0;
}

.report-versions {
  padding: 2px 0 4px 18px;
}

.report-version-item {
  display: flex;
  align-items: center;
  padding: 4px 10px;
  font-size: 11px;
  color: var(--text-muted);
  cursor: pointer;
  border-radius: var(--radius);
}

.report-version-item .version-label {
  flex: 1;
}

.report-version-item .version-delete {
  display: none;
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: 14px;
  line-height: 1;
  cursor: pointer;
  padding: 0 4px;
  border-radius: 3px;
}

.report-version-item:hover .version-delete {
  display: inline;
}

.report-version-item .version-delete:hover {
  color: #e74c3c;
  background: var(--surface2);
}

.report-version-item:hover {
  background: var(--surface2);
  color: var(--accent);
}

.sidebar-hint {
  padding: 8px 10px;
  font-size: 11px;
  color: var(--text-muted);
}

.files-hint {
  flex: 1;
}

.series-list {
  padding: 0 8px 8px;
}

.series-book-count {
  font-size: 10px;
  color: var(--text-muted);
  background: var(--surface2);
  padding: 1px 5px;
  border-radius: 6px;
  margin-left: auto;
}

.series-group {
  margin-bottom: 2px;
}

.series-header {
  cursor: pointer;
}

.series-header .story-item-name {
  flex: 1;
}

.series-expand-chevron {
  display: inline-block;
  font-size: 12px;
  color: var(--text-muted);
  transition: transform 0.15s ease;
  flex-shrink: 0;
  width: 12px;
}

.series-expand-chevron.open {
  transform: rotate(90deg);
}

.series-books {
  padding-left: 14px;
}

.series-books-hint {
  padding: 4px 10px 8px;
}

.series-book-item {
  padding-left: 8px;
  gap: 6px;
}

.series-book-order {
  font-size: 10px;
  font-weight: 600;
  color: var(--text-muted);
  min-width: 14px;
  text-align: center;
  flex-shrink: 0;
}
</style>

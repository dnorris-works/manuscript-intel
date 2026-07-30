<script setup lang="ts">
import { inject, ref, watch, computed, h, type Ref, type VNodeChild } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import {
  NButton, NButtonGroup, NCollapse, NCollapseItem, NMenu, NTree, NText, NTag,
  NEmpty, NScrollbar, useDialog, type MenuOption, type TreeOption,
} from 'naive-ui';
import { storiesKey, reportsKey, platformKey, showPanelKey, seriesKey, campaignsKey } from '../injectionKeys';
import type { Story, Series, SeriesBook } from '../types';
import FileTreeNodes, { type FileTreeEntry } from './FileTreeNodes.vue';

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
const dialog = useDialog();

const emit = defineEmits<{
  (e: 'open-story-form', story: Story | null): void;
  (e: 'open-series-form', series: Series | null): void;
  (e: 'open-campaign-form', id: number | null): void;
  (e: 'open-campaign-detail', id: number): void;
  (e: 'open-platform-accounts'): void;
}>();

type SidebarMode = 'files' | 'reports';
const sidebarMode = ref<SidebarMode>('files');

const expandedSections = ref<string[]>([
  'stories', 'workspace', 'series', 'campaigns', 'platformAccounts', 'tools',
]);

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
  if (appMode.value !== 'writing') return storiesCtx.stories.value;
  return storiesCtx.stories.value.filter(s => {
    const folder = s.folder.replace(/[/\\]+$/, '');
    return !seriesBookFolders.value.has(folder);
  });
});

const effectiveBrowseFolder = computed(() => {
  if (appMode.value === 'writing' && writingBrowseFolder.value) {
    return writingBrowseFolder.value;
  }
  return storiesCtx.activeFolder.value;
});

function findStoryByFolder(folder: string): Story | undefined {
  const norm = folder.replace(/[/\\]+$/, '');
  return storiesCtx.stories.value.find(s => s.folder.replace(/[/\\]+$/, '') === norm);
}

const fileTree = ref<FileTreeEntry[]>([]);
const expandedDirs = ref<string[]>([]);
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
  if (!expandedSections.value.includes('workspace')) {
    expandedSections.value = [...expandedSections.value, 'workspace'];
  }
  if (mode === 'files' && effectiveBrowseFolder.value) {
    void loadFileTree();
  }
}

function onFileClick(entry: FileTreeEntry): void {
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
    void loadFileTree();
  } else if (!writingBrowseFolder.value) {
    fileTree.value = [];
  }
}, { immediate: true });

watch(fileTreeTick, () => {
  if (effectiveBrowseFolder.value) void loadFileTree();
});

watch(sidebarMode, (mode) => {
  if (mode === 'files' && effectiveBrowseFolder.value) void loadFileTree();
});

watch(expandedSections, (names) => {
  if (names.includes('workspace') && sidebarMode.value === 'files' && effectiveBrowseFolder.value) {
    void loadFileTree();
  }
}, { deep: true });

watch(writingBrowseFolder, () => {
  if (writingBrowseFolder.value) void loadFileTree();
});

watch(appMode, (mode) => {
  if (mode === 'writing' && effectiveBrowseFolder.value) {
    sidebarMode.value = 'files';
    void loadFileTree();
  }
  if (mode === 'marketing' && storiesCtx.activeFolder.value) {
    showPanel('campaigns');
    void campaignsCtx.loadCampaigns(storiesCtx.activeFolder.value);
  }
});

watch(() => storiesCtx.stories.value.length, (n) => {
  if (n > 0 && !storiesCtx.activeStoryId.value) {
    storiesCtx.setActiveStory(storiesCtx.stories.value[0].id);
  }
});

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

async function loadFileTreeForFolder(folder: string): Promise<void> {
  if (!folder) {
    fileTree.value = [];
    return;
  }
  fileTreeError.value = '';
  try {
    fileTree.value = await invoke<FileTreeEntry[]>('list_manuscript_files', { folder });
    const expand: string[] = [];
    function walk(entries: FileTreeEntry[]): void {
      for (const e of entries) {
        if (e.is_dir) {
          expand.push(e.path);
          walk(e.children);
        }
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

function onEditSeries(s: Series): void {
  emit('open-series-form', s);
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

function onDeleteVersion(id: number): void {
  dialog.warning({
    title: 'Delete report',
    content: 'Delete this report? This cannot be undone.',
    positiveText: 'Delete',
    negativeText: 'Cancel',
    onPositiveClick: async () => {
      try {
        await reportsCtx.deleteReport(id);
        const folder = storiesCtx.activeFolder.value;
        if (folder) await reportsCtx.loadSidebarReports(folder, platformCtx.platform.value);
      } catch (err) {
        console.error(err);
      }
    },
  });
}

function formatTimestamp(ts: string): string {
  return new Date(ts).toLocaleString(undefined, {
    month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit',
  });
}

function renderStoryLabel(option: MenuOption): VNodeChild {
  const story = standaloneStories.value.find(s => s.id === option.key);
  return h('div', { style: 'display:flex;align-items:center;gap:4px;width:100%;' }, [
    h('span', { style: 'flex:1;overflow:hidden;text-overflow:ellipsis;' }, option.label as string),
    story
      ? h(NButton, {
        size: 'tiny',
        quaternary: true,
        onClick: (e: MouseEvent) => {
          e.stopPropagation();
          onEditStory(story);
        },
      }, { default: () => '✎' })
      : null,
  ]);
}

function renderSeriesLabel(option: MenuOption): VNodeChild {
  const series = seriesCtx.series.value.find(s => String(s.id) === option.key);
  return h('div', { style: 'display:flex;align-items:center;gap:6px;width:100%;' }, [
    h('span', { style: 'flex:1;overflow:hidden;text-overflow:ellipsis;' }, option.label as string),
    series
      ? h(NTag, { size: 'small', round: true }, { default: () => String(series.books.length) })
      : null,
    series
      ? h(NButton, {
        size: 'tiny',
        quaternary: true,
        onClick: (e: MouseEvent) => {
          e.stopPropagation();
          onEditSeries(series);
        },
      }, { default: () => '✎' })
      : null,
  ]);
}

function renderReportLabel(option: MenuOption): VNodeChild {
  if (String(option.key).startsWith('report-')) {
    const id = Number(String(option.key).replace('report-', ''));
    return h('div', { style: 'display:flex;align-items:center;gap:4px;width:100%;' }, [
      h('span', { style: 'flex:1;' }, option.label as string),
      h(NButton, {
        size: 'tiny',
        quaternary: true,
        type: 'error',
        onClick: (e: MouseEvent) => {
          e.stopPropagation();
          onDeleteVersion(id);
        },
      }, { default: () => '×' }),
    ]);
  }
  const group = reportsCtx.sidebarGroups.value.find(g => g.doc_type === option.key);
  return h('div', { style: 'display:flex;align-items:center;gap:6px;width:100%;' }, [
    h('span', { style: 'flex:1;overflow:hidden;text-overflow:ellipsis;' }, option.label as string),
    group ? h(NTag, { size: 'small', round: true }, { default: () => String(group.count) }) : null,
  ]);
}

const storyMenuOptions = computed<MenuOption[]>(() =>
  standaloneStories.value.map(story => ({
    key: story.id,
    label: story.name,
  })),
);

const seriesMenuOptions = computed<MenuOption[]>(() =>
  seriesCtx.series.value.map(s => ({
    key: String(s.id),
    label: s.name,
  })),
);

const seriesTreeData = computed<TreeOption[]>(() =>
  seriesCtx.series.value.map(s => ({
    key: `series-${s.id}`,
    label: s.name,
    children: s.books.map(book => ({
      key: book.story_folder,
      label: `${book.book_order}. ${book.story_name}`,
      isLeaf: true,
    })),
  })),
);

const campaignMenuOptions = computed<MenuOption[]>(() =>
  campaignsCtx.campaigns.value.map(c => ({
    key: String(c.id),
    label: c.name,
    extra: () => h(NTag, { size: 'small', round: true }, { default: () => c.status }),
  })),
);

const reportMenuOptions = computed<MenuOption[]>(() =>
  reportsCtx.sidebarGroups.value.map(type => ({
    key: type.doc_type,
    label: type.label,
    children: type.versions.map(version => ({
      key: `report-${version.id}`,
      label: formatTimestamp(version.generated_at),
    })),
  })),
);

function onStoryMenuSelect(key: string): void {
  const story = standaloneStories.value.find(s => s.id === key);
  if (story) onStoryClick(story);
}

function onSeriesMenuSelect(key: string): void {
  const series = seriesCtx.series.value.find(s => String(s.id) === key);
  if (series) onEditSeries(series);
}

function onCampaignMenuSelect(key: string): void {
  onCampaignClick(Number(key));
}

function onReportMenuSelect(key: string): void {
  if (key.startsWith('report-')) {
    void onVersionClick(Number(key.replace('report-', '')));
  }
}

function onSeriesTreeSelect(keys: Array<string | number>): void {
  const key = keys[0] as string | undefined;
  if (!key || key.startsWith('series-')) return;
  const book = seriesCtx.series.value
    .flatMap(s => s.books)
    .find(b => b.story_folder === key);
  if (book) onSeriesBookClick(book);
}

const seriesTreeSelectedKeys = computed(() => {
  for (const s of seriesCtx.series.value) {
    for (const book of s.books) {
      const story = findStoryByFolder(book.story_folder);
      if (story && story.id === storiesCtx.activeStoryId.value) {
        return [book.story_folder];
      }
      const norm = book.story_folder.replace(/[/\\]+$/, '');
      if (writingBrowseFolder.value.replace(/[/\\]+$/, '') === norm) {
        return [book.story_folder];
      }
    }
  }
  return [];
});

const emptyStoriesHint = computed(() => {
  if (appMode.value === 'writing' && seriesCtx.series.value.length > 0) {
    return 'No standalone stories. Open a book under Series below.';
  }
  return 'No stories yet. Click + to add one.';
});
</script>

<template>
  <aside id="sidebar">
    <n-scrollbar style="height: 100%;">
      <div class="sidebar-inner">
        <n-button-group class="mode-tabs">
          <n-button
            :type="appMode === 'analyzer' ? 'primary' : 'default'"
            @click="setAppMode('analyzer'); showPanel('analyzer')"
          >Analyzer</n-button>
          <n-button
            :type="appMode === 'writing' ? 'primary' : 'default'"
            @click="setAppMode('writing'); sidebarMode = 'files'"
          >Writing</n-button>
          <n-button
            :type="appMode === 'marketing' ? 'primary' : 'default'"
            @click="setAppMode('marketing')"
          >Marketing</n-button>
        </n-button-group>

        <n-collapse v-model:expanded-names="expandedSections" display-directive="show">
          <n-collapse-item name="stories" title="Stories">
            <template #header-extra>
              <n-button size="tiny" quaternary @click.stop="onNewStory">+</n-button>
            </template>
            <n-text depth="3" class="sub-label">Library</n-text>
            <n-empty v-if="standaloneStories.length === 0" :description="emptyStoriesHint" size="small" />
            <n-menu
              v-else
              :value="storiesCtx.activeStoryId.value"
              :options="storyMenuOptions"
              :render-label="renderStoryLabel"
              @update:value="onStoryMenuSelect"
            />
          </n-collapse-item>

          <n-collapse-item
            v-if="appMode !== 'marketing'"
            name="workspace"
            title="Workspace"
            :disabled="!effectiveBrowseFolder && appMode === 'analyzer'"
          >
            <n-text v-if="!effectiveBrowseFolder" depth="3" class="hint">
              Select a story or series book to browse files.
            </n-text>

            <template v-else>
              <div v-if="appMode === 'analyzer'" class="workspace-toolbar">
                <n-button-group style="flex: 1;">
                  <n-button
                    :type="sidebarMode === 'files' ? 'primary' : 'default'"
                    style="flex: 1;"
                    @click="switchSidebarMode('files')"
                  >Files</n-button>
                  <n-button
                    :type="sidebarMode === 'reports' ? 'primary' : 'default'"
                    style="flex: 1;"
                    @click="switchSidebarMode('reports')"
                  >Reports</n-button>
                </n-button-group>
                <n-button
                  v-if="sidebarMode === 'files' && storiesCtx.activeFolder.value"
                  size="tiny"
                  quaternary
                  title="New document"
                  @click="onAddDocument"
                >+</n-button>
              </div>

              <div v-if="sidebarMode === 'files' || appMode === 'writing'" class="workspace-panel">
                <div v-if="appMode === 'writing'" class="workspace-toolbar">
                  <n-text depth="3" class="sub-label">Files</n-text>
                  <n-button size="tiny" quaternary title="New document" @click="onAddDocument">+</n-button>
                </div>
                <n-text v-if="fileTreeError" depth="3" class="hint">{{ fileTreeError }}</n-text>
                <n-empty
                  v-else-if="fileTree.length === 0"
                  description="No documents yet. Click + to create one."
                  size="small"
                />
                <FileTreeNodes
                  v-else
                  :entries="fileTree"
                  :expanded-keys="expandedDirs"
                  @update:expanded-keys="expandedDirs = $event"
                  @open="onFileClick"
                  @add="onAddInFolder"
                />
              </div>

              <div v-if="sidebarMode === 'reports' && appMode === 'analyzer'" class="workspace-panel">
                <n-empty
                  v-if="!storiesCtx.activeFolder.value"
                  description="Select a story to see reports."
                  size="small"
                />
                <n-empty
                  v-else-if="reportsCtx.sidebarGroups.value.length === 0"
                  description="No saved reports yet."
                  size="small"
                />
                <n-menu
                  v-else
                  :options="reportMenuOptions"
                  :render-label="renderReportLabel"
                  @update:value="onReportMenuSelect"
                />
              </div>
            </template>
          </n-collapse-item>

          <n-collapse-item v-if="appMode === 'marketing'" name="campaigns" title="Campaigns">
            <template #header-extra>
              <n-button
                size="tiny"
                quaternary
                title="New campaign"
                :disabled="!storiesCtx.activeFolder.value"
                @click.stop="onNewCampaign"
              >+</n-button>
            </template>
            <n-text depth="3" class="sub-label">For this story</n-text>
            <n-empty v-if="!storiesCtx.activeFolder.value" description="Select a story first." size="small" />
            <n-empty
              v-else-if="campaignsCtx.campaigns.value.length === 0"
              description="No campaigns yet."
              size="small"
            />
            <n-menu
              v-else
              :options="campaignMenuOptions"
              @update:value="onCampaignMenuSelect"
            />
          </n-collapse-item>

          <n-collapse-item v-if="appMode === 'marketing'" name="platformAccounts" title="Platform Accounts">
            <n-button block quaternary @click="onPlatformAccounts">Manage accounts</n-button>
          </n-collapse-item>

          <n-collapse-item
            v-if="appMode === 'analyzer' || appMode === 'writing'"
            name="series"
            title="Series"
          >
            <template v-if="appMode === 'analyzer'" #header-extra>
              <n-button size="tiny" quaternary title="New series" @click.stop="onNewSeries">+</n-button>
            </template>
            <n-text depth="3" class="sub-label">Collections</n-text>
            <n-empty
              v-if="seriesCtx.series.value.length === 0"
              description="No series yet. Click + to add one."
              size="small"
            />
            <n-menu
              v-else-if="appMode === 'analyzer'"
              :options="seriesMenuOptions"
              :render-label="renderSeriesLabel"
              @update:value="onSeriesMenuSelect"
            />
            <n-tree
              v-else
              block-line
              selectable
              :data="seriesTreeData"
              :selected-keys="seriesTreeSelectedKeys"
              @update:selected-keys="onSeriesTreeSelect"
            />
          </n-collapse-item>

          <n-collapse-item name="tools" title="Utilities">
            <n-button block quaternary @click="showPanel('help')">Help</n-button>
            <n-button block quaternary @click="showPanel('settings')">Settings</n-button>
          </n-collapse-item>
        </n-collapse>
      </div>
    </n-scrollbar>
  </aside>
</template>

<style scoped>
#sidebar {
  grid-area: sidebar;
  background: var(--surface);
  border-right: 1px solid var(--border);
  overflow: hidden;
  height: 100%;
}

.sidebar-inner {
  padding: 12px 10px 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.mode-tabs {
  width: 100%;
}

.mode-tabs :deep(.n-button) {
  flex: 1;
}

.sub-label {
  display: block;
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  margin: 0 0 6px 4px;
}

.hint {
  display: block;
  font-size: 11px;
  padding: 4px;
}

.workspace-toolbar {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 8px;
}

.workspace-panel {
  max-height: 36vh;
  overflow-y: auto;
}

#sidebar :deep(.n-collapse-item__header) {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

#sidebar :deep(.n-menu-item-content) {
  font-size: 13px;
}

#sidebar :deep(.n-tree-node-content__text) {
  font-size: 12px;
}
</style>

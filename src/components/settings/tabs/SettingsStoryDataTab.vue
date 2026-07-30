<script setup lang="ts">
import { computed, inject, ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { DataTableColumns } from 'naive-ui';
import {
  NButton, NSpace, NAlert, NSpin, NEmpty, NDataTable, NCode, NText, useDialog, useMessage,
} from 'naive-ui';
import { storiesKey, analysisKey, settingsKey } from '../../../injectionKeys';
import type { StoryArtifactStateResponse } from '../../../types';

const props = defineProps<{
  active?: boolean;
}>();

const storiesCtx = inject(storiesKey)!;
const analysisCtx = inject(analysisKey)!;
const settingsCtx = inject(settingsKey)!;
const dialog = useDialog();
const message = useMessage();

const state = ref<StoryArtifactStateResponse | null>(null);
const loading = ref(false);
const error = ref('');
const refreshing = ref(false);

const activeFolder = computed(() => storiesCtx.activeFolder.value);
const activeStoryName = computed(() => storiesCtx.activeStory.value?.name ?? '');

const chapterColumns: DataTableColumns<StoryArtifactStateResponse['chapters'][number]> = [
  { title: 'File', key: 'file', ellipsis: { tooltip: true } },
  { title: 'Title', key: 'title', ellipsis: { tooltip: true } },
  { title: 'Words', key: 'word_count', width: 72 },
  { title: 'Updated', key: 'updated_at', width: 160, ellipsis: { tooltip: true } },
  { title: 'Summary preview', key: 'summary_preview', ellipsis: { tooltip: true } },
];

async function loadState(): Promise<void> {
  const folder = activeFolder.value;
  if (!folder) {
    state.value = null;
    return;
  }
  loading.value = true;
  error.value = '';
  try {
    state.value = await invoke<StoryArtifactStateResponse>('get_story_artifact_state', { folder });
  } catch (e) {
    error.value = String(e);
    state.value = null;
  } finally {
    loading.value = false;
  }
}

async function onRefreshSummaries(): Promise<void> {
  const folder = activeFolder.value;
  if (!folder) return;
  const setupIssues = settingsCtx.checkPublishAnalyzeSetup();
  if (setupIssues.length > 0) {
    message.warning(setupIssues.map(i => i.message).join(' '));
    return;
  }
  refreshing.value = true;
  try {
    const msg = await invoke<string>('refresh_chapter_fingerprints', {
      request: {
        folder,
        provider: settingsCtx.provider.value,
        api_key: settingsCtx.apiKey.value,
        model: settingsCtx.model.value,
        summaries_model: settingsCtx.modelFor('summaries'),
      },
    });
    message.success(msg || 'Chapter summaries refreshed.');
    await loadState();
    await analysisCtx.refreshState(folder);
  } catch (e) {
    message.error('Refresh failed: ' + String(e));
  } finally {
    refreshing.value = false;
  }
}

function onClearSummaries(): void {
  const folder = activeFolder.value;
  if (!folder) return;
  dialog.warning({
    title: 'Clear chapter summaries',
    content: 'Remove all stored chapter summaries for this story? Genre and keyword reports will need re-summarizing. Saved reports are not deleted.',
    positiveText: 'Clear',
    negativeText: 'Cancel',
    onPositiveClick: async () => {
      try {
        await invoke<void>('clear_chapter_fingerprints', { folder });
        message.success('Chapter summaries cleared.');
        await loadState();
        await analysisCtx.refreshState(folder);
      } catch (e) {
        message.error('Clear failed: ' + String(e));
      }
    },
  });
}

watch([() => props.active, activeFolder], ([isActive]) => {
  if (isActive) {
    void loadState();
  }
}, { immediate: true });
</script>

<template>
  <n-space vertical :size="16">
    <n-space justify="space-between" align="center">
      <div>
        <n-text strong>{{ activeStoryName || 'No story selected' }}</n-text>
        <n-text v-if="state" depth="3" style="display: block; font-size: 12px; margin-top: 4px;">
          {{ state.chapter_count }} chapter summar{{ state.chapter_count === 1 ? 'y' : 'ies' }}
          <span v-if="state.fingerprint_updated_at"> · updated {{ new Date(state.fingerprint_updated_at).toLocaleString() }}</span>
        </n-text>
      </div>
      <n-space>
        <n-button :disabled="!activeFolder" :loading="refreshing" @click="onRefreshSummaries">
          Refresh summaries
        </n-button>
        <n-button :disabled="!activeFolder" type="error" secondary @click="onClearSummaries">
          Clear
        </n-button>
        <n-button :loading="loading" :disabled="!activeFolder" @click="loadState">Reload</n-button>
      </n-space>
    </n-space>

    <n-alert v-if="error" type="error">{{ error }}</n-alert>

    <n-alert type="info" :bordered="false">
      Chapter summaries are AI-extracted genre signals (up to 2000 words per chapter). They power genre analysis, ranking, and keyword reports.
    </n-alert>

    <div v-if="state">
      <n-text depth="3" style="font-size: 12px; display: block; margin-bottom: 4px;">Manuscript fingerprint</n-text>
      <n-code :code="state.manuscript_fingerprint" word-wrap style="font-size: 11px;" />
    </div>

    <n-spin :show="loading && !state">
      <n-empty v-if="!activeFolder" description="Select a story to view chapter summaries." />
      <n-data-table
        v-else-if="state && state.chapters.length > 0"
        size="small"
        :columns="chapterColumns"
        :data="state.chapters"
        :bordered="false"
        :row-key="(row) => row.file"
        max-height="360"
      />
      <n-empty v-else-if="state" description="No chapter summaries stored yet." />
    </n-spin>

    <div v-if="state && state.artifacts.length > 0">
      <n-text depth="3" style="font-size: 12px; display: block; margin-bottom: 6px;">Artifact status</n-text>
      <n-space size="small">
        <n-text
          v-for="[name, status] in state.artifacts"
          :key="name"
          style="font-size: 12px;"
        >
          {{ name }}: <strong>{{ status }}</strong>
        </n-text>
      </n-space>
    </div>
  </n-space>
</template>

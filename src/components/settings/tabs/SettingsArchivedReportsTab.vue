<script setup lang="ts">
import { computed, h, inject, watch } from 'vue';
import type { DataTableColumns } from 'naive-ui';
import {
  NButton, NSpace, NAlert, NSpin, NEmpty, NDataTable, useDialog, useMessage,
} from 'naive-ui';
import { useArchivedReports } from '../../../composables/useArchivedReports';
import { storiesKey, reportsKey, showPanelKey } from '../../../injectionKeys';
import type { ArchivedReportRow } from '../../../types';

const props = defineProps<{
  active?: boolean;
}>();

const storiesCtx = inject(storiesKey)!;
const reportsCtx = inject(reportsKey)!;
const showPanel = inject(showPanelKey)!;
const dialog = useDialog();
const message = useMessage();

const {
  archivedRows,
  loading,
  error,
  loadArchivedReports,
  deleteArchivedReport,
} = useArchivedReports();

const activeFolder = computed(() => storiesCtx.activeFolder.value);
const activeStoryName = computed(() => storiesCtx.activeStory.value?.name ?? '');

function formatTs(ts: string): string {
  if (!ts) return '—';
  const d = new Date(ts);
  if (Number.isNaN(d.getTime())) return ts;
  return d.toLocaleString(undefined, {
    month: 'short', day: 'numeric', year: 'numeric',
    hour: 'numeric', minute: '2-digit',
  });
}

function reportTitle(row: ArchivedReportRow): string {
  return `${row.label} — ${formatTs(row.generated_at)}`;
}

async function onRead(row: ArchivedReportRow): Promise<void> {
  try {
    await reportsCtx.openReport(row.id);
    showPanel('reports');
  } catch (e) {
    message.error('Could not open report: ' + String(e));
  }
}

function onDelete(row: ArchivedReportRow): void {
  dialog.warning({
    title: 'Delete archived report',
    content: `Delete "${reportTitle(row)}"? This cannot be undone.`,
    positiveText: 'Delete',
    negativeText: 'Cancel',
    onPositiveClick: async () => {
      try {
        await deleteArchivedReport(row.id);
        if (activeFolder.value) {
          await loadArchivedReports(activeFolder.value);
        }
        message.success('Report deleted.');
      } catch (e) {
        message.error('Could not delete: ' + String(e));
      }
    },
  });
}

const columns = computed<DataTableColumns<ArchivedReportRow>>(() => [
  {
    title: 'Title',
    key: 'title',
    ellipsis: { tooltip: true },
    render: (row) => reportTitle(row),
  },
  {
    title: 'Archived',
    key: 'archived_at',
    width: 180,
    render: (row) => formatTs(row.archived_at),
  },
  {
    title: 'Reason',
    key: 'archive_reason',
    width: 160,
    ellipsis: { tooltip: true },
    render: (row) => row.archive_reason || '—',
  },
  {
    title: 'Actions',
    key: 'actions',
    width: 150,
    render: (row) => h(NSpace, { size: 8 }, {
      default: () => [
        h(NButton, { size: 'tiny', onClick: () => void onRead(row) }, { default: () => 'Read' }),
        h(NButton, { size: 'tiny', type: 'error', secondary: true, onClick: () => onDelete(row) }, { default: () => 'Delete' }),
      ],
    }),
  },
]);

watch([() => props.active, activeFolder], ([isActive, folder]) => {
  if (isActive && folder) {
    void loadArchivedReports(folder);
  }
}, { immediate: true });
</script>

<template>
  <n-space vertical :size="16">
    <n-space justify="space-between" align="center">
      <div>
        <strong v-if="activeStoryName">{{ activeStoryName }}</strong>
        <span v-else>No story selected</span>
      </div>
      <n-button
        :loading="loading"
        :disabled="!activeFolder"
        @click="activeFolder && loadArchivedReports(activeFolder)"
      >
        Refresh
      </n-button>
    </n-space>

    <n-alert v-if="error" type="error">{{ error }}</n-alert>

    <n-spin :show="loading && archivedRows.length === 0">
      <n-empty v-if="!activeFolder" description="Select a story to view archived reports." />
      <n-empty v-else-if="!loading && archivedRows.length === 0" description="No archived reports for this story." />
      <n-data-table
        v-else
        size="small"
        :columns="columns"
        :data="archivedRows"
        :bordered="false"
        :row-key="(row: ArchivedReportRow) => row.id"
      />
    </n-spin>
  </n-space>
</template>

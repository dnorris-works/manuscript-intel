<script setup lang="ts">
import { watch } from 'vue';
import {
  NButton, NText, NSpace, NGrid, NGi, NDataTable, NAlert, NSpin,
} from 'naive-ui';
import { useDatabaseInspector, formatBytes } from '../../../composables/useDatabaseInspector';

const props = defineProps<{
  active?: boolean;
}>();

const {
  dbOverview,
  dbLoading,
  dbError,
  selectedDbTable,
  dbPreview,
  dbPreviewLoading,
  dbPageSize,
  selectedTableInfo,
  dbPageEnd,
  schemaColumns,
  schemaRows,
  previewColumns,
  previewRows,
  loadDbOverview,
  selectDbTable,
  dbPrevPage,
  dbNextPage,
  onDatabaseTabActivated,
} = useDatabaseInspector();

watch(() => props.active, (isActive) => {
  if (isActive) {
    onDatabaseTabActivated();
  }
}, { immediate: true });
</script>

<template>
  <n-space vertical :size="12" class="db-tab">
    <n-space justify="space-between" align="start" style="width: 100%;">
      <n-space v-if="dbOverview" vertical :size="4">
        <n-text>
          <n-text depth="3">File</n-text>
          <code class="db-path">{{ dbOverview.path }}</code>
        </n-text>
        <n-text>
          <n-text depth="3">Size</n-text>
          {{ formatBytes(dbOverview.file_size_bytes) }}
        </n-text>
        <n-text>
          <n-text depth="3">Tables</n-text>
          {{ dbOverview.tables.length }}
        </n-text>
      </n-space>
      <n-button size="small" :loading="dbLoading" @click="loadDbOverview">
        Refresh
      </n-button>
    </n-space>

    <n-alert v-if="dbError" type="error" :bordered="false">
      {{ dbError }}
    </n-alert>

    <n-spin v-else-if="dbLoading && !dbOverview" size="small">
      <n-text depth="3">Loading database…</n-text>
    </n-spin>

    <n-grid v-else-if="dbOverview" :cols="24" :x-gap="16" class="db-layout">
      <n-gi :span="6">
        <div class="db-table-list">
          <div class="db-table-list-header">Tables</div>
          <button
            v-for="table in dbOverview.tables"
            :key="table.name"
            type="button"
            class="db-table-item"
            :class="{ active: selectedDbTable === table.name }"
            @click="selectDbTable(table.name)"
          >
            <span class="db-table-name">{{ table.name }}</span>
            <span class="db-table-count">{{ table.row_count.toLocaleString() }}</span>
          </button>
        </div>
      </n-gi>

      <n-gi :span="18">
        <template v-if="selectedTableInfo">
          <h3 class="db-detail-title">{{ selectedTableInfo.name }}</h3>
          <n-text depth="3" style="font-size: 13px; display: block; margin-bottom: 12px;">
            {{ selectedTableInfo.row_count.toLocaleString() }} rows ·
            {{ selectedTableInfo.columns.length }} columns
          </n-text>

          <n-text strong style="font-size: 12px; display: block; margin-bottom: 8px;">
            Structure
          </n-text>
          <n-data-table
            size="small"
            :columns="schemaColumns"
            :data="schemaRows"
            :bordered="true"
            :single-line="false"
            style="margin-bottom: 16px;"
          />

          <n-space justify="space-between" align="center" style="margin-bottom: 8px;">
            <n-text strong style="font-size: 12px;">Data preview</n-text>
            <n-text v-if="dbPreview" depth="3" style="font-size: 12px;">
              {{ dbPreview.total_rows === 0 ? '0 rows' : `${dbPreview.offset + 1}–${dbPageEnd} of ${dbPreview.total_rows.toLocaleString()}` }}
            </n-text>
          </n-space>

          <n-spin :show="dbPreviewLoading">
            <n-text v-if="dbPreview && dbPreview.rows.length === 0" depth="3">
              No rows.
            </n-text>
            <n-data-table
              v-else-if="dbPreview"
              size="small"
              :columns="previewColumns"
              :data="previewRows"
              :bordered="true"
              :single-line="false"
              :scroll-x="Math.max(600, previewColumns.length * 140)"
              max-height="42vh"
              :row-key="(row: Record<string, string>) => row._rowKey"
            />
          </n-spin>

          <n-space v-if="dbPreview && dbPreview.total_rows > dbPageSize" style="margin-top: 10px;">
            <n-button
              size="small"
              :disabled="dbPreview.offset <= 0 || dbPreviewLoading"
              @click="dbPrevPage"
            >Previous</n-button>
            <n-button
              size="small"
              :disabled="dbPageEnd >= dbPreview.total_rows || dbPreviewLoading"
              @click="dbNextPage"
            >Next</n-button>
          </n-space>
        </template>
        <n-text v-else depth="3">Select a table to inspect its structure and data.</n-text>
      </n-gi>
    </n-grid>
  </n-space>
</template>

<style scoped>
.db-tab {
  width: 100%;
}

.db-path {
  font-size: 12px;
  word-break: break-all;
  margin-left: 8px;
}

.db-layout {
  min-height: 360px;
}

.db-table-list {
  border: 1px solid var(--border);
  border-radius: var(--radius);
  overflow: auto;
  max-height: 70vh;
}

.db-table-list-header {
  padding: 8px 10px;
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-muted);
  border-bottom: 1px solid var(--border);
  background: var(--surface2);
}

.db-table-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 10px;
  border: none;
  border-bottom: 1px solid var(--border);
  background: transparent;
  color: var(--text);
  cursor: pointer;
  text-align: left;
  font-size: 12px;
}

.db-table-item:hover {
  background: var(--surface2);
}

.db-table-item.active {
  background: rgba(232, 97, 44, 0.1);
}

.db-table-name {
  font-family: ui-monospace, monospace;
  overflow: hidden;
  text-overflow: ellipsis;
}

.db-table-count {
  color: var(--text-muted);
  flex-shrink: 0;
}

.db-detail-title {
  font-size: 15px;
  margin: 0 0 4px;
}
</style>

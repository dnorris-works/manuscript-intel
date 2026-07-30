<script setup lang="ts">
import { watch } from 'vue';
import type { MenuOption } from 'naive-ui';
import {
  NLayout, NLayoutSider, NLayoutContent, NMenu, NDescriptions, NDescriptionsItem,
  NButton, NSpace, NCard, NDataTable, NAlert, NSpin, NEmpty, NPagination, NCode,
} from 'naive-ui';
import { useDatabaseInspector } from '../../../composables/useDatabaseInspector';

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
  selectedTableInfo,
  schemaColumns,
  schemaRows,
  previewColumns,
  previewRows,
  tableMenuOptions,
  dbMetaItems,
  dbPage,
  dbPageCount,
  loadDbOverview,
  selectDbTable,
  onDatabaseTabActivated,
} = useDatabaseInspector();

watch(() => props.active, (isActive) => {
  if (isActive) {
    onDatabaseTabActivated();
  }
}, { immediate: true });

function onMenuUpdate(key: string): void {
  selectDbTable(key);
}
</script>

<template>
  <n-space vertical :size="16">
    <n-space justify="space-between" align="center">
      <n-descriptions v-if="dbOverview" :column="3" size="small" label-placement="top">
        <n-descriptions-item
          v-for="item in dbMetaItems"
          :key="item.label"
          :label="item.label"
        >
          <n-code v-if="item.label === 'File'" :code="item.value" />
          <template v-else>{{ item.value }}</template>
        </n-descriptions-item>
      </n-descriptions>
      <n-button :loading="dbLoading" @click="loadDbOverview">Refresh</n-button>
    </n-space>

    <n-alert v-if="dbError" type="error">{{ dbError }}</n-alert>

    <n-spin v-else-if="dbLoading && !dbOverview" size="medium" />

    <n-layout v-else-if="dbOverview" has-sider style="min-height: 420px;">
      <n-layout-sider bordered :width="260" content-style="padding: 0;">
        <n-menu
          :value="selectedDbTable ?? undefined"
          :options="(tableMenuOptions as MenuOption[])"
          @update:value="onMenuUpdate"
        />
      </n-layout-sider>

      <n-layout-content content-style="padding: 16px;">
        <n-card v-if="selectedTableInfo" size="small">
          <template #header>
            <n-space justify="space-between" align="center">
              <span>{{ selectedTableInfo.name }}</span>
              <span>{{ selectedTableInfo.row_count.toLocaleString() }} rows · {{ selectedTableInfo.columns.length }} columns</span>
            </n-space>
          </template>

          <n-card title="Structure" size="small" embedded :bordered="false">
            <n-data-table
              size="small"
              :columns="schemaColumns"
              :data="schemaRows"
              :bordered="false"
              :single-line="false"
            />
          </n-card>

          <n-card title="Data preview" size="small" embedded :bordered="false" style="margin-top: 16px;">
            <n-spin :show="dbPreviewLoading">
              <n-empty v-if="dbPreview && dbPreview.rows.length === 0" description="No rows" />
              <n-data-table
                v-else-if="dbPreview"
                size="small"
                :columns="previewColumns"
                :data="previewRows"
                :bordered="false"
                :single-line="false"
                :scroll-x="Math.max(600, previewColumns.length * 140)"
                max-height="360"
                :row-key="(row: Record<string, string>) => row._rowKey"
              />
            </n-spin>

            <n-pagination
              v-if="dbPreview && dbPreview.total_rows > 0"
              v-model:page="dbPage"
              :page-count="dbPageCount"
              size="small"
              style="margin-top: 12px;"
            />
          </n-card>
        </n-card>

        <n-empty v-else description="Select a table to inspect" />
      </n-layout-content>
    </n-layout>
  </n-space>
</template>

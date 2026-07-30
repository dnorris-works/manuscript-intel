<script setup lang="ts">
import { computed, h, ref, watch } from 'vue';
import type { MenuOption } from 'naive-ui';
import {
  NLayout, NLayoutSider, NLayoutContent, NMenu, NDescriptions, NDescriptionsItem,
  NButton, NSpace, NCard, NDataTable, NAlert, NSpin, NEmpty, NPagination, NCode,
  NModal, NForm, NFormItem, NInput, useDialog, useMessage,
} from 'naive-ui';
import { useDatabaseInspector } from '../../../composables/useDatabaseInspector';

const props = defineProps<{
  active?: boolean;
}>();

const dialog = useDialog();
const message = useMessage();

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
  previewRows,
  tableMenuOptions,
  dbMetaItems,
  dbPage,
  dbPageCount,
  loadDbOverview,
  selectDbTable,
  onDatabaseTabActivated,
  deleteRow,
  updateRow,
} = useDatabaseInspector();

const showEditModal = ref(false);
const editingValues = ref<Record<string, string>>({});
const editingRowid = ref<number | null>(null);
const saving = ref(false);

const previewColumns = computed(() => {
  if (!dbPreview.value) return [];
  const cols = dbPreview.value.columns.map(col => ({
    title: col,
    key: col,
    ellipsis: { tooltip: true },
  }));
  cols.push({
    title: 'Actions',
    key: '_actions',
    width: 140,
    render: (row: Record<string, string>) => {
      const rowid = Number(row.rowid);
      return h(NSpace, { size: 8 }, {
        default: () => [
          h(NButton, { size: 'tiny', onClick: () => openEdit(row) }, { default: () => 'Edit' }),
          h(NButton, { size: 'tiny', type: 'error', secondary: true, onClick: () => confirmDelete(rowid) }, { default: () => 'Delete' }),
        ],
      });
    },
  } as never);
  return cols;
});

const editableColumns = computed(() => {
  if (!dbPreview.value) return [];
  return dbPreview.value.columns.filter(c => c !== 'rowid');
});

function openEdit(row: Record<string, string>): void {
  editingRowid.value = Number(row.rowid);
  const values: Record<string, string> = {};
  for (const col of editableColumns.value) {
    values[col] = row[col] ?? '';
  }
  editingValues.value = values;
  showEditModal.value = true;
}

async function saveEdit(): Promise<void> {
  if (editingRowid.value == null) return;
  saving.value = true;
  try {
    await updateRow(editingRowid.value, editingValues.value);
    showEditModal.value = false;
    message.success('Row updated.');
  } catch (e) {
    message.error('Update failed: ' + String(e));
  } finally {
    saving.value = false;
  }
}

function confirmDelete(rowid: number): void {
  dialog.warning({
    title: 'Delete row',
    content: `Delete row ${rowid}? This cannot be undone.`,
    positiveText: 'Delete',
    negativeText: 'Cancel',
    onPositiveClick: async () => {
      try {
        await deleteRow(rowid);
        message.success('Row deleted.');
      } catch (e) {
        message.error('Delete failed: ' + String(e));
      }
    },
  });
}

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
                :scroll-x="Math.max(700, previewColumns.length * 140)"
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

    <n-modal v-model:show="showEditModal" preset="card" title="Edit row" style="width: 520px;">
      <n-form v-if="editingRowid != null" label-placement="top">
        <n-form-item label="rowid">
          <n-input :value="String(editingRowid)" readonly />
        </n-form-item>
        <n-form-item v-for="col in editableColumns" :key="col" :label="col">
          <n-input v-model:value="editingValues[col]" type="textarea" :autosize="{ minRows: 1, maxRows: 6 }" />
        </n-form-item>
      </n-form>
      <template #footer>
        <n-space justify="end">
          <n-button @click="showEditModal = false">Cancel</n-button>
          <n-button type="primary" :loading="saving" @click="saveEdit">Save</n-button>
        </n-space>
      </template>
    </n-modal>
  </n-space>
</template>

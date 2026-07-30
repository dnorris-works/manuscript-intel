import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export interface DbColumnInfo {
  name: string;
  type_name: string;
  notnull: boolean;
  pk: boolean;
}

export interface DbTableInfo {
  name: string;
  row_count: number;
  columns: DbColumnInfo[];
}

export interface DbInspectOverview {
  path: string;
  file_size_bytes: number;
  tables: DbTableInfo[];
}

export interface DbTablePreview {
  table: string;
  columns: string[];
  rows: string[][];
  total_rows: number;
  offset: number;
  limit: number;
}

const DB_PAGE_SIZE = 50;

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

export function useDatabaseInspector() {
  const dbOverview = ref<DbInspectOverview | null>(null);
  const dbLoading = ref(false);
  const dbError = ref('');
  const selectedDbTable = ref<string | null>(null);
  const dbPreview = ref<DbTablePreview | null>(null);
  const dbPreviewLoading = ref(false);
  const dbPageOffset = ref(0);

  const selectedTableInfo = computed(() => {
    if (!dbOverview.value || !selectedDbTable.value) return null;
    return dbOverview.value.tables.find(t => t.name === selectedDbTable.value) ?? null;
  });

  const dbPageEnd = computed(() => {
    if (!dbPreview.value) return 0;
    return Math.min(dbPreview.value.offset + dbPreview.value.rows.length, dbPreview.value.total_rows);
  });

  const schemaColumns = computed(() => [
    { title: 'Column', key: 'name' },
    { title: 'Type', key: 'type_name' },
    { title: 'PK', key: 'pk', width: 48 },
    { title: 'NOT NULL', key: 'notnull', width: 80 },
  ]);

  const schemaRows = computed(() => {
    if (!selectedTableInfo.value) return [];
    return selectedTableInfo.value.columns.map(col => ({
      name: col.name,
      type_name: col.type_name,
      pk: col.pk ? '✓' : '',
      notnull: col.notnull ? '✓' : '',
    }));
  });

  const previewColumns = computed(() => {
    if (!dbPreview.value) return [];
    return dbPreview.value.columns.map(col => ({
      title: col,
      key: col,
      ellipsis: { tooltip: true },
    }));
  });

  const previewRows = computed(() => {
    if (!dbPreview.value) return [];
    return dbPreview.value.rows.map((row, index) => {
      const record: Record<string, string> = { _rowKey: String(index) };
      dbPreview.value!.columns.forEach((col, ci) => {
        record[col] = row[ci] ?? '';
      });
      return record;
    });
  });

  async function deleteRow(rowid: number): Promise<void> {
    if (!selectedDbTable.value) return;
    await invoke<void>('delete_database_row_cmd', {
      request: { table: selectedDbTable.value, rowid },
    });
    await loadDbTablePreview();
    await loadDbOverview();
  }

  async function updateRow(rowid: number, values: Record<string, string>): Promise<void> {
    if (!selectedDbTable.value) return;
    const filtered = { ...values };
    delete filtered.rowid;
    await invoke<void>('update_database_row_cmd', {
      request: { table: selectedDbTable.value, rowid, values: filtered },
    });
    await loadDbTablePreview();
  }

  const tableMenuOptions = computed(() => {
    if (!dbOverview.value) return [];
    return dbOverview.value.tables.map(t => ({
      label: t.name,
      key: t.name,
      extra: t.row_count.toLocaleString(),
    }));
  });

  const dbPageCount = computed(() => {
    if (!dbPreview.value || dbPreview.value.total_rows === 0) return 1;
    return Math.ceil(dbPreview.value.total_rows / DB_PAGE_SIZE);
  });

  const dbPage = computed({
    get: () => Math.floor(dbPageOffset.value / DB_PAGE_SIZE) + 1,
    set: (page: number) => {
      dbPageOffset.value = (page - 1) * DB_PAGE_SIZE;
      void loadDbTablePreview();
    },
  });

  const dbMetaItems = computed(() => {
    if (!dbOverview.value) return [];
    return [
      { label: 'File', value: dbOverview.value.path },
      { label: 'Size', value: formatBytes(dbOverview.value.file_size_bytes) },
      { label: 'Tables', value: String(dbOverview.value.tables.length) },
    ];
  });

  async function loadDbOverview(): Promise<void> {
    dbLoading.value = true;
    dbError.value = '';
    try {
      dbOverview.value = await invoke<DbInspectOverview>('inspect_database_overview');
      if (selectedDbTable.value && !dbOverview.value.tables.some(t => t.name === selectedDbTable.value)) {
        selectedDbTable.value = null;
        dbPreview.value = null;
      }
    } catch (e) {
      dbError.value = String(e);
      dbOverview.value = null;
    } finally {
      dbLoading.value = false;
    }
  }

  async function loadDbTablePreview(): Promise<void> {
    if (!selectedDbTable.value) {
      dbPreview.value = null;
      return;
    }
    dbPreviewLoading.value = true;
    dbError.value = '';
    try {
      dbPreview.value = await invoke<DbTablePreview>('inspect_database_table', {
        request: {
          table: selectedDbTable.value,
          offset: dbPageOffset.value,
          limit: DB_PAGE_SIZE,
        },
      });
    } catch (e) {
      dbError.value = String(e);
      dbPreview.value = null;
    } finally {
      dbPreviewLoading.value = false;
    }
  }

  function selectDbTable(name: string): void {
    selectedDbTable.value = name;
    dbPageOffset.value = 0;
    void loadDbTablePreview();
  }

  function onDatabaseTabActivated(): void {
    void loadDbOverview();
    if (selectedDbTable.value) {
      void loadDbTablePreview();
    }
  }

  return {
    dbOverview,
    dbLoading,
    dbError,
    selectedDbTable,
    dbPreview,
    dbPreviewLoading,
    dbPageSize: DB_PAGE_SIZE,
    selectedTableInfo,
    dbPageEnd,
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
    deleteRow,
    updateRow,
  };
}

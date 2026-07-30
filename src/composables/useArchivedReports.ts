import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { ArchivedReportRow } from '../types';

export function useArchivedReports() {
  const archivedRows = ref<ArchivedReportRow[]>([]);
  const loading = ref(false);
  const error = ref('');

  async function loadArchivedReports(folder: string): Promise<void> {
    if (!folder) {
      archivedRows.value = [];
      return;
    }
    loading.value = true;
    error.value = '';
    try {
      archivedRows.value = await invoke<ArchivedReportRow[]>('get_archived_reports', { folder });
    } catch (e) {
      error.value = String(e);
      archivedRows.value = [];
    } finally {
      loading.value = false;
    }
  }

  async function deleteArchivedReport(id: number): Promise<void> {
    await invoke<void>('delete_report_cmd', { id });
  }

  return {
    archivedRows,
    loading,
    error,
    loadArchivedReports,
    deleteArchivedReport,
  };
}

import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { ReportEnvelope, SidebarReport } from '../types';

const savedReports = ref<SidebarReport[]>([]);
const currentReport = ref<ReportEnvelope | null>(null);

async function loadSavedReports(folder: string, platform = 'saved'): Promise<void> {
  if (!folder) {
    savedReports.value = [];
    return;
  }
  try {
    savedReports.value = await invoke<SidebarReport[]>('get_sidebar_reports', { folder, platform });
  } catch (e) {
    console.error('loadSavedReports:', e);
    savedReports.value = [];
  }
}

async function openReport(id: number): Promise<ReportEnvelope> {
  const envelope = await invoke<ReportEnvelope>('get_report_cmd', { id });
  currentReport.value = envelope;
  return envelope;
}

async function deleteReport(id: number): Promise<void> {
  await invoke<void>('delete_report_cmd', { id });
}

function closeReport(): void {
  currentReport.value = null;
}

export function useReports() {
  return {
    savedReports,
    currentReport,
    loadSavedReports,
    openReport,
    deleteReport,
    closeReport,
  };
}

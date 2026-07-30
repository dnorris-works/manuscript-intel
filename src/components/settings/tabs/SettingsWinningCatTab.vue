<script setup lang="ts">
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useDialog } from 'naive-ui';
import { NButton, NText, NSpace, NAlert } from 'naive-ui';
import type { WinningCatImportResult, StaleCleanupResult } from '../../../types';

const dialog = useDialog();
const winningcatStatus = ref('');
const staleStatus = ref('');
const showStaleRow = ref(false);
const importDisabled = ref(false);
let lastImportedAt = '';

async function onImportWinningCat(): Promise<void> {
  winningcatStatus.value = 'Select the CSV file...';
  importDisabled.value = true;
  showStaleRow.value = false;
  try {
    const result = await invoke<WinningCatImportResult>('import_winningcat_csv');
    if (result.success) {
      winningcatStatus.value = `✓ Imported ${result.imported} categories (${result.imported_kindle} Kindle, ${result.imported_books} Books). Skipped ${result.skipped_other_department} (other department), ${result.skipped_unparseable} (unparseable).`;
      lastImportedAt = result.imported_at;
      if (result.stale_count > 0) {
        showStaleRow.value = true;
        const word = result.stale_count === 1 ? 'y was' : 'ies were';
        staleStatus.value = `${result.stale_count} categor${word} in the catalog from a previous import but missing from this one — possibly retired or renamed by Amazon.`;
      }
    } else {
      winningcatStatus.value = result.error || 'Import failed.';
    }
  } catch (e) {
    winningcatStatus.value = 'Error: ' + String(e);
  } finally {
    importDisabled.value = false;
  }
}

function onRemoveStale(): void {
  if (!lastImportedAt) return;
  dialog.warning({
    title: 'Remove stale categories?',
    content: 'Remove these stale categories from the catalog? This only affects reference data — no story data is touched.',
    positiveText: 'Remove Stale',
    negativeText: 'Cancel',
    onPositiveClick: async () => {
      try {
        const result = await invoke<StaleCleanupResult>('remove_stale_kdp_categories', { since: lastImportedAt });
        if (result.success) {
          const word = result.removed === 1 ? 'y' : 'ies';
          staleStatus.value = `✓ Removed ${result.removed} stale categor${word}.`;
          showStaleRow.value = false;
        } else {
          staleStatus.value = result.error || 'Cleanup failed.';
        }
      } catch (e) {
        staleStatus.value = 'Error: ' + String(e);
      }
    },
  });
}
</script>

<template>
  <n-space vertical :size="12">
    <n-text depth="3">
      Import the WinningCat category catalog CSV to enable category matching.
      Use the full Amazon categories export — it must include both
      <strong>Kindle Store</strong> and <strong>Books</strong> departments.
      Print and ebook browse trees are separate on KDP.
    </n-text>

    <n-button :disabled="importDisabled" @click="onImportWinningCat">Import CSV</n-button>

    <n-text v-if="winningcatStatus" depth="3" style="font-size: 12px;">
      {{ winningcatStatus }}
    </n-text>

    <n-alert v-if="showStaleRow" type="warning" :bordered="false">
      <n-text style="font-size: 12px;">{{ staleStatus }}</n-text>
      <n-button size="small" type="error" style="margin-top: 8px;" @click="onRemoveStale">
        Remove Stale
      </n-button>
    </n-alert>
  </n-space>
</template>

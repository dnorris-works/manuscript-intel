<script setup lang="ts">
import { inject, computed, h, watch, type VNodeChild } from 'vue';
import {
  NText, NMenu, NButton, NEmpty, NScrollbar, useDialog,
  type MenuOption,
} from 'naive-ui';
import AnalyzerPlatformTabs from './AnalyzerPlatformTabs.vue';
import { storiesKey, reportsKey, showPanelKey, analysisKey } from '../injectionKeys';

const storiesCtx = inject(storiesKey)!;
const reportsCtx = inject(reportsKey)!;
const showPanel = inject(showPanelKey)!;
const analysisCtx = inject(analysisKey)!;
const dialog = useDialog();

async function onReportClick(id: number): Promise<void> {
  await reportsCtx.openReport(id);
  showPanel('reports');
}

function onDeleteReport(id: number): void {
  dialog.warning({
    title: 'Delete report',
    content: 'Delete this saved report? This cannot be undone.',
    positiveText: 'Delete',
    negativeText: 'Cancel',
    onPositiveClick: async () => {
      try {
        await reportsCtx.deleteReport(id);
        const folder = storiesCtx.activeFolder.value;
        if (folder) {
          await analysisCtx.refreshState(folder);
          await reportsCtx.loadSavedReports(folder);
        }
      } catch (err) {
        console.error(err);
      }
    },
  });
}

const reportMenuOptions = computed<MenuOption[]>(() =>
  reportsCtx.savedReports.value.map(report => ({
    key: `report-${report.id}`,
    label: report.label,
  })),
);

function renderReportLabel(option: MenuOption): VNodeChild {
  const id = Number(String(option.key).replace('report-', ''));
  return h('div', { style: 'display:flex;align-items:center;gap:4px;width:100%;' }, [
    h('span', { style: 'flex:1;overflow:hidden;text-overflow:ellipsis;' }, option.label as string),
    h(NButton, {
      size: 'tiny',
      quaternary: true,
      type: 'error',
      onClick: (e: MouseEvent) => {
        e.stopPropagation();
        onDeleteReport(id);
      },
    }, { default: () => '×' }),
  ]);
}

function onReportMenuSelect(key: string): void {
  if (key.startsWith('report-')) {
    void onReportClick(Number(key.replace('report-', '')));
  }
}

watch(() => storiesCtx.activeFolder.value, (folder) => {
  void reportsCtx.loadSavedReports(folder ?? '');
}, { immediate: true });
</script>

<template>
  <div class="saved-reports-root">
    <n-text depth="3" style="display: block; margin-bottom: 12px; font-size: 13px;">
      {{ storiesCtx.activeStory.value ? `Story: ${storiesCtx.activeStory.value.name}` : 'Select or create a story to begin.' }}
    </n-text>

    <AnalyzerPlatformTabs />

    <n-scrollbar class="saved-reports-scroll">
      <n-empty
        v-if="!storiesCtx.activeFolder.value"
        description="Select a story to see saved reports."
      />
      <n-empty
        v-else-if="reportsCtx.savedReports.value.length === 0"
        description="No saved reports yet. Run reports from KDP/Wide, Craft, or Publish."
      />
      <n-menu
        v-else
        :options="reportMenuOptions"
        :render-label="renderReportLabel"
        @update:value="onReportMenuSelect"
      />
    </n-scrollbar>
  </div>
</template>

<style scoped>
.saved-reports-root {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 20px;
  overflow: hidden;
}

.saved-reports-scroll {
  flex: 1;
  min-height: 0;
}
</style>

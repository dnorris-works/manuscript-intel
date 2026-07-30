<script setup lang="ts">
import { inject, ref, computed, onMounted, onUnmounted } from 'vue';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import {
  NPageHeader, NScrollbar, NButton, NSpace, NEmpty, useDialog, useMessage,
} from 'naive-ui';
import { renderReport } from '../reportRenderer';
import { storiesKey, reportsKey, platformKey, showPanelKey, openManuscriptEditorKey, analysisKey } from '../injectionKeys';
import type { Finding } from '../types';

const reportsCtx = inject(reportsKey)!;
const storiesCtx = inject(storiesKey)!;
const platformCtx = inject(platformKey)!;
const showPanel = inject(showPanelKey)!;
const openManuscriptEditor = inject(openManuscriptEditorKey)!;
const analysisCtx = inject(analysisKey)!;
const dialog = useDialog();
const message = useMessage();

const copyLabel = ref('Copy');

const report = computed(() => reportsCtx.currentReport.value);

const renderedHtml = computed(() => {
  if (!report.value) return '';
  const storyName = storiesCtx.activeStory.value?.name || '';
  return renderReport(report.value, storyName);
});

const reportTitle = computed(() => report.value?.label ?? '');

async function onCopy(): Promise<void> {
  if (!report.value) return;
  await writeText(report.value.content);
  copyLabel.value = 'Copied!';
  setTimeout(() => { copyLabel.value = 'Copy'; }, 1500);
}

function onDelete(): void {
  if (!report.value) return;
  dialog.warning({
    title: 'Delete report',
    content: 'Delete this report version? This cannot be undone.',
    positiveText: 'Delete',
    negativeText: 'Cancel',
    onPositiveClick: async () => {
      try {
        await reportsCtx.deleteReport(report.value!.id);
        const folder = storiesCtx.activeFolder.value;
        if (folder) {
          await analysisCtx.refreshState(folder);
          await reportsCtx.loadSavedReports(folder);
        }
        reportsCtx.closeReport();
        showPanel('analyzer');
      } catch (e) {
        message.error('Could not delete: ' + String(e));
      }
    },
  });
}

function onClose(): void {
  reportsCtx.closeReport();
  showPanel('analyzer');
}

function openEditorForSdt(chapterIndex: number, violationIndex: number): void {
  openEditorForPassageReport(chapterIndex, violationIndex, 'show_dont_tell');
}

function openEditorForAiIsms(chapterIndex: number, violationIndex: number): void {
  openEditorForPassageReport(chapterIndex, violationIndex, 'ai_isms');
}

function openEditorForPassageReport(
  chapterIndex: number,
  violationIndex: number,
  reportType: 'show_dont_tell' | 'ai_isms',
): void {
  if (!report.value || report.value.format !== 'json') return;
  const folder = storiesCtx.activeFolder.value;
  if (!folder) return;

  const data = JSON.parse(report.value.content);
  const chapters: any[] = data.chapters || [];

  const findings: Finding[] = [];
  let targetIdx = 0;

  chapters.forEach((ch: any, chIdx: number) => {
    const violations: any[] = ch.violations || [];
    violations.forEach((v: any, vIdx: number) => {
      if (chIdx === chapterIndex && vIdx === violationIndex) {
        targetIdx = findings.length;
      }
      findings.push({
        filePath: folder + '/' + (ch.file || ''),
        chapterTitle: ch.title || ch.file || '',
        tellingText: v.telling_text || '',
        context: v.context || '',
        why: v.why || '',
        severity: v.severity || 'minor',
        reportType,
      });
    });
  });

  if (findings.length > 0) {
    openManuscriptEditor(findings, targetIdx);
  }
}

function openEditorForContinuity(findingIndex: number): void {
  if (!report.value || report.value.format !== 'json') return;
  const folder = storiesCtx.activeFolder.value;
  if (!folder) return;

  const data = JSON.parse(report.value.content);
  const reportFindings: any[] = data.findings || [];

  const findings: Finding[] = [];

  reportFindings.forEach((f: any) => {
    const occs: any[] = f.occurrences || [];
    const firstOcc = occs[0] || {};
    findings.push({
      filePath: folder + '/' + (firstOcc.file || ''),
      chapterTitle: firstOcc.chapter_title || firstOcc.file || '',
      tellingText: firstOcc.snippet || '',
      context: '',
      why: f.explanation || '',
      severity: f.verdict === 'contradiction' ? 'major' : 'moderate',
      reportType: 'continuity',
      entity: f.entity,
      attribute: f.attribute,
      explanation: f.explanation,
      occurrences: occs.map((o: any) => ({
        story_name: o.story_name || '',
        file: o.file || '',
        chapter_title: o.chapter_title || '',
        value: o.value || '',
        snippet: o.snippet || '',
      })),
    });
  });

  if (findings.length > 0) {
    openManuscriptEditor(findings, findingIndex);
  }
}

const contentRef = ref<HTMLElement | null>(null);

function onContentClick(e: MouseEvent): void {
  const target = e.target as HTMLElement;
  if (target.classList.contains('suggest-fix-link')) {
    e.preventDefault();
    const idx = parseInt(target.dataset.findingIndex || '', 10);
    if (!isNaN(idx)) openEditorForContinuity(idx);
  }
  if (target.classList.contains('suggest-sdt-fix-link')) {
    e.preventDefault();
    const chIdx = parseInt(target.dataset.chapterIndex || '', 10);
    const vIdx = parseInt(target.dataset.violationIndex || '', 10);
    if (!isNaN(chIdx) && !isNaN(vIdx)) openEditorForSdt(chIdx, vIdx);
  }
  if (target.classList.contains('suggest-ai-isms-fix-link')) {
    e.preventDefault();
    const chIdx = parseInt(target.dataset.chapterIndex || '', 10);
    const vIdx = parseInt(target.dataset.violationIndex || '', 10);
    if (!isNaN(chIdx) && !isNaN(vIdx)) openEditorForAiIsms(chIdx, vIdx);
  }
}

onMounted(() => {
  contentRef.value?.addEventListener('click', onContentClick);
});

onUnmounted(() => {
  contentRef.value?.removeEventListener('click', onContentClick);
});
</script>

<template>
  <div v-if="report" class="reports-root">
    <header class="reports-header">
      <n-page-header :title="reportTitle">
        <template #extra>
          <n-space>
            <n-button size="small" @click="onCopy">{{ copyLabel }}</n-button>
            <n-button size="small" type="error" ghost @click="onDelete">Delete</n-button>
            <n-button size="small" quaternary @click="onClose">Close</n-button>
          </n-space>
        </template>
      </n-page-header>
    </header>

    <n-scrollbar class="reports-scroll">
      <div ref="contentRef" class="reports-content" v-html="renderedHtml" />
    </n-scrollbar>
  </div>

  <div v-else class="reports-root reports-empty">
    <n-empty description="No report selected." style="margin: auto;">
      <template #extra>
        <n-button @click="onClose">Back</n-button>
      </template>
    </n-empty>
  </div>
</template>

<style scoped>
.reports-root {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.reports-header {
  flex-shrink: 0;
  padding: 16px 24px 0;
  background: var(--bg);
  border-bottom: 1px solid var(--border);
}

.reports-scroll {
  flex: 1;
  min-height: 0;
}

.reports-content {
  padding: 16px 24px 24px;
  user-select: text;
}

.reports-empty {
  padding: 24px;
}
</style>

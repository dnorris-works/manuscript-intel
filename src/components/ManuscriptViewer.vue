<script setup lang="ts">
import { ref, computed, inject } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import {
  NPageHeader, NButton, NSpace, NTag, NAlert, NInput, NSpin, useDialog, useMessage,
} from 'naive-ui';
import { useSettings } from '../composables/useSettings';
import { formatMarkdown } from '../formatMarkdown';
import ChapterEditor from './ChapterEditor.vue';
import type { Finding } from '../types';

// ── Props ─────────────────────────────────────────────────────────────────────

const props = defineProps<{
  findings: Finding[];
  startIndex: number;
  storyFolder: string;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
}>();

const settings = useSettings();
const bumpFileTree = inject<() => void>('bumpFileTree', () => {});
const dialog = useDialog();
const message = useMessage();

// ── State ─────────────────────────────────────────────────────────────────────

const currentIndex = ref(props.startIndex);
const suggestion = ref('');
const loadingSuggestion = ref(false);
const suggestionError = ref('');
const applyText = ref('');
const deleting = ref(false);

const editorRef = ref<InstanceType<typeof ChapterEditor> | null>(null);

// ── Computed ──────────────────────────────────────────────────────────────────

const finding = computed(() => props.findings[currentIndex.value]);
const totalFindings = computed(() => props.findings.length);
const isReadMode = computed(() => !finding.value?.tellingText);

// ── Generate suggestion ───────────────────────────────────────────────────────

async function onSuggestFix(): Promise<void> {
  if (!finding.value) return;

  const proseModel = settings.modelFor('prose');
  if (!proseModel) {
    suggestionError.value = 'No model set. Go to Settings.';
    return;
  }

  suggestion.value = '';
  suggestionError.value = '';
  loadingSuggestion.value = true;

  try {
    if (finding.value.reportType === 'show_dont_tell') {
      const result = await invoke<{ success: boolean; suggestions: string; error: string }>('suggest_sdt_fix', {
        request: {
          provider: settings.provider.value,
          api_key: settings.apiKey.value,
          model: proseModel,
          telling_text: finding.value.tellingText,
          context: finding.value.context,
          why: finding.value.why,
          chapter_title: finding.value.chapterTitle,
          folder: props.storyFolder,
        }
      });
      if (result.success) suggestion.value = result.suggestions;
      else suggestionError.value = result.error;
    } else if (finding.value.reportType === 'ai_isms') {
      const result = await invoke<{ success: boolean; suggestions: string; error: string }>('suggest_ai_isms_fix', {
        request: {
          provider: settings.provider.value,
          api_key: settings.apiKey.value,
          model: proseModel,
          telling_text: finding.value.tellingText,
          context: finding.value.context,
          why: finding.value.why,
          chapter_title: finding.value.chapterTitle,
          folder: props.storyFolder,
        }
      });
      if (result.success) suggestion.value = result.suggestions;
      else suggestionError.value = result.error;
    } else if (finding.value.reportType === 'continuity') {
      const result = await invoke<{ success: boolean; suggestions: string; error: string }>('suggest_continuity_fix', {
        request: {
          provider: settings.provider.value,
          api_key: settings.apiKey.value,
          model: proseModel,
          entity: finding.value.entity || '',
          attribute: finding.value.attribute || '',
          explanation: finding.value.explanation || '',
          occurrences: finding.value.occurrences || [],
          folder: props.storyFolder,
        }
      });
      if (result.success) suggestion.value = result.suggestions;
      else suggestionError.value = result.error;
    }
  } catch (e) {
    suggestionError.value = String(e);
  } finally {
    loadingSuggestion.value = false;
  }
}

// ── Apply: replace the highlighted text in the editor ─────────────────────────

function onReplace(): void {
  if (!applyText.value.trim() || !editorRef.value) return;
  editorRef.value.replaceSelection(applyText.value);
  applyText.value = '';
}

function onInsertAtCursor(): void {
  if (!applyText.value.trim() || !editorRef.value) return;
  editorRef.value.insertAtCursor(applyText.value);
  applyText.value = '';
}

// ── Navigation ────────────────────────────────────────────────────────────────

function onPrev(): void {
  if (currentIndex.value > 0) {
    currentIndex.value--;
    resetSuggestion();
  }
}

function onNext(): void {
  if (currentIndex.value < totalFindings.value - 1) {
    currentIndex.value++;
    resetSuggestion();
  }
}

function resetSuggestion(): void {
  suggestion.value = '';
  suggestionError.value = '';
  applyText.value = '';
}

function onClose(): void {
  // Save before closing
  editorRef.value?.saveNow();
  emit('close');
}

function onDelete(): void {
  const path = finding.value?.filePath;
  if (!path || !props.storyFolder) return;
  const name = finding.value?.chapterTitle || 'this document';
  dialog.warning({
    title: 'Delete document',
    content: `Delete “${name}”? This cannot be undone.`,
    positiveText: 'Delete',
    negativeText: 'Cancel',
    onPositiveClick: async () => {
      deleting.value = true;
      try {
        await invoke<void>('delete_story_document', {
          storyFolder: props.storyFolder,
          filePath: path,
        });
        bumpFileTree();
        emit('close');
      } catch (e) {
        message.error('Could not delete: ' + String(e));
      } finally {
        deleting.value = false;
      }
    },
  });
}
</script>

<template>
  <div class="manuscript-viewer">
    <header class="mv-header">
      <n-page-header :title="finding?.chapterTitle || ''">
        <template #extra>
          <n-space align="center">
            <n-text v-if="!isReadMode" depth="3" style="font-size: 12px;">
              {{ currentIndex + 1 }} of {{ totalFindings }}
            </n-text>
            <n-button v-if="!isReadMode" size="small" :disabled="currentIndex === 0" @click="onPrev">Prev</n-button>
            <n-button v-if="!isReadMode" size="small" :disabled="currentIndex === totalFindings - 1" @click="onNext">Next</n-button>
            <n-button size="small" @click="onClose">Close</n-button>
            <n-button size="small" type="error" ghost :loading="deleting" @click="onDelete">Delete</n-button>
          </n-space>
        </template>
      </n-page-header>
    </header>

    <div class="mv-body" :class="{ 'read-mode': isReadMode }">
      <div class="mv-chapter">
        <ChapterEditor
          ref="editorRef"
          :file-path="finding?.filePath || ''"
          :highlight-text="isReadMode ? '' : (finding?.tellingText || '')"
        />
      </div>

      <div v-if="!isReadMode" class="mv-suggestion-panel">
        <n-card size="small">
          <n-tag size="small" style="margin-bottom: 6px; text-transform: capitalize;">{{ finding?.severity }}</n-tag>
          <n-text style="font-size: 13px; line-height: 1.5;">{{ finding?.why }}</n-text>
        </n-card>

        <n-button v-if="!suggestion && !loadingSuggestion" type="primary" @click="onSuggestFix">
          Suggest Fix
        </n-button>

        <n-spin v-if="loadingSuggestion" size="small">Generating suggestion...</n-spin>
        <n-alert v-if="suggestionError" type="error" :show-icon="false">{{ suggestionError }}</n-alert>

        <div v-if="suggestion" class="mv-suggestion-content">
          <div class="mv-suggestion-text" v-html="formatMarkdown(suggestion)"></div>
          <div class="mv-apply-section">
            <n-text depth="3" style="font-size: 11px; display: block; margin-bottom: 4px;">Replacement text:</n-text>
            <n-input v-model:value="applyText" type="textarea" :rows="3" placeholder="Paste or type the replacement text" />
            <n-space style="margin-top: 8px;">
              <n-button size="small" type="primary" :disabled="!applyText.trim()" @click="onReplace">Replace selected</n-button>
              <n-button size="small" :disabled="!applyText.trim()" @click="onInsertAtCursor">Insert at cursor</n-button>
              <n-button size="small" quaternary @click="onNext">Skip</n-button>
            </n-space>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.manuscript-viewer {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.mv-header {
  flex-shrink: 0;
  padding: 8px 16px 0;
  border-bottom: 1px solid var(--border);
}

.mv-body {
  flex: 1;
  display: flex;
  overflow: hidden;
}

.mv-chapter {
  flex: 0 0 60%;
  overflow: hidden;
  border-right: 1px solid var(--border);
}

.read-mode .mv-chapter {
  flex: 1;
  border-right: none;
}

.mv-suggestion-panel {
  flex: 0 0 40%;
  overflow-y: auto;
  padding: 16px 20px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.mv-suggestion-content { flex: 1; }

.mv-suggestion-text {
  font-size: 13px;
  line-height: 1.6;
  color: var(--text);
}

.mv-suggestion-text :deep(pre) {
  background: var(--surface2);
  padding: 10px;
  border-radius: var(--radius);
  overflow-x: auto;
  font-size: 12px;
}

.mv-suggestion-text :deep(code) {
  background: var(--surface2);
  padding: 1px 4px;
  border-radius: 3px;
  font-size: 12px;
}

.mv-apply-section {
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--border);
}
</style>

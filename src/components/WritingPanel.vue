<script setup lang="ts">
import { ref, watch, computed, inject } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { NPageHeader, NButton, NSpace, NEmpty, NText, useDialog, useMessage } from 'naive-ui';
import ChapterEditor from './ChapterEditor.vue';
import AiChat from './AiChat.vue';

const props = defineProps<{
  filePath: string;
  chapterTitle: string;
  storyFolder: string;
}>();

const openNewDocumentForm = inject<() => void>('openNewDocumentForm')!;
const closeWritingDocument = inject<() => void>('closeWritingDocument')!;
const bumpFileTree = inject<() => void>('bumpFileTree')!;
const dialog = useDialog();
const message = useMessage();

const editorRef = ref<InstanceType<typeof ChapterEditor> | null>(null);
const chapterText = ref('');
const deleting = ref(false);

const selectedText = computed(() => editorRef.value?.selectedText || '');
const pinnedCount = computed(() => editorRef.value?.pinnedCount || 0);

function pinSelection(): void {
  editorRef.value?.pinSelection();
}

function clearPins(): void {
  editorRef.value?.clearPins();
}

watch(() => props.filePath, async (fp) => {
  if (!fp) { chapterText.value = ''; return; }
  try {
    chapterText.value = await invoke<string>('read_chapter', { filePath: fp });
  } catch {
    chapterText.value = '';
  }
}, { immediate: true });

function onEditorSaved(): void {
  if (props.filePath) {
    invoke<string>('read_chapter', { filePath: props.filePath })
      .then(text => { chapterText.value = text; })
      .catch(() => {});
  }
}

async function onClose(): Promise<void> {
  await editorRef.value?.saveNow();
  closeWritingDocument();
}

function onDelete(): void {
  if (!props.filePath || !props.storyFolder) return;
  const name = props.chapterTitle || 'this document';
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
          filePath: props.filePath,
        });
        bumpFileTree();
        closeWritingDocument();
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
  <div class="writing-panel">
    <n-empty
      v-if="!filePath"
      description="Create a chapter, bible page, or other document to start writing."
      style="margin: auto;"
    >
      <template #extra>
        <n-space vertical align="center" :size="8">
          <n-text strong>No document open</n-text>
          <n-button type="primary" @click="openNewDocumentForm">New document</n-button>
        </n-space>
      </template>
    </n-empty>

    <template v-else>
      <header class="writing-header">
        <n-page-header :title="chapterTitle">
          <template #extra>
            <n-space>
              <n-button @click="onClose">Close</n-button>
              <n-button type="error" ghost :loading="deleting" @click="onDelete">Delete</n-button>
            </n-space>
          </template>
        </n-page-header>
      </header>
      <div class="writing-body">
        <div class="writing-editor">
          <ChapterEditor
            ref="editorRef"
            :file-path="filePath"
            @saved="onEditorSaved"
          />
        </div>
        <div class="writing-chat">
          <AiChat
            :chapter-text="chapterText"
            :chapter-title="chapterTitle"
            :story-folder="storyFolder"
            :selected-text="selectedText"
            :pinned-count="pinnedCount"
            @pin="pinSelection"
            @clear-pins="clearPins"
          />
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.writing-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.writing-header {
  flex-shrink: 0;
  padding: 8px 16px 0;
  border-bottom: 1px solid var(--border);
}

.writing-body {
  flex: 1;
  display: flex;
  overflow: hidden;
}

.writing-editor {
  flex: 1;
  overflow: hidden;
}

.writing-chat {
  flex: 0 0 320px;
  overflow: hidden;
  border-left: 1px solid var(--border);
}
</style>

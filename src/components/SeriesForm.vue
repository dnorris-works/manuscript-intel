<script setup lang="ts">
import { inject, ref, watch, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import {
  NForm, NFormItem, NInput, NInputGroup, NRadioGroup, NRadio, NButton, NSpace,
  NAlert, NCheckbox, NList, NListItem, NThing, NButtonGroup, useDialog,
} from 'naive-ui';
import type { Story, Series, SeriesResult } from '../types';
import { storiesKey, showPanelKey } from '../injectionKeys';
import { useSeries } from '../composables/useSeries';
import FormPanelShell from './FormPanelShell.vue';

const storiesCtx = inject(storiesKey)!;
const showPanel = inject(showPanelKey)!;
const { createSeries, updateSeries, deleteSeries } = useSeries();
const dialog = useDialog();

const props = defineProps<{
  series: Series | null;
}>();

type CreateMode = 'link' | 'create';

const name = ref('');
const biblePath = ref('');
const createMode = ref<CreateMode>('link');
const parentFolder = ref('');
const selectedBooks = ref<{ story_folder: string; story_name: string; book_order: number }[]>([]);
const error = ref('');
const isEditing = ref(false);
const editId = ref(0);

const panelTitle = computed(() => (isEditing.value ? 'Edit Series' : 'New Series'));
const availableStories = computed(() => storiesCtx.stories.value);

watch(() => props.series, (s) => {
  if (s) {
    name.value = s.name;
    biblePath.value = s.bible_path || '';
    selectedBooks.value = s.books.map(b => ({ ...b }));
    editId.value = s.id;
    isEditing.value = true;
  } else {
    name.value = '';
    biblePath.value = '';
    createMode.value = 'link';
    parentFolder.value = '';
    selectedBooks.value = [];
    editId.value = 0;
    isEditing.value = false;
  }
  error.value = '';
}, { immediate: true });

function isStorySelected(folder: string): boolean {
  return selectedBooks.value.some(b => b.story_folder === folder);
}

function toggleStory(story: Story): void {
  const idx = selectedBooks.value.findIndex(b => b.story_folder === story.folder);
  if (idx >= 0) {
    selectedBooks.value.splice(idx, 1);
    selectedBooks.value.forEach((b, i) => { b.book_order = i + 1; });
  } else {
    selectedBooks.value.push({
      story_folder: story.folder,
      story_name: story.name,
      book_order: selectedBooks.value.length + 1,
    });
  }
}

function moveUp(idx: number): void {
  if (idx <= 0) return;
  const items = [...selectedBooks.value];
  [items[idx - 1], items[idx]] = [items[idx], items[idx - 1]];
  items.forEach((b, i) => { b.book_order = i + 1; });
  selectedBooks.value = items;
}

function moveDown(idx: number): void {
  if (idx >= selectedBooks.value.length - 1) return;
  const items = [...selectedBooks.value];
  [items[idx], items[idx + 1]] = [items[idx + 1], items[idx]];
  items.forEach((b, i) => { b.book_order = i + 1; });
  selectedBooks.value = items;
}

async function onSave(): Promise<void> {
  const trimName = name.value.trim();
  if (!trimName) { error.value = 'Enter a series name.'; return; }
  if (!isEditing.value && createMode.value === 'create' && !parentFolder.value.trim()) {
    error.value = 'Select a parent folder for the new series.';
    return;
  }
  error.value = '';

  let result: SeriesResult;
  if (isEditing.value) {
    result = await updateSeries(editId.value, trimName, selectedBooks.value, biblePath.value.trim());
  } else {
    result = await createSeries(
      trimName,
      selectedBooks.value,
      biblePath.value.trim(),
      createMode.value === 'create',
      parentFolder.value.trim(),
    );
  }

  if (!result.success) {
    error.value = result.error;
    return;
  }
  showPanel('analyzer');
}

function onCancel(): void {
  showPanel('analyzer');
}

async function onPickParentFolder(): Promise<void> {
  try {
    const title = createMode.value === 'create'
      ? 'Select Parent Folder for New Series'
      : 'Select Existing Series Folder';
    const path = await invoke<string>('pick_manuscript_folder', { title });
    if (path) parentFolder.value = path;
  } catch (e) {
    if (!String(e).includes('No folder')) {
      error.value = String(e);
    }
  }
}

function onDelete(): void {
  if (!editId.value) return;
  dialog.warning({
    title: 'Delete series',
    content: 'Delete this series? Books are not affected.',
    positiveText: 'Delete',
    negativeText: 'Cancel',
    onPositiveClick: async () => {
      const result = await deleteSeries(editId.value);
      if (!result.success) {
        error.value = result.error;
        return;
      }
      showPanel('analyzer');
    },
  });
}
</script>

<template>
  <FormPanelShell :title="panelTitle">
    <n-form label-placement="top">
      <n-form-item v-if="!isEditing" label="How to add">
        <n-radio-group v-model:value="createMode">
          <n-space vertical>
            <n-radio value="link">
              <strong>Link series only</strong>
              <div style="font-size: 12px; color: var(--text-muted); margin-top: 2px;">
                Create the series now and add books later.
              </div>
            </n-radio>
            <n-radio value="create">
              <strong>Create empty series folder</strong>
              <div style="font-size: 12px; color: var(--text-muted); margin-top: 2px;">
                Creates a folder named after this series with Bible, Characters, Locations, Books, and your extra scaffold folders.
              </div>
            </n-radio>
          </n-space>
        </n-radio-group>
      </n-form-item>

      <n-form-item label="Series Name">
        <n-input v-model:value="name" placeholder="e.g. The Calloway Brothers" />
      </n-form-item>

      <n-form-item
        v-if="!isEditing && createMode === 'create'"
        label="Parent Folder"
        feedback="New series folder will be created here"
      >
        <n-input-group>
          <n-input
            v-model:value="parentFolder"
            placeholder="/path/to/series-parent"
            readonly
            style="font-family: var(--mono);"
          />
          <n-button @click="onPickParentFolder">Browse</n-button>
        </n-input-group>
      </n-form-item>

      <n-form-item
        label="Series Bible"
        feedback="Optional — markdown file with series-wide canon; auto-created for empty-series scaffold"
      >
        <n-input v-model:value="biblePath" placeholder="/path/to/series-bible.md" />
      </n-form-item>

      <n-form-item label="Books in Order" feedback="Check to include, use arrows to reorder.">
        <n-space vertical>
          <n-checkbox
            v-for="story in availableStories"
            :key="story.folder"
            :checked="isStorySelected(story.folder)"
            :label="story.name"
            @update:checked="toggleStory(story)"
          />
        </n-space>

        <n-list v-if="selectedBooks.length > 0" bordered style="margin-top: 12px;">
          <template #header>Reading order</template>
          <n-list-item v-for="(book, idx) in selectedBooks" :key="book.story_folder">
            <n-thing :title="`${idx + 1}. ${book.story_name}`">
              <template #action>
                <n-button-group size="small">
                  <n-button :disabled="idx === 0" @click="moveUp(idx)">↑</n-button>
                  <n-button :disabled="idx === selectedBooks.length - 1" @click="moveDown(idx)">↓</n-button>
                </n-button-group>
              </template>
            </n-thing>
          </n-list-item>
        </n-list>
      </n-form-item>
    </n-form>

    <n-alert v-if="error" type="error" :show-icon="false" style="margin-top: 8px;">
      {{ error }}
    </n-alert>

    <template #footer>
      <n-space>
        <n-button type="primary" @click="onSave">Save</n-button>
        <n-button @click="onCancel">Cancel</n-button>
        <n-button v-if="isEditing" type="error" ghost @click="onDelete">Delete</n-button>
      </n-space>
    </template>
  </FormPanelShell>
</template>

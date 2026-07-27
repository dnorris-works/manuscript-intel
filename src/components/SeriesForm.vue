<script setup lang="ts">
import { inject, ref, watch, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { Story, Series, SeriesResult } from '../types';
import { storiesKey, showPanelKey } from '../injectionKeys';
import { useSeries } from '../composables/useSeries';

const storiesCtx = inject(storiesKey)!;
const showPanel = inject(showPanelKey)!;
const { createSeries, updateSeries, deleteSeries } = useSeries();

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

const availableStories = computed(() => storiesCtx.stories.value);

function isStorySelected(folder: string): boolean {
  return selectedBooks.value.some(b => b.story_folder === folder);
}

function toggleStory(story: Story): void {
  const idx = selectedBooks.value.findIndex(b => b.story_folder === story.folder);
  if (idx >= 0) {
    selectedBooks.value.splice(idx, 1);
    // Re-number
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

async function onDelete(): Promise<void> {
  if (!editId.value) return;
  if (!confirm('Delete this series? Books are not affected.')) return;
  const result = await deleteSeries(editId.value);
  if (!result.success) { error.value = result.error; return; }
  showPanel('analyzer');
}
</script>

<template>
  <div class="panel series-form-panel">
    <h2 class="panel-title">{{ isEditing ? 'Edit Series' : 'New Series' }}</h2>

    <div v-if="!isEditing" class="form-group">
      <label>How to add</label>
      <div class="create-options">
        <label class="create-option" :class="{ selected: createMode === 'link' }">
          <input v-model="createMode" type="radio" value="link" />
          <span class="create-option-title">Link series only</span>
          <span class="create-option-desc">Create the series now and add books later.</span>
        </label>
        <label class="create-option" :class="{ selected: createMode === 'create' }">
          <input v-model="createMode" type="radio" value="create" />
          <span class="create-option-title">Create empty series folder</span>
          <span class="create-option-desc">Creates a folder named after this series with Bible, Characters, Locations, Books, and your extra scaffold folders.</span>
        </label>
      </div>
    </div>

    <div class="form-group">
      <label>Series Name</label>
      <input v-model="name" type="text" placeholder="e.g. The Calloway Brothers" />
    </div>

    <div v-if="!isEditing && createMode === 'create'" class="form-group">
      <label>Parent Folder <span style="text-transform:none;letter-spacing:0;font-size:11px;font-weight:400">(new series folder will be created here)</span></label>
      <div class="folder-row">
        <input v-model="parentFolder" type="text" placeholder="/path/to/series-parent" readonly />
        <button class="btn btn-sm" @click="onPickParentFolder">Browse</button>
      </div>
    </div>

    <div class="form-group">
      <label>Series Bible <span style="text-transform:none;letter-spacing:0;font-size:11px;font-weight:400">(optional — markdown file with series-wide canon; auto-created for empty-series scaffold)</span></label>
      <input v-model="biblePath" type="text" placeholder="/path/to/series-bible.md" />
    </div>

    <div class="form-group">
      <label>Books in Order</label>
      <p class="form-hint">Check to include, use arrows to reorder.</p>

      <div class="book-list">
        <div
          v-for="story in availableStories"
          :key="story.folder"
          class="book-item"
          :class="{ selected: isStorySelected(story.folder) }"
          @click="toggleStory(story)"
        >
          <input type="checkbox" :checked="isStorySelected(story.folder)" @click.stop />
          <span class="book-name">{{ story.name }}</span>
        </div>
      </div>

      <div v-if="selectedBooks.length > 0" class="ordered-list">
        <div class="ordered-label">Reading order:</div>
        <div v-for="(book, idx) in selectedBooks" :key="book.story_folder" class="ordered-item">
          <span class="order-num">{{ idx + 1 }}.</span>
          <span class="order-name">{{ book.story_name }}</span>
          <button class="order-btn" @click="moveUp(idx)" :disabled="idx === 0">&#x25B2;</button>
          <button class="order-btn" @click="moveDown(idx)" :disabled="idx === selectedBooks.length - 1">&#x25BC;</button>
        </div>
      </div>
    </div>

    <div v-if="error" class="form-error">{{ error }}</div>

    <div class="form-actions">
      <button class="btn" @click="onSave">Save</button>
      <button class="btn btn-secondary" @click="onCancel">Cancel</button>
      <button v-if="isEditing" class="btn btn-danger" @click="onDelete">Delete</button>
    </div>
  </div>
</template>

<style scoped>
.series-form-panel {
  padding: 20px;
  max-width: 500px;
}
.panel-title { font-size: 16px; font-weight: 700; margin-bottom: 16px; }
.form-group { margin-bottom: 14px; }
.form-group label { display: block; font-size: 12px; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.06em; margin-bottom: 6px; }
.form-group input[type="text"] { background: var(--surface2); border: 1px solid var(--border); border-radius: var(--radius); color: var(--text); font-size: 13px; padding: 8px 10px; width: 100%; }
.form-hint { font-size: 11px; color: var(--text-muted); margin-bottom: 8px; }

.create-options { display: flex; flex-direction: column; gap: 8px; }
.create-option {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 10px 12px 10px 32px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface2);
  cursor: pointer;
  position: relative;
  color: var(--text);
}
.create-option.selected {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 10%, var(--surface2));
}
.create-option input[type="radio"] {
  position: absolute;
  left: 10px;
  top: 12px;
  margin: 0;
}
.create-option-title { font-size: 13px; font-weight: 600; }
.create-option-desc { font-size: 11px; color: var(--text-muted); font-weight: 400; }

.folder-row { display: flex; gap: 8px; align-items: center; }
.folder-row input { flex: 1; font-family: var(--mono); font-size: 12px; }

.book-list { display: flex; flex-direction: column; gap: 4px; margin-bottom: 12px; }
.book-item { display: flex; align-items: center; gap: 8px; padding: 6px 10px; border-radius: var(--radius); cursor: pointer; font-size: 13px; color: var(--text-muted); border: 1px solid var(--border); }
.book-item:hover { border-color: var(--accent); color: var(--text); }
.book-item.selected { background: var(--surface2); color: var(--text); border-color: var(--accent); }
.book-item input[type="checkbox"] { accent-color: var(--accent); pointer-events: none; }
.book-name { flex: 1; }

.ordered-list { padding: 10px; background: var(--surface2); border-radius: var(--radius); }
.ordered-label { font-size: 11px; color: var(--text-muted); text-transform: uppercase; margin-bottom: 6px; }
.ordered-item { display: flex; align-items: center; gap: 8px; padding: 4px 0; font-size: 13px; }
.order-num { color: var(--accent); font-weight: 600; min-width: 20px; }
.order-name { flex: 1; }
.order-btn { background: none; border: 1px solid var(--border); color: var(--text-muted); width: 22px; height: 22px; border-radius: 4px; cursor: pointer; font-size: 10px; display: flex; align-items: center; justify-content: center; }
.order-btn:hover:not(:disabled) { color: var(--accent); border-color: var(--accent); }
.order-btn:disabled { opacity: 0.3; cursor: not-allowed; }

.form-error { color: var(--danger); font-size: 12px; margin-bottom: 12px; }
.form-actions { display: flex; gap: 8px; margin-top: 16px; }
.btn { background: var(--accent); border: none; border-radius: var(--radius); color: #fff; cursor: pointer; font-size: 13px; font-weight: 600; padding: 9px 18px; transition: background 0.15s; }
.btn:hover { background: var(--accent-dim); }
.btn-secondary { background: var(--surface2); border: 1px solid var(--border); color: var(--text-muted); }
.btn-secondary:hover { color: var(--text); border-color: var(--accent); }
.btn-danger { background: #c0392b; color: #fff; }
.btn-danger:hover { background: #a93226; }
</style>

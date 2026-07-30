<script setup lang="ts">
import { computed, inject, ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import {
  NForm, NFormItem, NInput, NInputGroup, NRadioGroup, NRadio, NButton, NSpace, NAlert, useDialog,
} from 'naive-ui';
import { storiesKey, showPanelKey, settingsKey } from '../injectionKeys';
import type { Story, StoriesResult } from '../types';
import FormPanelShell from './FormPanelShell.vue';

const storiesCtx = inject(storiesKey)!;
const settingsCtx = inject(settingsKey)!;
const showPanel = inject(showPanelKey)!;
const dialog = useDialog();

const props = defineProps<{
  story: Story | null;
}>();

type CreateMode = 'link' | 'create';

const name = ref('');
const folder = ref('');
const biblePath = ref('');
const error = ref('');
const isEditing = ref(false);
const editId = ref('');
const createMode = ref<CreateMode>('link');

const structureHint = computed(() => {
  const s = settingsCtx.folderStructure.value;
  const ms = s.manuscript || 'Manuscript';
  const acts = (s.acts?.length ? s.acts : ['Act-1', 'Act-2', 'Act-3'])
    .map(a => `${ms}/${a}`);
  return [
    ...acts,
    s.bible, s.characters, s.locations,
    ...(s.extra || []),
  ]
    .filter(Boolean)
    .join(', ');
});

const folderLabel = computed(() => {
  if (isEditing.value) return 'Story Folder';
  return createMode.value === 'create' ? 'Parent Folder' : 'Story Folder';
});

const folderHint = computed(() => {
  if (isEditing.value) return undefined;
  if (createMode.value === 'create') {
    return `A folder named after the story will be created here with ${structureHint.value}.`;
  }
  return 'Point at the existing story folder on disk.';
});

const folderPlaceholder = computed(() => {
  if (isEditing.value) return '/path/to/story';
  return createMode.value === 'create' ? '/path/to/parent' : '/path/to/existing/story';
});

const panelTitle = computed(() => (isEditing.value ? 'Edit Story' : 'New Story'));
const saveLabel = computed(() => (!isEditing.value && createMode.value === 'create' ? 'Create' : 'Save'));

watch(() => props.story, (s) => {
  if (s) {
    name.value = s.name;
    folder.value = s.folder;
    biblePath.value = s.bible_path || '';
    editId.value = s.id;
    isEditing.value = true;
  } else {
    name.value = '';
    folder.value = '';
    biblePath.value = '';
    editId.value = '';
    isEditing.value = false;
    createMode.value = 'link';
  }
  error.value = '';
}, { immediate: true });

watch(createMode, () => {
  if (!isEditing.value) {
    folder.value = '';
    error.value = '';
  }
});

async function onPickFolder(): Promise<void> {
  try {
    let title = 'Select Story Folder';
    if (!isEditing.value && createMode.value === 'create') {
      title = 'Select Parent Folder for New Story';
    }
    const path = await invoke<string>('pick_manuscript_folder', { title });
    if (path) folder.value = path;
  } catch (e) {
    if (!String(e).includes('No folder')) {
      error.value = String(e);
    }
  }
}

async function onSave(): Promise<void> {
  const trimName = name.value.trim();
  const trimFolder = folder.value.trim();

  if (!trimName) { error.value = 'Please enter a story name.'; return; }
  if (!trimFolder) {
    error.value = createMode.value === 'create' && !isEditing.value
      ? 'Please select a parent folder.'
      : 'Please select a story folder.';
    return;
  }
  error.value = '';

  let result: StoriesResult;
  if (isEditing.value && editId.value) {
    result = await storiesCtx.updateStory(editId.value, trimName, trimFolder, biblePath.value.trim());
  } else if (createMode.value === 'create') {
    result = await storiesCtx.initStory(trimName, trimFolder);
  } else {
    result = await storiesCtx.addStory(trimName, trimFolder);
  }

  if (!result.success) {
    error.value = result.error;
    return;
  }

  const saved = isEditing.value && editId.value
    ? result.stories.find(s => s.id === editId.value)
    : [...result.stories].reverse().find(s => s.name === trimName);
  if (saved) storiesCtx.setActiveStory(saved.id);
  showPanel('analyzer');
}

function onCancel(): void {
  showPanel('analyzer');
}

function onDelete(): void {
  if (!editId.value) return;
  dialog.warning({
    title: 'Remove story',
    content: 'Remove this story from the list? The folder and files will not be deleted.',
    positiveText: 'Remove',
    negativeText: 'Cancel',
    onPositiveClick: async () => {
      const result = await storiesCtx.deleteStory(editId.value);
      if (result.success) {
        showPanel('analyzer');
      } else {
        error.value = result.error;
      }
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
              <strong>Link existing story</strong>
              <div style="font-size: 12px; color: var(--text-muted); margin-top: 2px;">
                Name it and choose the folder(s) that already exist on disk
              </div>
            </n-radio>
            <n-radio value="create">
              <strong>Create empty story</strong>
              <div style="font-size: 12px; color: var(--text-muted); margin-top: 2px;">
                New folder named after the story, using Settings → Folder Structure
                ({{ structureHint }})
              </div>
            </n-radio>
          </n-space>
        </n-radio-group>
      </n-form-item>

      <n-form-item label="Story Name">
        <n-input v-model:value="name" placeholder="My Novel" />
      </n-form-item>

      <n-form-item :label="folderLabel" :feedback="folderHint">
        <n-input-group>
          <n-input
            v-model:value="folder"
            :placeholder="folderPlaceholder"
            readonly
            style="font-family: var(--mono);"
          />
          <n-button @click="onPickFolder">Browse</n-button>
        </n-input-group>
      </n-form-item>

      <n-form-item
        v-if="isEditing || createMode === 'link'"
        label="Story Bible"
        feedback="Override — leave blank to auto-discover from your configured Bible/Characters folders"
      >
        <n-input
          v-model:value="biblePath"
          placeholder="Auto-detected if present in story folder"
        />
      </n-form-item>
    </n-form>

    <n-alert v-if="error" type="error" :show-icon="false" style="margin-top: 8px;">
      {{ error }}
    </n-alert>

    <template #footer>
      <n-space>
        <n-button type="primary" @click="onSave">{{ saveLabel }}</n-button>
        <n-button @click="onCancel">Cancel</n-button>
        <n-button v-if="isEditing" type="error" ghost @click="onDelete">Delete</n-button>
      </n-space>
    </template>
  </FormPanelShell>
</template>

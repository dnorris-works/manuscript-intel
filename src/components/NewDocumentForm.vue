<script setup lang="ts">
import { computed, inject, ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import {
  NForm, NFormItem, NInput, NRadioGroup, NRadio, NButton, NSpace, NAlert,
} from 'naive-ui';
import { storiesKey, settingsKey } from '../injectionKeys';
import { manuscriptActPaths } from '../composables/useSettings';
import FormPanelShell from './FormPanelShell.vue';

const props = defineProps<{
  initialLocation?: string;
}>();

const emit = defineEmits<{
  (e: 'created', path: string, title: string): void;
  (e: 'cancel'): void;
}>();

const storiesCtx = inject(storiesKey)!;
const settingsCtx = inject(settingsKey)!;

interface DocTypeOption {
  id: string;
  label: string;
  path: string;
}

const docTypes = computed<DocTypeOption[]>(() => {
  const s = settingsCtx.folderStructure.value;
  const types: DocTypeOption[] = manuscriptActPaths(s).map((path, i) => ({
    id: `act-${i + 1}`,
    label: `Chapter (${path.split('/').pop()})`,
    path,
  }));
  types.push(
    { id: 'bible', label: 'Bible', path: s.bible || 'Bible' },
    { id: 'characters', label: 'Character', path: s.characters || 'Characters' },
    { id: 'locations', label: 'Location', path: s.locations || 'Locations' },
  );
  for (const extra of s.extra || []) {
    const path = extra.trim();
    if (!path) continue;
    const label = path.split(/[/\\]/).filter(Boolean).pop() || path;
    types.push({ id: `extra:${path}`, label, path });
  }
  return types;
});

const name = ref('');
const docTypeId = ref('act-1');
const location = ref('Manuscript/Act-1');
const error = ref('');
const saving = ref(false);
const locationFromFolder = ref(false);

const selectedType = computed(() =>
  docTypes.value.find(t => t.id === docTypeId.value) || docTypes.value[0]
);

const namePlaceholder = computed(() => {
  const id = selectedType.value?.id || '';
  if (id.startsWith('act-')) return 'Chapter 1';
  switch (id) {
    case 'bible': return 'World Rules';
    case 'characters': return 'Protagonist';
    case 'locations': return 'Primary Setting';
    default: return 'Untitled';
  }
});

function matchTypeForLocation(loc: string): string {
  const norm = loc.replace(/\\/g, '/').replace(/^\/+|\/+$/g, '');
  let best = docTypes.value[0]?.id || 'act-1';
  let bestLen = -1;
  for (const t of docTypes.value) {
    const p = t.path.replace(/\\/g, '/');
    if (norm === p || norm.startsWith(p + '/')) {
      if (p.length > bestLen) {
        best = t.id;
        bestLen = p.length;
      }
    }
  }
  return best;
}

function applyInitialLocation(loc: string | undefined): void {
  name.value = '';
  error.value = '';
  saving.value = false;
  if (loc && loc.trim()) {
    location.value = loc.trim().replace(/\\/g, '/');
    docTypeId.value = matchTypeForLocation(location.value);
    locationFromFolder.value = true;
  } else {
    locationFromFolder.value = false;
    docTypeId.value = 'act-1';
    location.value = selectedType.value?.path || 'Manuscript/Act-1';
  }
}

watch(() => props.initialLocation, (loc) => {
  applyInitialLocation(loc);
}, { immediate: true });

watch(docTypes, (types) => {
  if (!types.some(t => t.id === docTypeId.value) && types[0]) {
    docTypeId.value = types[0].id;
  }
});

watch(docTypeId, (id) => {
  if (locationFromFolder.value) return;
  const t = docTypes.value.find(x => x.id === id);
  if (t) location.value = t.path;
});

function onLocationEdit(): void {
  locationFromFolder.value = true;
}

async function onCreate(): Promise<void> {
  const folder = storiesCtx.activeFolder.value;
  if (!folder) {
    error.value = 'Select a story first.';
    return;
  }
  const trimName = name.value.trim();
  const trimLoc = location.value.trim();
  if (!trimName) { error.value = 'Please enter a document name.'; return; }
  if (!trimLoc) { error.value = 'Please choose a location.'; return; }

  error.value = '';
  saving.value = true;
  try {
    const result = await invoke<{ path: string; title: string }>('create_story_document', {
      request: {
        story_folder: folder,
        name: trimName,
        location: trimLoc,
      },
    });
    emit('created', result.path, result.title);
  } catch (e) {
    error.value = String(e);
  } finally {
    saving.value = false;
  }
}

function onCancel(): void {
  emit('cancel');
}
</script>

<template>
  <FormPanelShell title="New Document">
    <n-form label-placement="top">
      <n-form-item label="Name">
        <n-input
          v-model:value="name"
          :placeholder="namePlaceholder"
          @keydown.enter="onCreate"
        />
      </n-form-item>

      <n-form-item label="Type">
        <n-radio-group v-model:value="docTypeId">
          <n-space vertical>
            <n-radio
              v-for="t in docTypes"
              :key="t.id"
              :value="t.id"
            >
              <strong>{{ t.label }}</strong>
              <div style="font-size: 11px; color: var(--text-muted); font-family: var(--mono);">
                {{ t.path }}
              </div>
            </n-radio>
          </n-space>
        </n-radio-group>
      </n-form-item>

      <n-form-item label="Location" feedback="Relative to the story folder (editable)">
        <n-input
          v-model:value="location"
          placeholder="Manuscript"
          style="font-family: var(--mono);"
          @input="onLocationEdit"
        />
      </n-form-item>
    </n-form>

    <n-alert v-if="error" type="error" :show-icon="false" style="margin-top: 8px;">
      {{ error }}
    </n-alert>

    <template #footer>
      <n-space>
        <n-button type="primary" :loading="saving" @click="onCreate">Create</n-button>
        <n-button @click="onCancel">Cancel</n-button>
      </n-space>
    </template>
  </FormPanelShell>
</template>

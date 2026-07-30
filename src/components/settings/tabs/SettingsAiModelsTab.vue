<script setup lang="ts">
import { inject, computed } from 'vue';
import {
  NForm, NFormItem, NInput, NSelect, NButton, NText, NSpace,
  NCheckbox, NCard, NAlert, NTag, NButtonGroup, NDivider,
} from 'naive-ui';
import { settingsKey } from '../../../injectionKeys';
import type { ModelAssignments } from '../../../composables/useSettings';
import { useReportTypes } from '../../../composables/useReportTypes';
import { useSettingsModels } from '../../../composables/useSettingsModels';

const settingsCtx = inject(settingsKey)!;
const { reportTypes, loadReportTypes } = useReportTypes();
loadReportTypes();

const {
  modelSort,
  pricedOnly,
  freeOnly,
  modelFetchStatus,
  sortedModels,
  filteredModels,
  modelSelectOptions,
  fnSelectOptions,
} = useSettingsModels(settingsCtx.models, reportTypes);

const apiKeyMissing = computed(() => !settingsCtx.apiKey.value.trim());

const defaultModelOptions = computed(() => {
  if (settingsCtx.models.value.length === 0) {
    return [{ label: 'No models loaded', value: '', disabled: true }];
  }
  if (filteredModels.value.length === 0) {
    return [{ label: 'No models match current filters', value: '', disabled: true }];
  }
  return modelSelectOptions.value;
});

function assignmentOptions(fnKey: string) {
  const opts = fnSelectOptions(fnKey);
  return [{ label: '(Use default)', value: '' }, ...opts];
}

async function onFetchModels(): Promise<void> {
  modelFetchStatus.value = 'Fetching models...';
  const result = await settingsCtx.fetchModels();
  if (result.success) {
    modelFetchStatus.value = `${settingsCtx.models.value.length} models loaded.`;
  } else {
    modelFetchStatus.value = result.error;
  }
}

const ASSIGNMENTS: { key: keyof ModelAssignments; label: string; hint: string }[] = [
  { key: 'summaries', label: 'Chapter Summaries', hint: 'Per-chapter genre signal extraction. Fast model is usually fine.' },
  { key: 'genre', label: 'Genre Analysis', hint: 'Classification task. Mid-tier model is sufficient.' },
  { key: 'keywords', label: 'Keywords & Categories', hint: 'Short structured output. Fast model works — speed over depth.' },
  { key: 'continuity', label: 'Continuity Check', hint: 'Needs reasoning to spot contradictions across chapters.' },
  { key: 'showDontTell', label: "Show Don't Tell", hint: 'Literary judgment — prose craft. Use a strong model.' },
  { key: 'aiIsms', label: 'AI-isms', hint: 'Spots synthetic / template-sounding prose. Use a strong model.' },
  { key: 'prose', label: 'Prose Suggestions', hint: 'Creative rewriting — highest-quality model recommended.' },
];
</script>

<template>
  <n-space vertical :size="16">
    <n-alert type="info" :bordered="false">
      All AI features use <strong>TokenMix</strong>, including per-chapter genre summaries used for publishing analysis.
    </n-alert>

    <n-form label-placement="top">
      <n-form-item>
        <template #label>
          <n-space align="center" :size="8">
            <span>TokenMix API Key</span>
            <n-tag size="small" type="warning" round>Required</n-tag>
          </n-space>
        </template>
        <n-input
          v-model:value="settingsCtx.apiKey.value"
          type="password"
          show-password-on="click"
          placeholder="Enter your TokenMix API key"
          :status="apiKeyMissing ? 'warning' : undefined"
        />
      </n-form-item>

      <n-form-item label="Default Model" feedback="Fetch models first, then assign each function below.">
        <n-space vertical>
          <n-space>
            <n-select
              v-model:value="settingsCtx.activeModelAssignments.value.default"
              :options="defaultModelOptions"
              filterable
              style="min-width: 320px;"
            />
            <n-button @click="onFetchModels">Fetch Models</n-button>
          </n-space>
          <n-text v-if="modelFetchStatus" depth="3">{{ modelFetchStatus }}</n-text>
        </n-space>
      </n-form-item>
    </n-form>

    <template v-if="sortedModels.length > 0">
      <n-space align="center">
        <n-text depth="3">Sort</n-text>
        <n-button-group>
          <n-button
            :type="modelSort === 'price' ? 'primary' : 'default'"
            @click="modelSort = 'price'"
          >Price</n-button>
          <n-button
            :type="modelSort === 'provider' ? 'primary' : 'default'"
            @click="modelSort = 'provider'"
          >Provider</n-button>
        </n-button-group>
        <n-checkbox v-model:checked="pricedOnly" :disabled="freeOnly">Priced only</n-checkbox>
        <n-checkbox v-model:checked="freeOnly">Free only</n-checkbox>
        <n-text depth="3">{{ filteredModels.length }} shown</n-text>
      </n-space>

      <n-card title="Model per function" size="small">
        <n-alert v-if="filteredModels.length === 0" type="warning" :bordered="false">
          No models match the current filters.
        </n-alert>

        <n-alert type="default" :bordered="false">
          <strong>Chapter Fingerprints</strong> — instant deterministic scan in Rust (no AI).
        </n-alert>

        <n-divider />

        <n-form label-placement="top">
          <n-form-item
            v-for="item in ASSIGNMENTS"
            :key="item.key"
            :label="item.label"
            :feedback="item.hint"
          >
            <n-select
              v-model:value="settingsCtx.activeModelAssignments.value[item.key]"
              :options="assignmentOptions(item.key)"
              filterable
            />
          </n-form-item>
        </n-form>
      </n-card>
    </template>
  </n-space>
</template>

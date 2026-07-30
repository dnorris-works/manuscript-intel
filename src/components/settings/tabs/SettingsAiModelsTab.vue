<script setup lang="ts">
import { inject, computed } from 'vue';
import {
  NForm, NFormItem, NInput, NSelect, NButton, NText, NSpace,
  NCheckbox, NCard, NDivider, NTag,
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
  { key: 'genre', label: 'Genre Analysis', hint: 'Classification task. Mid-tier model is sufficient.' },
  { key: 'keywords', label: 'Keywords & Categories', hint: 'Short structured output. Fast model works — speed over depth.' },
  { key: 'continuity', label: 'Continuity Check', hint: 'Needs reasoning ability to spot contradictions across chapters. Use a capable model (e.g. GPT-4o or another high-reasoning model).' },
  { key: 'showDontTell', label: "Show Don't Tell", hint: 'Literary judgment — needs to understand prose craft. Use a strong model (e.g. GPT-4o or another high-reasoning model).' },
  { key: 'aiIsms', label: 'AI-isms', hint: 'Literary judgment — spots synthetic / template-sounding prose. Use a strong model (e.g. GPT-4o or another high-reasoning model).' },
  { key: 'prose', label: 'Prose Suggestions', hint: 'Creative rewriting. Use the highest-quality model you have — this writes prose the author will paste into their manuscript.' },
];
</script>

<template>
  <n-form label-placement="top" :show-feedback="false">
    <n-text depth="3" style="display: block; margin-bottom: 12px;">
      All AI features use <strong>TokenMix</strong>. Chapter fingerprints are scanned locally in Rust (no AI).
    </n-text>

    <n-form-item>
      <template #label>
        <n-space align="center" :size="8">
          <span>TokenMix API Key</span>
          <n-tag size="small" type="warning" :bordered="false">Required</n-tag>
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

    <n-form-item label="Default Model">
      <template #label>
        <span>Default Model</span>
        <n-text depth="3" style="font-size: 11px; display: block; font-weight: 400;">
          Fetch models first, then assign each function below.
        </n-text>
      </template>
      <n-space align="center" style="width: 100%;">
        <n-select
          v-model:value="settingsCtx.activeModelAssignments.value.default"
          :options="defaultModelOptions"
          filterable
          style="flex: 1; min-width: 200px;"
        />
        <n-button size="small" @click="onFetchModels">Fetch Models</n-button>
      </n-space>
      <n-text v-if="modelFetchStatus" depth="3" style="font-size: 12px; margin-top: 6px;">
        {{ modelFetchStatus }}
      </n-text>
    </n-form-item>

    <template v-if="sortedModels.length > 0">
      <n-space align="center" :size="8">
        <n-text depth="3" style="font-size: 11px;">Sort:</n-text>
        <n-button
          size="tiny"
          :type="modelSort === 'price' ? 'primary' : 'default'"
          @click="modelSort = 'price'"
        >Price</n-button>
        <n-button
          size="tiny"
          :type="modelSort === 'provider' ? 'primary' : 'default'"
          @click="modelSort = 'provider'"
        >Provider</n-button>
      </n-space>

      <n-space align="center" :size="12" style="margin-top: 8px;">
        <n-checkbox v-model:checked="pricedOnly" :disabled="freeOnly">Priced only</n-checkbox>
        <n-checkbox v-model:checked="freeOnly">Free only</n-checkbox>
        <n-text depth="3" style="font-size: 11px;">{{ filteredModels.length }} shown</n-text>
      </n-space>

      <n-card size="small" style="margin-top: 12px;" title="Model per function">
        <n-text v-if="filteredModels.length === 0" depth="3" style="font-size: 12px; display: block; margin-bottom: 10px;">
          No models match the current filters. Disable filters to see all models.
        </n-text>

        <div class="assignment-note">
          <strong>Chapter Fingerprints</strong>
          <n-text depth="3" style="font-size: 11px; display: block; font-style: italic;">
            Instant deterministic scan in Rust — no AI model used.
          </n-text>
        </div>

        <n-divider style="margin: 12px 0;" />

        <div
          v-for="item in ASSIGNMENTS"
          :key="item.key"
          class="assignment-row"
        >
          <div class="assignment-label">
            <strong>{{ item.label }}</strong>
            <n-text depth="3" style="font-size: 11px; display: block; font-style: italic;">
              {{ item.hint }}
            </n-text>
          </div>
          <n-select
            v-model:value="settingsCtx.activeModelAssignments.value[item.key]"
            :options="assignmentOptions(item.key)"
            filterable
            style="width: 100%;"
          />
        </div>
      </n-card>
    </template>
  </n-form>
</template>

<style scoped>
.assignment-note {
  padding: 4px 0;
}

.assignment-row {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 8px 0;
  border-bottom: 1px solid var(--border);
}

.assignment-row:last-child {
  border-bottom: none;
  padding-bottom: 0;
}

.assignment-label strong {
  font-size: 13px;
}
</style>

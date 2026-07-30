<script setup lang="ts">
import { inject, ref, watch, computed } from 'vue';
import {
  NForm, NFormItem, NInput, NInputNumber, NSelect, NButton, NSpace, NAlert,
} from 'naive-ui';
import { storiesKey, campaignsKey } from '../../injectionKeys';
import FormPanelShell from '../FormPanelShell.vue';

const props = defineProps<{
  campaignId: number | null;
}>();

const emit = defineEmits<{
  (e: 'saved', id: number): void;
  (e: 'cancel'): void;
}>();

const storiesCtx = inject(storiesKey)!;
const campaignsCtx = inject(campaignsKey)!;

const name = ref('');
const platform = ref('meta');
const objective = ref('awareness');
const status = ref('draft');
const budget = ref<number | null>(null);
const budgetPeriod = ref('lifetime');
const startDate = ref('');
const endDate = ref('');
const targetAudience = ref('');
const landingPageId = ref<number | null>(null);
const platformAccountId = ref<number | null>(null);
const notes = ref('');
const error = ref('');
const saving = ref(false);

const isEditing = computed(() => props.campaignId != null && props.campaignId > 0);
const storyFolder = computed(() => storiesCtx.activeFolder.value);
const panelTitle = computed(() => (isEditing.value ? 'Edit Campaign' : 'New Campaign'));

const PLATFORMS = ['meta', 'amazon', 'tiktok', 'google', 'bookbub', 'other'];
const OBJECTIVES = ['awareness', 'conversion', 'traffic', 'engagement'];
const STATUSES = ['draft', 'active', 'paused', 'archived'];

const platformOptions = PLATFORMS.map(p => ({ label: p, value: p }));
const objectiveOptions = OBJECTIVES.map(o => ({ label: o, value: o }));
const statusOptions = STATUSES.map(s => ({ label: s, value: s }));
const budgetPeriodOptions = [
  { label: 'Daily', value: 'daily' },
  { label: 'Lifetime', value: 'lifetime' },
];

const platformAccountOptions = computed(() =>
  campaignsCtx.platformAccounts.value.map(a => ({
    label: `${a.platform} — ${a.account_id || 'no ID'}`,
    value: a.id,
  })),
);

const landingPageOptions = computed(() =>
  campaignsCtx.landingPages.value.map(lp => ({
    label: `${lp.name} — ${lp.url}`,
    value: lp.id,
  })),
);

watch(() => props.campaignId, async (id) => {
  error.value = '';
  if (!id) {
    name.value = '';
    platform.value = 'meta';
    objective.value = 'awareness';
    status.value = 'draft';
    budget.value = null;
    budgetPeriod.value = 'lifetime';
    startDate.value = '';
    endDate.value = '';
    targetAudience.value = '';
    landingPageId.value = null;
    platformAccountId.value = null;
    notes.value = '';
    return;
  }
  const detail = await campaignsCtx.loadCampaignDetail(id);
  if (!detail) return;
  const c = detail.campaign;
  name.value = c.name;
  platform.value = c.platform || 'meta';
  objective.value = c.objective || 'awareness';
  status.value = c.status || 'draft';
  budget.value = c.budget;
  budgetPeriod.value = c.budget_period || 'lifetime';
  startDate.value = c.start_date || '';
  endDate.value = c.end_date || '';
  targetAudience.value = c.target_audience || '';
  landingPageId.value = c.landing_page_id;
  platformAccountId.value = c.platform_account_id;
  notes.value = c.notes || '';
}, { immediate: true });

watch(storyFolder, (folder) => {
  if (folder) void campaignsCtx.loadLandingPages(folder);
}, { immediate: true });

async function onSave(): Promise<void> {
  const trimName = name.value.trim();
  if (!trimName) { error.value = 'Campaign name is required.'; return; }
  if (!storyFolder.value) { error.value = 'Select a story first.'; return; }
  saving.value = true;
  error.value = '';
  try {
    if (isEditing.value && props.campaignId) {
      const result = await campaignsCtx.updateCampaign({
        id: props.campaignId,
        name: trimName,
        platform: platform.value,
        platform_account_id: platformAccountId.value,
        objective: objective.value,
        status: status.value,
        budget: budget.value,
        budget_period: budgetPeriod.value,
        start_date: startDate.value,
        end_date: endDate.value,
        target_audience: targetAudience.value,
        landing_page_id: landingPageId.value,
        notes: notes.value,
      });
      if (!result.success) { error.value = result.error; return; }
      emit('saved', props.campaignId);
    } else {
      const result = await campaignsCtx.createCampaign(storyFolder.value, trimName, platform.value, objective.value);
      if (!result.success) { error.value = result.error; return; }
      await campaignsCtx.updateCampaign({
        id: result.id,
        name: trimName,
        platform: platform.value,
        platform_account_id: platformAccountId.value,
        objective: objective.value,
        status: status.value,
        budget: budget.value,
        budget_period: budgetPeriod.value,
        start_date: startDate.value,
        end_date: endDate.value,
        target_audience: targetAudience.value,
        landing_page_id: landingPageId.value,
        notes: notes.value,
      });
      await campaignsCtx.loadCampaigns(storyFolder.value);
      emit('saved', result.id);
    }
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <FormPanelShell :title="panelTitle">
    <n-form label-placement="top">
      <n-form-item label="Name">
        <n-input v-model:value="name" placeholder="e.g. Book 1 Launch — Meta" />
      </n-form-item>

      <n-form-item label="Platform">
        <n-select v-model:value="platform" :options="platformOptions" />
      </n-form-item>

      <n-form-item label="Objective">
        <n-select v-model:value="objective" :options="objectiveOptions" />
      </n-form-item>

      <n-form-item label="Status">
        <n-select v-model:value="status" :options="statusOptions" />
      </n-form-item>

      <n-form-item label="Platform Account">
        <n-select v-model:value="platformAccountId" clearable :options="platformAccountOptions" />
      </n-form-item>

      <n-form-item label="Budget">
        <n-space>
          <n-input-number v-model:value="budget" :min="0" :step="0.01" placeholder="0.00" style="width: 160px;" />
          <n-select v-model:value="budgetPeriod" :options="budgetPeriodOptions" style="width: 140px;" />
        </n-space>
      </n-form-item>

      <n-form-item label="Start Date">
        <n-input v-model:value="startDate" placeholder="YYYY-MM-DD" />
      </n-form-item>

      <n-form-item label="End Date">
        <n-input v-model:value="endDate" placeholder="YYYY-MM-DD" />
      </n-form-item>

      <n-form-item label="Landing Page">
        <n-select v-model:value="landingPageId" clearable :options="landingPageOptions" />
      </n-form-item>

      <n-form-item label="Target Audience">
        <n-input
          v-model:value="targetAudience"
          type="textarea"
          :rows="3"
          placeholder="Demographics, interests, lookalikes…"
        />
      </n-form-item>

      <n-form-item label="Notes">
        <n-input v-model:value="notes" type="textarea" :rows="2" />
      </n-form-item>
    </n-form>

    <n-alert v-if="error" type="error" :show-icon="false" style="margin-top: 8px;">
      {{ error }}
    </n-alert>

    <template #footer>
      <n-space>
        <n-button type="primary" :loading="saving" @click="onSave">Save</n-button>
        <n-button @click="emit('cancel')">Cancel</n-button>
      </n-space>
    </template>
  </FormPanelShell>
</template>

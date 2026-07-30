<script setup lang="ts">
import { inject, ref, watch, computed, h } from 'vue';
import {
  NPageHeader, NScrollbar, NTabs, NTabPane, NButton, NSpace, NAlert, NSpin,
  NCard, NForm, NFormItem, NInput, NInputNumber, NSelect, NDescriptions,
  NDescriptionsItem, NDataTable, NTag, NText, useDialog,
} from 'naive-ui';
import type { DataTableColumns } from 'naive-ui';
import { storiesKey, campaignsKey } from '../../injectionKeys';

const props = defineProps<{
  campaignId: number;
}>();

const emit = defineEmits<{
  (e: 'back'): void;
  (e: 'edit'): void;
}>();

const storiesCtx = inject(storiesKey)!;
const campaignsCtx = inject(campaignsKey)!;
const dialog = useDialog();

type DetailTab = 'overview' | 'creatives' | 'metrics' | 'spend' | 'landing' | 'audience';
const activeTab = ref<DetailTab>('overview');
const loading = ref(false);
const error = ref('');

const detail = computed(() => campaignsCtx.campaignDetail.value);
const campaign = computed(() => detail.value?.campaign);
const storyFolder = computed(() => storiesCtx.activeFolder.value);

const tabs = [
  { id: 'overview' as const, label: 'Overview' },
  { id: 'creatives' as const, label: 'Creatives' },
  { id: 'metrics' as const, label: 'Metrics' },
  { id: 'spend' as const, label: 'Spend' },
  { id: 'landing' as const, label: 'Landing Page' },
  { id: 'audience' as const, label: 'Audience' },
];

const showCreativeForm = ref(false);
const editingCreativeId = ref<number | null>(null);
const crName = ref('');
const crType = ref('video');
const crVersion = ref('v1');
const crFormat = ref('');
const crStatus = ref('draft');
const crAssetPath = ref('');
const crBodyText = ref('');
const crNotes = ref('');

const showMetricsForm = ref(false);
const mDate = ref('');
const mCreativeId = ref<number | null>(null);
const mImpressions = ref(0);
const mClicks = ref(0);
const mConversions = ref(0);
const mCtr = ref(0);
const mCpc = ref(0);
const mCpa = ref(0);
const mSpend = ref(0);
const mNotes = ref('');

const showSpendForm = ref(false);
const sPlatform = ref('');
const sAmount = ref(0);
const sDate = ref('');
const sNotes = ref('');

const showLandingForm = ref(false);
const lpName = ref('');
const lpUrl = ref('');
const lpConversion = ref<number | null>(null);
const lpNotes = ref('');

const showAudienceForm = ref(false);
const editingAudienceId = ref<number | null>(null);
const auLabel = ref('');
const auDemographics = ref('');
const auInterests = ref('');
const auLookalike = ref('');
const auOutcome = ref('untested');
const auNotes = ref('');

const creativeTypeOptions = [
  { label: 'Video', value: 'video' },
  { label: 'Thumbnail', value: 'thumbnail' },
  { label: 'Hook', value: 'hook' },
  { label: 'Caption', value: 'caption' },
  { label: 'CTA', value: 'cta' },
];
const creativeStatusOptions = [
  { label: 'Draft', value: 'draft' },
  { label: 'Live', value: 'live' },
  { label: 'Paused', value: 'paused' },
  { label: 'Retired', value: 'retired' },
];
const outcomeOptions = [
  { label: 'Untested', value: 'untested' },
  { label: 'Worked', value: 'worked' },
  { label: 'Poor', value: 'poor' },
];

const creativeSelectOptions = computed(() =>
  (detail.value?.creatives || []).map(c => ({ label: c.name, value: c.id })),
);

const snapshotColumns: DataTableColumns = [
  { title: 'Date', key: 'snapshot_date' },
  { title: 'Impr.', key: 'impressions' },
  { title: 'Clicks', key: 'clicks' },
  { title: 'CTR', key: 'ctr', render: (row) => `${row.ctr}%` },
  { title: 'Conv.', key: 'conversions' },
  { title: 'CPA', key: 'cpa', render: (row) => `$${row.cpa}` },
  { title: 'Spend', key: 'spend', render: (row) => `$${row.spend}` },
  {
    title: '',
    key: 'actions',
    width: 48,
    render: (row) => h(
      NButton,
      { size: 'small', quaternary: true, type: 'error', onClick: () => confirmRemoveSnapshot(row.id as number) },
      { default: () => '×' },
    ),
  },
];

const spendColumns: DataTableColumns = [
  { title: 'Date', key: 'spent_at' },
  { title: 'Platform', key: 'platform' },
  { title: 'Amount', key: 'amount', render: (row) => `$${(row.amount as number).toFixed(2)}` },
  { title: 'Notes', key: 'notes' },
  {
    title: '',
    key: 'actions',
    width: 48,
    render: (row) => h(
      NButton,
      { size: 'small', quaternary: true, type: 'error', onClick: () => confirmRemoveSpend(row.id as number) },
      { default: () => '×' },
    ),
  },
];

async function reload(): Promise<void> {
  loading.value = true;
  error.value = '';
  try {
    await campaignsCtx.loadCampaignDetail(props.campaignId);
    if (storyFolder.value) await campaignsCtx.loadLandingPages(storyFolder.value);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

watch(() => props.campaignId, () => { void reload(); }, { immediate: true });

function resetCreativeForm(): void {
  showCreativeForm.value = false;
  editingCreativeId.value = null;
  crName.value = '';
  crType.value = 'video';
  crVersion.value = 'v1';
  crFormat.value = '';
  crStatus.value = 'draft';
  crAssetPath.value = '';
  crBodyText.value = '';
  crNotes.value = '';
}

function editCreative(c: {
  id: number; name: string; creative_type: string; version: string;
  platform_format: string; status: string; asset_path: string; body_text: string; notes: string;
}): void {
  editingCreativeId.value = c.id;
  crName.value = c.name;
  crType.value = c.creative_type;
  crVersion.value = c.version;
  crFormat.value = c.platform_format;
  crStatus.value = c.status;
  crAssetPath.value = c.asset_path;
  crBodyText.value = c.body_text;
  crNotes.value = c.notes;
  showCreativeForm.value = true;
}

async function saveCreative(): Promise<void> {
  if (!crName.value.trim()) return;
  if (editingCreativeId.value) {
    await campaignsCtx.updateCreative({
      id: editingCreativeId.value,
      campaign_id: props.campaignId,
      name: crName.value.trim(),
      creative_type: crType.value,
      version: crVersion.value,
      platform_format: crFormat.value,
      status: crStatus.value,
      asset_path: crAssetPath.value,
      body_text: crBodyText.value,
      notes: crNotes.value,
    });
  } else {
    await campaignsCtx.createCreative({
      campaign_id: props.campaignId,
      name: crName.value.trim(),
      creative_type: crType.value,
      version: crVersion.value,
      platform_format: crFormat.value,
      status: crStatus.value,
      asset_path: crAssetPath.value,
      body_text: crBodyText.value,
      notes: crNotes.value,
    });
  }
  resetCreativeForm();
  await reload();
}

function confirmRemoveCreative(id: number): void {
  dialog.warning({
    title: 'Delete creative',
    content: 'Delete this creative?',
    positiveText: 'Delete',
    negativeText: 'Cancel',
    onPositiveClick: async () => {
      await campaignsCtx.deleteCreative(id);
      await reload();
    },
  });
}

async function saveMetrics(): Promise<void> {
  if (!mDate.value) return;
  await campaignsCtx.addPerformanceSnapshot({
    campaign_id: props.campaignId,
    creative_id: mCreativeId.value,
    snapshot_date: mDate.value,
    impressions: mImpressions.value,
    clicks: mClicks.value,
    conversions: mConversions.value,
    ctr: mCtr.value,
    cpc: mCpc.value,
    cpa: mCpa.value,
    spend: mSpend.value,
    notes: mNotes.value,
  });
  showMetricsForm.value = false;
  await reload();
}

function confirmRemoveSnapshot(id: number): void {
  dialog.warning({
    title: 'Delete snapshot',
    content: 'Delete this snapshot?',
    positiveText: 'Delete',
    negativeText: 'Cancel',
    onPositiveClick: async () => {
      await campaignsCtx.deletePerformanceSnapshot(id);
      await reload();
    },
  });
}

async function saveSpend(): Promise<void> {
  if (!sDate.value || sAmount.value <= 0) return;
  await campaignsCtx.addSpendEntry({
    campaign_id: props.campaignId,
    platform: sPlatform.value || campaign.value?.platform || '',
    amount: sAmount.value,
    spent_at: sDate.value,
    notes: sNotes.value,
  });
  showSpendForm.value = false;
  await reload();
}

function confirmRemoveSpend(id: number): void {
  dialog.warning({
    title: 'Delete spend entry',
    content: 'Delete this spend entry?',
    positiveText: 'Delete',
    negativeText: 'Cancel',
    onPositiveClick: async () => {
      await campaignsCtx.deleteSpendEntry(id);
      await reload();
    },
  });
}

async function saveLanding(): Promise<void> {
  if (!storyFolder.value || !lpName.value.trim()) return;
  const result = await campaignsCtx.createLandingPage({
    story_folder: storyFolder.value,
    name: lpName.value.trim(),
    url: lpUrl.value,
    conversion_rate: lpConversion.value,
    notes: lpNotes.value,
  });
  if (result.success && campaign.value) {
    await campaignsCtx.updateCampaign({
      id: props.campaignId,
      name: campaign.value.name,
      platform: campaign.value.platform,
      platform_account_id: campaign.value.platform_account_id,
      objective: campaign.value.objective,
      status: campaign.value.status,
      budget: campaign.value.budget,
      budget_period: campaign.value.budget_period,
      start_date: campaign.value.start_date,
      end_date: campaign.value.end_date,
      target_audience: campaign.value.target_audience,
      landing_page_id: result.id,
      notes: campaign.value.notes,
    });
    showLandingForm.value = false;
    await reload();
  }
}

function resetAudienceForm(): void {
  showAudienceForm.value = false;
  editingAudienceId.value = null;
  auLabel.value = '';
  auDemographics.value = '';
  auInterests.value = '';
  auLookalike.value = '';
  auOutcome.value = 'untested';
  auNotes.value = '';
}

async function saveAudience(): Promise<void> {
  if (editingAudienceId.value) {
    await campaignsCtx.updateAudienceNote({
      id: editingAudienceId.value,
      campaign_id: props.campaignId,
      label: auLabel.value,
      demographics: auDemographics.value,
      interests: auInterests.value,
      lookalike_notes: auLookalike.value,
      outcome: auOutcome.value,
      notes: auNotes.value,
    });
  } else {
    await campaignsCtx.addAudienceNote({
      campaign_id: props.campaignId,
      label: auLabel.value,
      demographics: auDemographics.value,
      interests: auInterests.value,
      lookalike_notes: auLookalike.value,
      outcome: auOutcome.value,
      notes: auNotes.value,
    });
  }
  resetAudienceForm();
  await reload();
}

function confirmRemoveAudience(id: number): void {
  dialog.warning({
    title: 'Delete audience note',
    content: 'Delete this audience note?',
    positiveText: 'Delete',
    negativeText: 'Cancel',
    onPositiveClick: async () => {
      await campaignsCtx.deleteAudienceNote(id);
      await reload();
    },
  });
}

function onDeleteCampaign(): void {
  dialog.warning({
    title: 'Delete campaign',
    content: 'Delete this campaign and all its data?',
    positiveText: 'Delete',
    negativeText: 'Cancel',
    onPositiveClick: async () => {
      await campaignsCtx.deleteCampaign(props.campaignId);
      if (storyFolder.value) await campaignsCtx.loadCampaigns(storyFolder.value);
      emit('back');
    },
  });
}
</script>

<template>
  <div class="detail-root">
    <header class="detail-header">
      <n-page-header :title="campaign?.name || 'Campaign'">
        <template #extra>
          <n-space>
            <n-button @click="emit('back')">Back</n-button>
            <n-button @click="emit('edit')">Edit</n-button>
            <n-button type="error" ghost @click="onDeleteCampaign">Delete</n-button>
          </n-space>
        </template>
      </n-page-header>

      <n-tabs
        v-if="campaign"
        v-model:value="activeTab"
        type="card"
        animated
        class="detail-tabs"
        style="padding: 0 24px;"
      >
        <n-tab-pane
          v-for="tab in tabs"
          :key="tab.id"
          :name="tab.id"
          :tab="tab.label"
          :display-directive="'show'"
        />
      </n-tabs>
    </header>

    <n-scrollbar class="detail-scroll">
      <div class="detail-body">
        <n-spin v-if="loading" />
        <n-alert v-else-if="error" type="error">{{ error }}</n-alert>

        <template v-else-if="campaign">
          <div v-if="activeTab === 'overview'">
            <n-descriptions :column="1" label-placement="left" bordered size="small">
              <n-descriptions-item label="Platform">{{ campaign.platform }}</n-descriptions-item>
              <n-descriptions-item label="Objective">{{ campaign.objective }}</n-descriptions-item>
              <n-descriptions-item label="Status">{{ campaign.status }}</n-descriptions-item>
              <n-descriptions-item label="Budget">
                {{ campaign.budget != null ? `$${campaign.budget} (${campaign.budget_period})` : '—' }}
              </n-descriptions-item>
              <n-descriptions-item label="Dates">
                {{ campaign.start_date || '—' }} – {{ campaign.end_date || '—' }}
              </n-descriptions-item>
              <n-descriptions-item label="Total Spend">
                ${{ (campaign.total_spend || 0).toFixed(2) }}
              </n-descriptions-item>
              <n-descriptions-item v-if="campaign.target_audience" label="Audience">
                {{ campaign.target_audience }}
              </n-descriptions-item>
              <n-descriptions-item v-if="detail?.landing_page" label="Landing Page">
                <a :href="detail.landing_page.url" target="_blank" rel="noopener">
                  {{ detail.landing_page.name }}
                </a>
              </n-descriptions-item>
              <n-descriptions-item v-if="campaign.notes" label="Notes">
                {{ campaign.notes }}
              </n-descriptions-item>
            </n-descriptions>
          </div>

          <div v-else-if="activeTab === 'creatives'">
            <n-space style="margin-bottom: 12px;">
              <n-button size="small" type="primary" @click="showCreativeForm = true; editingCreativeId = null">
                Add Creative
              </n-button>
            </n-space>

            <n-card v-if="showCreativeForm" size="small" style="margin-bottom: 12px;">
              <n-form label-placement="top">
                <n-form-item label="Name"><n-input v-model:value="crName" placeholder="Name" /></n-form-item>
                <n-form-item label="Type"><n-select v-model:value="crType" :options="creativeTypeOptions" /></n-form-item>
                <n-form-item label="Version"><n-input v-model:value="crVersion" placeholder="v1" /></n-form-item>
                <n-form-item label="Format"><n-input v-model:value="crFormat" placeholder="Platform format" /></n-form-item>
                <n-form-item label="Status"><n-select v-model:value="crStatus" :options="creativeStatusOptions" /></n-form-item>
                <n-form-item label="Asset path"><n-input v-model:value="crAssetPath" placeholder="Asset file path" /></n-form-item>
                <n-form-item label="Copy"><n-input v-model:value="crBodyText" type="textarea" :rows="2" placeholder="Copy / caption / CTA text" /></n-form-item>
              </n-form>
              <n-space>
                <n-button size="small" type="primary" @click="saveCreative">Save</n-button>
                <n-button size="small" @click="resetCreativeForm">Cancel</n-button>
              </n-space>
            </n-card>

            <n-text v-if="!detail?.creatives.length" depth="3">No creatives yet.</n-text>
            <n-space v-else vertical :size="8">
              <n-card v-for="c in detail.creatives" :key="c.id" size="small">
                <n-space justify="space-between">
                  <n-text strong>{{ c.name }}</n-text>
                  <n-tag size="small">{{ c.status }}</n-tag>
                </n-space>
                <n-text depth="3" style="display: block; font-size: 12px; margin-top: 4px;">
                  {{ c.creative_type }} · {{ c.version }} · {{ c.platform_format }}
                </n-text>
                <n-text v-if="c.body_text" depth="3" style="display: block; font-size: 12px; margin-top: 4px;">
                  {{ c.body_text }}
                </n-text>
                <n-space style="margin-top: 8px;">
                  <n-button size="small" @click="editCreative(c)">Edit</n-button>
                  <n-button size="small" type="error" ghost @click="confirmRemoveCreative(c.id)">Delete</n-button>
                </n-space>
              </n-card>
            </n-space>
          </div>

          <div v-else-if="activeTab === 'metrics'">
            <n-space style="margin-bottom: 12px;">
              <n-button size="small" type="primary" @click="showMetricsForm = true">Log Snapshot</n-button>
            </n-space>

            <n-card v-if="showMetricsForm" size="small" style="margin-bottom: 12px;">
              <n-form label-placement="top">
                <n-form-item label="Date"><n-input v-model:value="mDate" placeholder="YYYY-MM-DD" /></n-form-item>
                <n-form-item label="Creative"><n-select v-model:value="mCreativeId" clearable :options="creativeSelectOptions" placeholder="Campaign-level" /></n-form-item>
                <n-space>
                  <n-form-item label="Impressions"><n-input-number v-model:value="mImpressions" :min="0" style="width: 120px;" /></n-form-item>
                  <n-form-item label="Clicks"><n-input-number v-model:value="mClicks" :min="0" style="width: 120px;" /></n-form-item>
                  <n-form-item label="Conversions"><n-input-number v-model:value="mConversions" :min="0" style="width: 120px;" /></n-form-item>
                </n-space>
                <n-space>
                  <n-form-item label="CTR %"><n-input-number v-model:value="mCtr" :step="0.01" style="width: 100px;" /></n-form-item>
                  <n-form-item label="CPC"><n-input-number v-model:value="mCpc" :step="0.01" style="width: 100px;" /></n-form-item>
                  <n-form-item label="CPA"><n-input-number v-model:value="mCpa" :step="0.01" style="width: 100px;" /></n-form-item>
                  <n-form-item label="Spend"><n-input-number v-model:value="mSpend" :step="0.01" style="width: 100px;" /></n-form-item>
                </n-space>
              </n-form>
              <n-space>
                <n-button size="small" type="primary" @click="saveMetrics">Save</n-button>
                <n-button size="small" @click="showMetricsForm = false">Cancel</n-button>
              </n-space>
            </n-card>

            <n-data-table
              v-if="detail?.snapshots.length"
              :columns="snapshotColumns"
              :data="detail.snapshots"
              :bordered="false"
              size="small"
            />
            <n-text v-else depth="3">No metrics logged yet. Add a weekly snapshot.</n-text>
          </div>

          <div v-else-if="activeTab === 'spend'">
            <n-space style="margin-bottom: 12px;">
              <n-button size="small" type="primary" @click="showSpendForm = true">Log Spend</n-button>
            </n-space>

            <n-card v-if="showSpendForm" size="small" style="margin-bottom: 12px;">
              <n-form label-placement="top">
                <n-form-item label="Platform"><n-input v-model:value="sPlatform" :placeholder="campaign.platform" /></n-form-item>
                <n-form-item label="Amount"><n-input-number v-model:value="sAmount" :min="0" :step="0.01" /></n-form-item>
                <n-form-item label="Date"><n-input v-model:value="sDate" placeholder="YYYY-MM-DD" /></n-form-item>
                <n-form-item label="Notes"><n-input v-model:value="sNotes" placeholder="Notes" /></n-form-item>
              </n-form>
              <n-space>
                <n-button size="small" type="primary" @click="saveSpend">Save</n-button>
                <n-button size="small" @click="showSpendForm = false">Cancel</n-button>
              </n-space>
            </n-card>

            <n-data-table
              v-if="detail?.spend_entries.length"
              :columns="spendColumns"
              :data="detail.spend_entries"
              :bordered="false"
              size="small"
            />
            <n-text v-else depth="3">No spend logged yet.</n-text>
          </div>

          <div v-else-if="activeTab === 'landing'">
            <n-card v-if="detail?.landing_page" size="small" style="margin-bottom: 12px;">
              <n-text strong>{{ detail.landing_page.name }}</n-text>
              <p><a :href="detail.landing_page.url" target="_blank" rel="noopener">{{ detail.landing_page.url }}</a></p>
              <n-text v-if="detail.landing_page.conversion_rate != null" depth="3">
                Conversion: {{ detail.landing_page.conversion_rate }}%
              </n-text>
              <n-text v-if="detail.landing_page.notes" depth="3" style="display: block; margin-top: 4px;">
                {{ detail.landing_page.notes }}
              </n-text>
            </n-card>
            <template v-else>
              <n-text depth="3" style="display: block; margin-bottom: 12px;">No landing page linked.</n-text>
              <n-button size="small" type="primary" @click="showLandingForm = true">Create & Link</n-button>
            </template>

            <n-card v-if="showLandingForm" size="small" style="margin-top: 12px;">
              <n-form label-placement="top">
                <n-form-item label="Name"><n-input v-model:value="lpName" placeholder="Page name" /></n-form-item>
                <n-form-item label="URL"><n-input v-model:value="lpUrl" placeholder="https://…" /></n-form-item>
                <n-form-item label="Conversion %"><n-input-number v-model:value="lpConversion" :step="0.1" /></n-form-item>
                <n-form-item label="Notes"><n-input v-model:value="lpNotes" type="textarea" :rows="2" /></n-form-item>
              </n-form>
              <n-space>
                <n-button size="small" type="primary" @click="saveLanding">Save & Link</n-button>
                <n-button size="small" @click="showLandingForm = false">Cancel</n-button>
              </n-space>
            </n-card>
          </div>

          <div v-else-if="activeTab === 'audience'">
            <n-space style="margin-bottom: 12px;">
              <n-button size="small" type="primary" @click="showAudienceForm = true; editingAudienceId = null">
                Add Note
              </n-button>
            </n-space>

            <n-card v-if="showAudienceForm" size="small" style="margin-bottom: 12px;">
              <n-form label-placement="top">
                <n-form-item label="Label"><n-input v-model:value="auLabel" placeholder="e.g. Lookalike 1%" /></n-form-item>
                <n-form-item label="Demographics"><n-input v-model:value="auDemographics" type="textarea" :rows="2" /></n-form-item>
                <n-form-item label="Interests"><n-input v-model:value="auInterests" type="textarea" :rows="2" /></n-form-item>
                <n-form-item label="Lookalike notes"><n-input v-model:value="auLookalike" type="textarea" :rows="2" /></n-form-item>
                <n-form-item label="Outcome"><n-select v-model:value="auOutcome" :options="outcomeOptions" /></n-form-item>
                <n-form-item label="Notes"><n-input v-model:value="auNotes" type="textarea" :rows="2" /></n-form-item>
              </n-form>
              <n-space>
                <n-button size="small" type="primary" @click="saveAudience">Save</n-button>
                <n-button size="small" @click="resetAudienceForm">Cancel</n-button>
              </n-space>
            </n-card>

            <n-text v-if="!detail?.audience_notes.length" depth="3">No audience tests recorded.</n-text>
            <n-space v-else vertical :size="8">
              <n-card v-for="n in detail.audience_notes" :key="n.id" size="small">
                <n-space justify="space-between">
                  <n-text strong>{{ n.label || 'Untitled' }}</n-text>
                  <n-tag size="small">{{ n.outcome }}</n-tag>
                </n-space>
                <n-text v-if="n.demographics" depth="3" style="display: block; margin-top: 4px; font-size: 12px;">
                  Demographics: {{ n.demographics }}
                </n-text>
                <n-text v-if="n.interests" depth="3" style="display: block; font-size: 12px;">
                  Interests: {{ n.interests }}
                </n-text>
                <n-text v-if="n.lookalike_notes" depth="3" style="display: block; font-size: 12px;">
                  Lookalikes: {{ n.lookalike_notes }}
                </n-text>
                <n-button size="small" type="error" ghost style="margin-top: 8px;" @click="confirmRemoveAudience(n.id)">
                  Delete
                </n-button>
              </n-card>
            </n-space>
          </div>
        </template>
      </div>
    </n-scrollbar>
  </div>
</template>

<style scoped>
.detail-root {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.detail-header {
  flex-shrink: 0;
  padding: 20px 24px 0;
  background: var(--bg);
  border-bottom: 1px solid var(--border);
}

.detail-tabs :deep(.n-tabs-pane-wrapper) {
  display: none;
}

.detail-scroll {
  flex: 1;
  min-height: 0;
}

.detail-body {
  padding: 16px 24px 24px;
  max-width: 720px;
}
</style>

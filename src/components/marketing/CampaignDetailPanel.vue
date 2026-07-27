<script setup lang="ts">
import { inject, ref, watch, computed } from 'vue';
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

type DetailTab = 'overview' | 'creatives' | 'metrics' | 'spend' | 'landing' | 'audience';
const activeTab = ref<DetailTab>('overview');
const loading = ref(false);
const error = ref('');

const detail = computed(() => campaignsCtx.campaignDetail.value);
const campaign = computed(() => detail.value?.campaign);
const storyFolder = computed(() => storiesCtx.activeFolder.value);

const tabs: { id: DetailTab; label: string }[] = [
  { id: 'overview', label: 'Overview' },
  { id: 'creatives', label: 'Creatives' },
  { id: 'metrics', label: 'Metrics' },
  { id: 'spend', label: 'Spend' },
  { id: 'landing', label: 'Landing Page' },
  { id: 'audience', label: 'Audience' },
];

// Creative form
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

// Metrics form
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

// Spend form
const showSpendForm = ref(false);
const sPlatform = ref('');
const sAmount = ref(0);
const sDate = ref('');
const sNotes = ref('');

// Landing page form
const showLandingForm = ref(false);
const lpName = ref('');
const lpUrl = ref('');
const lpConversion = ref<number | null>(null);
const lpNotes = ref('');

// Audience form
const showAudienceForm = ref(false);
const editingAudienceId = ref<number | null>(null);
const auLabel = ref('');
const auDemographics = ref('');
const auInterests = ref('');
const auLookalike = ref('');
const auOutcome = ref('untested');
const auNotes = ref('');

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

function editCreative(c: { id: number; name: string; creative_type: string; version: string; platform_format: string; status: string; asset_path: string; body_text: string; notes: string }): void {
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

async function removeCreative(id: number): Promise<void> {
  if (!confirm('Delete this creative?')) return;
  await campaignsCtx.deleteCreative(id);
  await reload();
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

async function removeSnapshot(id: number): Promise<void> {
  if (!confirm('Delete this snapshot?')) return;
  await campaignsCtx.deletePerformanceSnapshot(id);
  await reload();
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

async function removeSpend(id: number): Promise<void> {
  if (!confirm('Delete this spend entry?')) return;
  await campaignsCtx.deleteSpendEntry(id);
  await reload();
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
  if (result.success) {
    await campaignsCtx.updateCampaign({
      id: props.campaignId,
      name: campaign.value!.name,
      platform: campaign.value!.platform,
      platform_account_id: campaign.value!.platform_account_id,
      objective: campaign.value!.objective,
      status: campaign.value!.status,
      budget: campaign.value!.budget,
      budget_period: campaign.value!.budget_period,
      start_date: campaign.value!.start_date,
      end_date: campaign.value!.end_date,
      target_audience: campaign.value!.target_audience,
      landing_page_id: result.id,
      notes: campaign.value!.notes,
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

async function removeAudience(id: number): Promise<void> {
  if (!confirm('Delete this audience note?')) return;
  await campaignsCtx.deleteAudienceNote(id);
  await reload();
}

async function onDeleteCampaign(): Promise<void> {
  if (!confirm('Delete this campaign and all its data?')) return;
  await campaignsCtx.deleteCampaign(props.campaignId);
  if (storyFolder.value) await campaignsCtx.loadCampaigns(storyFolder.value);
  emit('back');
}
</script>

<template>
  <div class="panel campaign-detail-panel">
    <div class="detail-header">
      <button class="btn-back" @click="emit('back')">← Back</button>
      <h2 class="panel-title">{{ campaign?.name || 'Campaign' }}</h2>
      <div class="header-actions">
        <button class="btn btn-sm btn-secondary" @click="emit('edit')">Edit</button>
        <button class="btn btn-sm btn-danger" @click="onDeleteCampaign">Delete</button>
      </div>
    </div>

    <div v-if="loading" class="loading">Loading…</div>
    <div v-else-if="error" class="form-error">{{ error }}</div>

    <template v-else-if="campaign">
      <div class="platform-tabs detail-tabs">
        <button
          v-for="tab in tabs"
          :key="tab.id"
          class="platform-tab"
          :class="{ active: activeTab === tab.id }"
          @click="activeTab = tab.id"
        >{{ tab.label }}</button>
      </div>

      <!-- Overview -->
      <div v-show="activeTab === 'overview'" class="tab-content">
        <table class="overview-table">
          <tbody>
            <tr><td>Platform</td><td>{{ campaign.platform }}</td></tr>
            <tr><td>Objective</td><td>{{ campaign.objective }}</td></tr>
            <tr><td>Status</td><td>{{ campaign.status }}</td></tr>
            <tr><td>Budget</td><td>{{ campaign.budget != null ? `$${campaign.budget} (${campaign.budget_period})` : '—' }}</td></tr>
            <tr><td>Dates</td><td>{{ campaign.start_date || '—' }} – {{ campaign.end_date || '—' }}</td></tr>
            <tr><td>Total Spend</td><td>${{ (campaign.total_spend || 0).toFixed(2) }}</td></tr>
            <tr v-if="campaign.target_audience"><td>Audience</td><td>{{ campaign.target_audience }}</td></tr>
            <tr v-if="detail?.landing_page"><td>Landing Page</td><td><a :href="detail.landing_page.url" target="_blank" rel="noopener">{{ detail.landing_page.name }}</a></td></tr>
            <tr v-if="campaign.notes"><td>Notes</td><td>{{ campaign.notes }}</td></tr>
          </tbody>
        </table>
      </div>

      <!-- Creatives -->
      <div v-show="activeTab === 'creatives'" class="tab-content">
        <div class="tab-toolbar">
          <button class="btn btn-sm" @click="showCreativeForm = true; editingCreativeId = null">+ Add Creative</button>
        </div>
        <div v-if="showCreativeForm" class="inline-form">
          <input v-model="crName" placeholder="Name" />
          <select v-model="crType">
            <option value="video">Video</option>
            <option value="thumbnail">Thumbnail</option>
            <option value="hook">Hook</option>
            <option value="caption">Caption</option>
            <option value="cta">CTA</option>
          </select>
          <input v-model="crVersion" placeholder="Version (v1)" />
          <input v-model="crFormat" placeholder="Platform format" />
          <select v-model="crStatus">
            <option value="draft">Draft</option>
            <option value="live">Live</option>
            <option value="paused">Paused</option>
            <option value="retired">Retired</option>
          </select>
          <input v-model="crAssetPath" placeholder="Asset file path" />
          <textarea v-model="crBodyText" placeholder="Copy / caption / CTA text" rows="2" />
          <div class="inline-form-actions">
            <button class="btn btn-sm" @click="saveCreative">Save</button>
            <button class="btn btn-sm btn-secondary" @click="resetCreativeForm">Cancel</button>
          </div>
        </div>
        <div v-if="!detail?.creatives.length" class="empty-hint">No creatives yet.</div>
        <div v-for="c in detail?.creatives" :key="c.id" class="list-card">
          <div class="list-card-header">
            <strong>{{ c.name }}</strong>
            <span class="badge">{{ c.status }}</span>
          </div>
          <div class="list-card-meta">{{ c.creative_type }} · {{ c.version }} · {{ c.platform_format }}</div>
          <p v-if="c.body_text" class="list-card-body">{{ c.body_text }}</p>
          <div class="list-card-actions">
            <button class="btn btn-sm btn-secondary" @click="editCreative(c)">Edit</button>
            <button class="btn btn-sm btn-danger" @click="removeCreative(c.id)">Delete</button>
          </div>
        </div>
      </div>

      <!-- Metrics -->
      <div v-show="activeTab === 'metrics'" class="tab-content">
        <div class="tab-toolbar">
          <button class="btn btn-sm" @click="showMetricsForm = true">+ Log Snapshot</button>
        </div>
        <div v-if="showMetricsForm" class="inline-form">
          <input v-model="mDate" type="date" />
          <select v-model="mCreativeId">
            <option :value="null">Campaign-level</option>
            <option v-for="c in detail?.creatives" :key="c.id" :value="c.id">{{ c.name }}</option>
          </select>
          <input v-model.number="mImpressions" type="number" placeholder="Impressions" />
          <input v-model.number="mClicks" type="number" placeholder="Clicks" />
          <input v-model.number="mConversions" type="number" placeholder="Conversions" />
          <input v-model.number="mCtr" type="number" step="0.01" placeholder="CTR %" />
          <input v-model.number="mCpc" type="number" step="0.01" placeholder="CPC" />
          <input v-model.number="mCpa" type="number" step="0.01" placeholder="CPA" />
          <input v-model.number="mSpend" type="number" step="0.01" placeholder="Spend" />
          <div class="inline-form-actions">
            <button class="btn btn-sm" @click="saveMetrics">Save</button>
            <button class="btn btn-sm btn-secondary" @click="showMetricsForm = false">Cancel</button>
          </div>
        </div>
        <table v-if="detail?.snapshots.length" class="data-table">
          <thead>
            <tr><th>Date</th><th>Impr.</th><th>Clicks</th><th>CTR</th><th>Conv.</th><th>CPA</th><th>Spend</th><th></th></tr>
          </thead>
          <tbody>
            <tr v-for="s in detail.snapshots" :key="s.id">
              <td>{{ s.snapshot_date }}</td>
              <td>{{ s.impressions }}</td>
              <td>{{ s.clicks }}</td>
              <td>{{ s.ctr }}%</td>
              <td>{{ s.conversions }}</td>
              <td>${{ s.cpa }}</td>
              <td>${{ s.spend }}</td>
              <td><button class="btn-icon" @click="removeSnapshot(s.id)">×</button></td>
            </tr>
          </tbody>
        </table>
        <div v-else class="empty-hint">No metrics logged yet. Add a weekly snapshot.</div>
      </div>

      <!-- Spend -->
      <div v-show="activeTab === 'spend'" class="tab-content">
        <div class="tab-toolbar">
          <button class="btn btn-sm" @click="showSpendForm = true">+ Log Spend</button>
        </div>
        <div v-if="showSpendForm" class="inline-form">
          <input v-model="sPlatform" :placeholder="campaign.platform" />
          <input v-model.number="sAmount" type="number" step="0.01" placeholder="Amount" />
          <input v-model="sDate" type="date" />
          <input v-model="sNotes" placeholder="Notes" />
          <div class="inline-form-actions">
            <button class="btn btn-sm" @click="saveSpend">Save</button>
            <button class="btn btn-sm btn-secondary" @click="showSpendForm = false">Cancel</button>
          </div>
        </div>
        <table v-if="detail?.spend_entries.length" class="data-table">
          <thead><tr><th>Date</th><th>Platform</th><th>Amount</th><th>Notes</th><th></th></tr></thead>
          <tbody>
            <tr v-for="e in detail.spend_entries" :key="e.id">
              <td>{{ e.spent_at }}</td>
              <td>{{ e.platform }}</td>
              <td>${{ e.amount.toFixed(2) }}</td>
              <td>{{ e.notes }}</td>
              <td><button class="btn-icon" @click="removeSpend(e.id)">×</button></td>
            </tr>
          </tbody>
        </table>
        <div v-else class="empty-hint">No spend logged yet.</div>
      </div>

      <!-- Landing Page -->
      <div v-show="activeTab === 'landing'" class="tab-content">
        <div v-if="detail?.landing_page" class="list-card">
          <strong>{{ detail.landing_page.name }}</strong>
          <p><a :href="detail.landing_page.url" target="_blank" rel="noopener">{{ detail.landing_page.url }}</a></p>
          <p v-if="detail.landing_page.conversion_rate != null">Conversion: {{ detail.landing_page.conversion_rate }}%</p>
          <p v-if="detail.landing_page.notes">{{ detail.landing_page.notes }}</p>
        </div>
        <div v-else>
          <p class="empty-hint">No landing page linked.</p>
          <button class="btn btn-sm" @click="showLandingForm = true">+ Create &amp; Link</button>
        </div>
        <div v-if="showLandingForm" class="inline-form">
          <input v-model="lpName" placeholder="Page name" />
          <input v-model="lpUrl" placeholder="https://…" />
          <input v-model.number="lpConversion" type="number" step="0.1" placeholder="Conversion %" />
          <textarea v-model="lpNotes" placeholder="Notes" rows="2" />
          <div class="inline-form-actions">
            <button class="btn btn-sm" @click="saveLanding">Save &amp; Link</button>
            <button class="btn btn-sm btn-secondary" @click="showLandingForm = false">Cancel</button>
          </div>
        </div>
      </div>

      <!-- Audience -->
      <div v-show="activeTab === 'audience'" class="tab-content">
        <div class="tab-toolbar">
          <button class="btn btn-sm" @click="showAudienceForm = true; editingAudienceId = null">+ Add Note</button>
        </div>
        <div v-if="showAudienceForm" class="inline-form">
          <input v-model="auLabel" placeholder="Label (e.g. Lookalike 1%)" />
          <textarea v-model="auDemographics" placeholder="Demographics" rows="2" />
          <textarea v-model="auInterests" placeholder="Interests" rows="2" />
          <textarea v-model="auLookalike" placeholder="Lookalike notes" rows="2" />
          <select v-model="auOutcome">
            <option value="untested">Untested</option>
            <option value="worked">Worked</option>
            <option value="poor">Poor</option>
          </select>
          <textarea v-model="auNotes" placeholder="Notes" rows="2" />
          <div class="inline-form-actions">
            <button class="btn btn-sm" @click="saveAudience">Save</button>
            <button class="btn btn-sm btn-secondary" @click="resetAudienceForm">Cancel</button>
          </div>
        </div>
        <div v-if="!detail?.audience_notes.length" class="empty-hint">No audience tests recorded.</div>
        <div v-for="n in detail?.audience_notes" :key="n.id" class="list-card">
          <div class="list-card-header">
            <strong>{{ n.label || 'Untitled' }}</strong>
            <span class="badge">{{ n.outcome }}</span>
          </div>
          <p v-if="n.demographics"><em>Demographics:</em> {{ n.demographics }}</p>
          <p v-if="n.interests"><em>Interests:</em> {{ n.interests }}</p>
          <p v-if="n.lookalike_notes"><em>Lookalikes:</em> {{ n.lookalike_notes }}</p>
          <div class="list-card-actions">
            <button class="btn btn-sm btn-danger" @click="removeAudience(n.id)">Delete</button>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.campaign-detail-panel {
  padding: clamp(14px, 2vw, 24px);
  overflow-y: auto;
}

.detail-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
}

.panel-title {
  flex: 1;
  font-size: 16px;
  font-weight: 700;
  margin: 0;
}

.header-actions {
  display: flex;
  gap: 8px;
}

.btn-back {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 13px;
  padding: 4px 0;
}

.btn-back:hover { color: var(--accent); }

.detail-tabs {
  margin-bottom: 16px;
  flex-wrap: wrap;
}

.tab-content {
  max-width: 720px;
}

.tab-toolbar {
  margin-bottom: 12px;
}

.overview-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.overview-table td {
  padding: 8px 10px;
  border-bottom: 1px solid var(--border);
}

.overview-table td:first-child {
  color: var(--text-muted);
  width: 140px;
}

.inline-form {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  margin-bottom: 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface2);
}

.inline-form input,
.inline-form select,
.inline-form textarea {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text);
  font-size: 13px;
  padding: 6px 8px;
}

.inline-form-actions {
  display: flex;
  gap: 8px;
}

.list-card {
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 10px 12px;
  margin-bottom: 8px;
}

.list-card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 4px;
}

.list-card-meta {
  font-size: 11px;
  color: var(--text-muted);
  margin-bottom: 4px;
}

.list-card-body {
  font-size: 12px;
  color: var(--text-muted);
  margin: 4px 0;
}

.list-card-actions {
  display: flex;
  gap: 6px;
  margin-top: 6px;
}

.badge {
  font-size: 10px;
  text-transform: uppercase;
  padding: 2px 6px;
  border-radius: 8px;
  background: var(--surface2);
  color: var(--text-muted);
}

.data-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}

.data-table th,
.data-table td {
  padding: 6px 8px;
  border-bottom: 1px solid var(--border);
  text-align: left;
}

.data-table th {
  color: var(--text-muted);
  font-weight: 600;
}

.btn-icon {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 16px;
}

.btn-icon:hover { color: #c0392b; }

.empty-hint {
  color: var(--text-muted);
  font-size: 13px;
  margin-bottom: 12px;
}

.loading { color: var(--text-muted); font-size: 13px; }
.form-error { color: var(--danger); font-size: 12px; }

.btn {
  background: var(--accent);
  border: none;
  border-radius: var(--radius);
  color: #fff;
  cursor: pointer;
  font-size: 13px;
  font-weight: 600;
  padding: 9px 18px;
}

.btn:hover { background: var(--accent-dim); }
.btn-sm { padding: 6px 12px; font-size: 12px; }
.btn-secondary {
  background: var(--surface2);
  border: 1px solid var(--border);
  color: var(--text-muted);
}
.btn-danger { background: #c0392b; color: #fff; }
</style>

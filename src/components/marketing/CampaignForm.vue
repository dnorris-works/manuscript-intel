<script setup lang="ts">
import { inject, ref, watch, computed } from 'vue';
import { storiesKey, campaignsKey } from '../../injectionKeys';

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

const PLATFORMS = ['meta', 'amazon', 'tiktok', 'google', 'bookbub', 'other'];
const OBJECTIVES = ['awareness', 'conversion', 'traffic', 'engagement'];
const STATUSES = ['draft', 'active', 'paused', 'archived'];

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
  <div class="panel campaign-form-panel">
    <h2 class="panel-title">{{ isEditing ? 'Edit Campaign' : 'New Campaign' }}</h2>

    <div class="form-grid">
      <label>Name</label>
      <input v-model="name" type="text" placeholder="e.g. Book 1 Launch — Meta" />

      <label>Platform</label>
      <select v-model="platform">
        <option v-for="p in PLATFORMS" :key="p" :value="p">{{ p }}</option>
      </select>

      <label>Objective</label>
      <select v-model="objective">
        <option v-for="o in OBJECTIVES" :key="o" :value="o">{{ o }}</option>
      </select>

      <label>Status</label>
      <select v-model="status">
        <option v-for="s in STATUSES" :key="s" :value="s">{{ s }}</option>
      </select>

      <label>Platform Account</label>
      <select v-model="platformAccountId">
        <option :value="null">(None)</option>
        <option v-for="a in campaignsCtx.platformAccounts.value" :key="a.id" :value="a.id">
          {{ a.platform }} — {{ a.account_id || 'no ID' }}
        </option>
      </select>

      <label>Budget</label>
      <div class="budget-row">
        <input v-model.number="budget" type="number" min="0" step="0.01" placeholder="0.00" />
        <select v-model="budgetPeriod">
          <option value="daily">Daily</option>
          <option value="lifetime">Lifetime</option>
        </select>
      </div>

      <label>Start Date</label>
      <input v-model="startDate" type="date" />

      <label>End Date</label>
      <input v-model="endDate" type="date" />

      <label>Landing Page</label>
      <select v-model="landingPageId">
        <option :value="null">(None)</option>
        <option v-for="lp in campaignsCtx.landingPages.value" :key="lp.id" :value="lp.id">
          {{ lp.name }} — {{ lp.url }}
        </option>
      </select>

      <label>Target Audience</label>
      <textarea v-model="targetAudience" rows="3" placeholder="Demographics, interests, lookalikes…" />

      <label>Notes</label>
      <textarea v-model="notes" rows="2" />
    </div>

    <div v-if="error" class="form-error">{{ error }}</div>

    <div class="form-actions">
      <button class="btn" :disabled="saving" @click="onSave">Save</button>
      <button class="btn btn-secondary" @click="emit('cancel')">Cancel</button>
    </div>
  </div>
</template>

<style scoped>
.campaign-form-panel {
  padding: 20px;
  max-width: 520px;
  overflow-y: auto;
}

.panel-title {
  font-size: 16px;
  font-weight: 700;
  margin-bottom: 16px;
}

.form-grid {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.form-grid label {
  font-size: 12px;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.form-grid input,
.form-grid select,
.form-grid textarea {
  background: var(--surface2);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text);
  font-size: 13px;
  padding: 8px 10px;
  width: 100%;
}

.budget-row {
  display: flex;
  gap: 8px;
}

.budget-row input { flex: 1; }
.budget-row select { width: auto; }

.form-error {
  color: var(--danger);
  font-size: 12px;
  margin-top: 10px;
}

.form-actions {
  display: flex;
  gap: 8px;
  margin-top: 16px;
}

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
.btn:disabled { opacity: 0.5; cursor: not-allowed; }
.btn-secondary {
  background: var(--surface2);
  border: 1px solid var(--border);
  color: var(--text-muted);
}
</style>

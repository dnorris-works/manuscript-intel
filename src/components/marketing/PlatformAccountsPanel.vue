<script setup lang="ts">
import { inject, ref, onMounted } from 'vue';
import { campaignsKey, showPanelKey } from '../../injectionKeys';
import type { AdPlatformAccount } from '../../types';

const campaignsCtx = inject(campaignsKey)!;
const showPanel = inject(showPanelKey)!;

const showForm = ref(false);
const editingId = ref<number | null>(null);
const platform = ref('meta');
const accountId = ref('');
const pixelId = ref('');
const trackingNotes = ref('');
const paymentNotes = ref('');
const error = ref('');

const PLATFORMS = ['meta', 'amazon', 'tiktok', 'google', 'bookbub', 'other'];

onMounted(() => {
  void campaignsCtx.loadPlatformAccounts();
});

function resetForm(): void {
  showForm.value = false;
  editingId.value = null;
  platform.value = 'meta';
  accountId.value = '';
  pixelId.value = '';
  trackingNotes.value = '';
  paymentNotes.value = '';
  error.value = '';
}

function editAccount(a: AdPlatformAccount): void {
  editingId.value = a.id;
  platform.value = a.platform;
  accountId.value = a.account_id;
  pixelId.value = a.pixel_id;
  trackingNotes.value = a.tracking_notes;
  paymentNotes.value = a.payment_notes;
  showForm.value = true;
}

async function onSave(): Promise<void> {
  error.value = '';
  if (editingId.value) {
    const result = await campaignsCtx.updatePlatformAccount({
      id: editingId.value,
      platform: platform.value,
      account_id: accountId.value,
      pixel_id: pixelId.value,
      tracking_notes: trackingNotes.value,
      payment_notes: paymentNotes.value,
    });
    if (!result.success) { error.value = result.error; return; }
  } else {
    const result = await campaignsCtx.createPlatformAccount({
      platform: platform.value,
      account_id: accountId.value,
      pixel_id: pixelId.value,
      tracking_notes: trackingNotes.value,
      payment_notes: paymentNotes.value,
    });
    if (!result.success) { error.value = result.error; return; }
  }
  resetForm();
}

async function onDelete(id: number): Promise<void> {
  if (!confirm('Delete this platform account reference?')) return;
  await campaignsCtx.deletePlatformAccount(id);
}

function onBack(): void {
  showPanel('campaigns');
}
</script>

<template>
  <div class="panel platform-accounts-panel">
    <div class="panel-header">
      <button class="btn-back" @click="onBack">← Campaigns</button>
      <h2 class="panel-title">Platform Accounts</h2>
      <button class="btn btn-sm" @click="showForm = true; editingId = null">+ Add Account</button>
    </div>

    <p class="panel-desc">
      Reference data for your ad accounts — IDs, pixels, and payment notes. API tokens belong in Settings.
    </p>

    <div v-if="showForm" class="account-form">
      <label>Platform</label>
      <select v-model="platform">
        <option v-for="p in PLATFORMS" :key="p" :value="p">{{ p }}</option>
      </select>
      <label>Account ID</label>
      <input v-model="accountId" type="text" placeholder="Ad account ID" />
      <label>Pixel / Tag ID</label>
      <input v-model="pixelId" type="text" placeholder="Pixel or tracking tag ID" />
      <label>Tracking Notes</label>
      <textarea v-model="trackingNotes" rows="2" placeholder="Install notes, tag locations…" />
      <label>Payment Notes</label>
      <textarea v-model="paymentNotes" rows="2" placeholder="Card reference, billing contact…" />
      <div v-if="error" class="form-error">{{ error }}</div>
      <div class="form-actions">
        <button class="btn btn-sm" @click="onSave">Save</button>
        <button class="btn btn-sm btn-secondary" @click="resetForm">Cancel</button>
      </div>
    </div>

    <div v-if="campaignsCtx.platformAccounts.value.length === 0 && !showForm" class="empty-state">
      No platform accounts yet.
    </div>

    <div v-for="a in campaignsCtx.platformAccounts.value" :key="a.id" class="account-card">
      <div class="account-card-header">
        <strong>{{ a.platform }}</strong>
        <span class="account-id">{{ a.account_id || 'no ID' }}</span>
      </div>
      <div v-if="a.pixel_id" class="account-meta">Pixel: {{ a.pixel_id }}</div>
      <div v-if="a.tracking_notes" class="account-meta">{{ a.tracking_notes }}</div>
      <div v-if="a.payment_notes" class="account-meta">Payment: {{ a.payment_notes }}</div>
      <div class="account-actions">
        <button class="btn btn-sm btn-secondary" @click="editAccount(a)">Edit</button>
        <button class="btn btn-sm btn-danger" @click="onDelete(a.id)">Delete</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.platform-accounts-panel {
  padding: clamp(14px, 2vw, 24px);
  max-width: 560px;
  overflow-y: auto;
}

.panel-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.panel-title {
  flex: 1;
  font-size: 16px;
  font-weight: 700;
  margin: 0;
}

.panel-desc {
  color: var(--text-muted);
  font-size: 13px;
  margin-bottom: 16px;
}

.btn-back {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 13px;
}

.btn-back:hover { color: var(--accent); }

.account-form {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  margin-bottom: 16px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface2);
}

.account-form label {
  font-size: 11px;
  color: var(--text-muted);
  text-transform: uppercase;
}

.account-form input,
.account-form select,
.account-form textarea {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text);
  font-size: 13px;
  padding: 6px 8px;
}

.account-card {
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 10px 12px;
  margin-bottom: 8px;
}

.account-card-header {
  display: flex;
  justify-content: space-between;
  text-transform: capitalize;
}

.account-id {
  font-size: 12px;
  color: var(--text-muted);
}

.account-meta {
  font-size: 12px;
  color: var(--text-muted);
  margin-top: 4px;
}

.account-actions {
  display: flex;
  gap: 6px;
  margin-top: 8px;
}

.empty-state {
  color: var(--text-muted);
  font-size: 13px;
  padding: 16px;
  text-align: center;
  border: 1px dashed var(--border);
  border-radius: var(--radius);
}

.form-error { color: var(--danger); font-size: 12px; }
.form-actions { display: flex; gap: 8px; margin-top: 8px; }

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

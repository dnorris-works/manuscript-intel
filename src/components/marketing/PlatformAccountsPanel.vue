<script setup lang="ts">
import { inject, ref, onMounted, computed } from 'vue';
import {
  NPageHeader, NScrollbar, NButton, NCard, NForm, NFormItem, NInput, NSelect,
  NSpace, NAlert, NEmpty, NTag, useDialog,
} from 'naive-ui';
import { campaignsKey, showPanelKey } from '../../injectionKeys';
import type { AdPlatformAccount } from '../../types';

const campaignsCtx = inject(campaignsKey)!;
const showPanel = inject(showPanelKey)!;
const dialog = useDialog();

const showForm = ref(false);
const editingId = ref<number | null>(null);
const platform = ref('meta');
const accountId = ref('');
const pixelId = ref('');
const trackingNotes = ref('');
const paymentNotes = ref('');
const error = ref('');

const PLATFORMS = ['meta', 'amazon', 'tiktok', 'google', 'bookbub', 'other'];
const platformOptions = PLATFORMS.map(p => ({ label: p, value: p }));

const formTitle = computed(() => (editingId.value ? 'Edit Account' : 'Add Account'));

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

function onDelete(id: number): void {
  dialog.warning({
    title: 'Delete account',
    content: 'Delete this platform account reference?',
    positiveText: 'Delete',
    negativeText: 'Cancel',
    onPositiveClick: () => campaignsCtx.deletePlatformAccount(id),
  });
}

function onBack(): void {
  showPanel('campaigns');
}
</script>

<template>
  <div class="panel-root">
    <header class="panel-header">
      <n-page-header title="Platform Accounts">
        <template #extra>
          <n-space>
            <n-button @click="onBack">Campaigns</n-button>
            <n-button type="primary" @click="showForm = true; editingId = null">
              Add Account
            </n-button>
          </n-space>
        </template>
      </n-page-header>
    </header>

    <n-scrollbar class="panel-scroll">
      <div class="panel-body">
        <n-text depth="3" style="display: block; margin-bottom: 16px;">
          Reference data for your ad accounts — IDs, pixels, and payment notes. API tokens belong in Settings.
        </n-text>

        <n-card v-if="showForm" :title="formTitle" size="small" style="margin-bottom: 16px;">
          <n-form label-placement="top">
            <n-form-item label="Platform">
              <n-select v-model:value="platform" :options="platformOptions" />
            </n-form-item>
            <n-form-item label="Account ID">
              <n-input v-model:value="accountId" placeholder="Ad account ID" />
            </n-form-item>
            <n-form-item label="Pixel / Tag ID">
              <n-input v-model:value="pixelId" placeholder="Pixel or tracking tag ID" />
            </n-form-item>
            <n-form-item label="Tracking Notes">
              <n-input v-model:value="trackingNotes" type="textarea" :rows="2" placeholder="Install notes, tag locations…" />
            </n-form-item>
            <n-form-item label="Payment Notes">
              <n-input v-model:value="paymentNotes" type="textarea" :rows="2" placeholder="Card reference, billing contact…" />
            </n-form-item>
          </n-form>
          <n-alert v-if="error" type="error" :show-icon="false" style="margin: 8px 0;">
            {{ error }}
          </n-alert>
          <n-space>
            <n-button type="primary" @click="onSave">Save</n-button>
            <n-button @click="resetForm">Cancel</n-button>
          </n-space>
        </n-card>

        <n-empty
          v-if="campaignsCtx.platformAccounts.value.length === 0 && !showForm"
          description="No platform accounts yet."
        />

        <n-space v-else vertical :size="8">
          <n-card v-for="a in campaignsCtx.platformAccounts.value" :key="a.id" size="small">
            <n-space justify="space-between" align="center">
              <n-tag style="text-transform: capitalize;">{{ a.platform }}</n-tag>
              <n-text depth="3">{{ a.account_id || 'no ID' }}</n-text>
            </n-space>
            <n-text v-if="a.pixel_id" depth="3" style="display: block; margin-top: 6px; font-size: 12px;">
              Pixel: {{ a.pixel_id }}
            </n-text>
            <n-text v-if="a.tracking_notes" depth="3" style="display: block; margin-top: 4px; font-size: 12px;">
              {{ a.tracking_notes }}
            </n-text>
            <n-text v-if="a.payment_notes" depth="3" style="display: block; margin-top: 4px; font-size: 12px;">
              Payment: {{ a.payment_notes }}
            </n-text>
            <n-space style="margin-top: 8px;">
              <n-button size="small" @click="editAccount(a)">Edit</n-button>
              <n-button size="small" type="error" ghost @click="onDelete(a.id)">Delete</n-button>
            </n-space>
          </n-card>
        </n-space>
      </div>
    </n-scrollbar>
  </div>
</template>

<style scoped>
.panel-root {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.panel-header {
  flex-shrink: 0;
  padding: 20px 24px 0;
  background: var(--bg);
  border-bottom: 1px solid var(--border);
}

.panel-scroll {
  flex: 1;
  min-height: 0;
}

.panel-body {
  padding: 16px 24px 24px;
  max-width: 560px;
}
</style>

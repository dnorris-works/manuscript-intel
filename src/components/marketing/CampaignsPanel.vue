<script setup lang="ts">
import { inject, computed } from 'vue';
import { storiesKey, campaignsKey } from '../../injectionKeys';
import type { AdCampaign } from '../../types';

const storiesCtx = inject(storiesKey)!;
const campaignsCtx = inject(campaignsKey)!;

const emit = defineEmits<{
  (e: 'open-campaign', id: number): void;
  (e: 'new-campaign'): void;
}>();

const activeStory = computed(() => storiesCtx.activeStory.value);
const campaigns = computed(() => campaignsCtx.campaigns.value);

function statusClass(status: string): string {
  return `status-badge status-${status}`;
}

function formatSpend(amount: number): string {
  return `$${amount.toFixed(2)}`;
}

function onOpen(c: AdCampaign): void {
  emit('open-campaign', c.id);
}
</script>

<template>
  <div class="panel campaigns-panel">
    <div class="panel-header">
      <h2 class="panel-title">Ad Campaigns</h2>
      <button
        class="btn btn-sm"
        :disabled="!activeStory"
        @click="emit('new-campaign')"
      >+ New Campaign</button>
    </div>

    <p v-if="!activeStory" class="panel-desc">
      Select a story in the sidebar to manage its ad campaigns.
    </p>

    <template v-else>
      <p class="panel-desc">
        Campaigns for <strong>{{ activeStory.name }}</strong>. Create a campaign, then add creatives, metrics, and spend.
      </p>

      <div v-if="campaigns.length === 0" class="empty-state">
        No campaigns yet. Click <strong>New Campaign</strong> to get started.
      </div>

      <div v-else class="campaign-cards">
        <div
          v-for="c in campaigns"
          :key="c.id"
          class="campaign-card"
          @click="onOpen(c)"
        >
          <div class="campaign-card-header">
            <span class="campaign-name">{{ c.name }}</span>
            <span :class="statusClass(c.status)">{{ c.status }}</span>
          </div>
          <div class="campaign-card-meta">
            <span v-if="c.platform">{{ c.platform }}</span>
            <span v-if="c.objective"> · {{ c.objective }}</span>
          </div>
          <div class="campaign-card-footer">
            <span v-if="c.total_spend">Spent: {{ formatSpend(c.total_spend) }}</span>
            <span v-if="c.start_date">{{ c.start_date }}<template v-if="c.end_date"> – {{ c.end_date }}</template></span>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.campaigns-panel {
  padding: clamp(14px, 2vw, 24px);
  overflow-y: auto;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.panel-title {
  font-size: 16px;
  font-weight: 700;
  margin: 0;
}

.panel-desc {
  color: var(--text-muted);
  font-size: 13px;
  margin-bottom: 16px;
}

.empty-state {
  padding: 24px;
  text-align: center;
  color: var(--text-muted);
  font-size: 13px;
  border: 1px dashed var(--border);
  border-radius: var(--radius);
}

.campaign-cards {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.campaign-card {
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 12px 14px;
  cursor: pointer;
  background: color-mix(in srgb, var(--surface2) 40%, transparent);
  transition: border-color 0.15s;
}

.campaign-card:hover {
  border-color: var(--accent);
}

.campaign-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 4px;
}

.campaign-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
}

.status-badge {
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  padding: 2px 8px;
  border-radius: 10px;
  letter-spacing: 0.04em;
}

.status-draft { background: var(--surface2); color: var(--text-muted); }
.status-active { background: color-mix(in srgb, var(--success) 20%, transparent); color: var(--success); }
.status-paused { background: color-mix(in srgb, #f39c12 20%, transparent); color: #f39c12; }
.status-archived { background: var(--surface2); color: var(--text-muted); opacity: 0.7; }

.campaign-card-meta {
  font-size: 12px;
  color: var(--text-muted);
  text-transform: capitalize;
}

.campaign-card-footer {
  display: flex;
  gap: 12px;
  margin-top: 6px;
  font-size: 11px;
  color: var(--text-muted);
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
.btn-sm { padding: 6px 12px; font-size: 12px; }
</style>

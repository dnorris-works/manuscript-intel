<script setup lang="ts">
import { inject, computed } from 'vue';
import {
  NPageHeader, NScrollbar, NButton, NCard, NTag, NEmpty, NText, NSpace,
} from 'naive-ui';
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

function statusType(status: string): 'default' | 'success' | 'warning' {
  if (status === 'active') return 'success';
  if (status === 'paused') return 'warning';
  return 'default';
}

function formatSpend(amount: number): string {
  return `$${amount.toFixed(2)}`;
}

function onOpen(c: AdCampaign): void {
  emit('open-campaign', c.id);
}
</script>

<template>
  <div class="panel-root">
    <header class="panel-header">
      <n-page-header title="Ad Campaigns">
        <template #extra>
          <n-button type="primary" :disabled="!activeStory" @click="emit('new-campaign')">
            New Campaign
          </n-button>
        </template>
      </n-page-header>
    </header>

    <n-scrollbar class="panel-scroll">
      <div class="panel-body">
        <n-empty
          v-if="!activeStory"
          description="Select a story in the sidebar to manage its ad campaigns."
        />

        <template v-else>
          <n-text depth="3" style="display: block; margin-bottom: 16px;">
            Campaigns for <strong>{{ activeStory.name }}</strong>. Create a campaign, then add creatives, metrics, and spend.
          </n-text>

          <n-empty
            v-if="campaigns.length === 0"
            description="No campaigns yet. Click New Campaign to get started."
          />

          <n-space v-else vertical :size="12">
            <n-card
              v-for="c in campaigns"
              :key="c.id"
              size="small"
              hoverable
              style="cursor: pointer;"
              @click="onOpen(c)"
            >
              <n-space justify="space-between" align="center">
                <n-text strong>{{ c.name }}</n-text>
                <n-tag :type="statusType(c.status)" size="small">{{ c.status }}</n-tag>
              </n-space>
              <n-text depth="3" style="display: block; margin-top: 4px; text-transform: capitalize;">
                <template v-if="c.platform">{{ c.platform }}</template>
                <template v-if="c.objective"> · {{ c.objective }}</template>
              </n-text>
              <n-space :size="12" style="margin-top: 6px;">
                <n-text v-if="c.total_spend" depth="3" style="font-size: 12px;">
                  Spent: {{ formatSpend(c.total_spend) }}
                </n-text>
                <n-text v-if="c.start_date" depth="3" style="font-size: 12px;">
                  {{ c.start_date }}<template v-if="c.end_date"> – {{ c.end_date }}</template>
                </n-text>
              </n-space>
            </n-card>
          </n-space>
        </template>
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
  max-width: 640px;
}
</style>

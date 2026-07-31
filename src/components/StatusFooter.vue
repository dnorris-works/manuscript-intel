<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue';
import { NText } from 'naive-ui';
import { useAiSpend } from '../composables/useAiSpend';

const { totals, refreshAiSpend } = useAiSpend();

function formatUsd(amount: number): string {
  if (amount === 0) return '$0.00';
  if (amount < 0.01) return '<$0.01';
  return `$${amount.toFixed(2)}`;
}

let refreshTimer: ReturnType<typeof setInterval> | undefined;

onMounted(() => {
  void refreshAiSpend();
  refreshTimer = setInterval(() => void refreshAiSpend(), 30_000);
});

onUnmounted(() => {
  if (refreshTimer) clearInterval(refreshTimer);
});

defineExpose({ refreshAiSpend });
</script>

<template>
  <footer id="status-footer">
    <n-text depth="3" class="status-label">AI spend</n-text>
    <n-text class="status-value">{{ formatUsd(totals.month_usd) }}</n-text>
    <n-text depth="3" class="status-sep">this month</n-text>
    <span class="status-divider" aria-hidden="true" />
    <n-text class="status-value">{{ formatUsd(totals.ytd_usd) }}</n-text>
    <n-text depth="3" class="status-sep">YTD</n-text>
  </footer>
</template>

<style scoped>
#status-footer {
  grid-area: footer;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 12px;
  height: 24px;
  border-top: 1px solid var(--border);
  background: var(--surface);
  font-size: 11px;
  user-select: none;
}

.status-label {
  margin-right: 2px;
}

.status-value {
  font-variant-numeric: tabular-nums;
  font-weight: 600;
  color: var(--text);
}

.status-sep {
  margin-right: 4px;
}

.status-divider {
  width: 1px;
  height: 12px;
  background: var(--border);
  margin: 0 6px;
}
</style>

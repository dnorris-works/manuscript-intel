<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, inject } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { settingsKey, showPanelKey } from '../injectionKeys';
import { DEFAULT_LOCAL_MODEL } from '../composables/useSettings';

const emit = defineEmits<{
  done: [];
}>();

const settingsCtx = inject(settingsKey)!;
const showPanel = inject(showPanelKey)!;

const progress = ref('');
const downloading = ref(false);
const error = ref('');
const statusLabel = computed(() => {
  const s = settingsCtx.localAiStatus.value;
  if (!s) return 'Checking...';
  if (!s.running) return 'Not running';
  if (!s.ready) return 'Starting...';
  if (s.default_model_installed) return 'Ready';
  return 'Model not installed';
});

let unlistenProgress: UnlistenFn | null = null;

onMounted(async () => {
  await settingsCtx.refreshLocalAiStatus();
  unlistenProgress = await listen<string>('local-ai:progress', (event) => {
    progress.value = event.payload || '';
    if (event.payload === 'done') {
      downloading.value = false;
      void settingsCtx.refreshLocalAiStatus();
    }
  });
});

onUnmounted(() => {
  void unlistenProgress?.();
});

async function onDownload(): Promise<void> {
  error.value = '';
  downloading.value = true;
  progress.value = 'Starting download...';
  const result = await settingsCtx.pullLocalModel(settingsCtx.localDefaultModel.value || DEFAULT_LOCAL_MODEL);
  downloading.value = false;
  if (result.success) {
    emit('done');
  } else {
    error.value = result.error;
  }
}

function onSkipToCloud(): void {
  settingsCtx.provider.value = 'tokenmix';
  emit('done');
  showPanel('settings');
}

function onDismiss(): void {
  emit('done');
}
</script>

<template>
  <div class="local-ai-overlay">
    <div class="local-ai-card">
      <h2>Set up Local AI</h2>
      <p>
        Loremetry includes on-device AI powered by Ollama. Download the default model
        (<strong>{{ settingsCtx.localDefaultModel.value || DEFAULT_LOCAL_MODEL }}</strong>, ~2.5 GB)
        to run analysis and writing features privately — no API key required.
      </p>
      <p class="status-line">Status: <strong>{{ statusLabel }}</strong></p>
      <p v-if="progress" class="progress-line">{{ progress }}</p>
      <p v-if="error" class="error-line">{{ error }}</p>
      <div class="actions">
        <button class="btn btn-primary" :disabled="downloading" @click="onDownload">
          {{ downloading ? 'Downloading...' : 'Download default model' }}
        </button>
        <button class="btn" :disabled="downloading" @click="onDismiss">Skip for now</button>
        <button class="btn btn-link" :disabled="downloading" @click="onSkipToCloud">
          Use cloud AI instead
        </button>
      </div>
      <p class="hint">
        Requires internet once. After download, local AI works offline. You can switch
        between Local and TokenMix anytime in Settings.
      </p>
    </div>
  </div>
</template>

<style scoped>
.local-ai-overlay {
  position: fixed;
  inset: 0;
  z-index: 1000;
  background: rgba(0, 0, 0, 0.65);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
}

.local-ai-card {
  max-width: 480px;
  background: var(--panel-bg, #1e1e1e);
  border: 1px solid var(--border, #333);
  border-radius: 8px;
  padding: 28px 32px;
}

.local-ai-card h2 {
  margin: 0 0 12px;
  font-size: 1.25rem;
}

.local-ai-card p {
  margin: 0 0 12px;
  line-height: 1.5;
  color: var(--text-muted, #aaa);
}

.status-line {
  color: var(--text, #eee);
}

.progress-line {
  font-family: monospace;
  font-size: 0.85rem;
}

.error-line {
  color: #e55;
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin: 20px 0 12px;
}

.hint {
  font-size: 0.85rem;
}

.btn-link {
  background: none;
  border: none;
  color: var(--accent, #6af);
  cursor: pointer;
  text-decoration: underline;
  padding: 6px 8px;
}
</style>

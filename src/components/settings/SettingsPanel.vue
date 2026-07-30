<script setup lang="ts">
import { inject, ref, computed } from 'vue';
import { NTabs, NTabPane, NButton, NText, NSpace } from 'naive-ui';
import { settingsKey, showPanelKey } from '../../injectionKeys';
import { SETTINGS_TABS, type SettingsTab } from './types';
import SettingsGeneralTab from './tabs/SettingsGeneralTab.vue';
import SettingsAiModelsTab from './tabs/SettingsAiModelsTab.vue';
import SettingsFoldersTab from './tabs/SettingsFoldersTab.vue';
import SettingsCanopyTab from './tabs/SettingsCanopyTab.vue';
import SettingsDataForSeoTab from './tabs/SettingsDataForSeoTab.vue';
import SettingsWinningCatTab from './tabs/SettingsWinningCatTab.vue';
import SettingsDatabaseTab from './tabs/SettingsDatabaseTab.vue';

const settingsCtx = inject(settingsKey)!;
const showPanel = inject(showPanelKey)!;

const activeTab = ref<SettingsTab>('general');
const savedMsg = ref('');

const showSaveFooter = computed(() => activeTab.value !== 'winningcat' && activeTab.value !== 'database');

function onSave(): void {
  settingsCtx.saveSettings().then(() => {
    savedMsg.value = '✓ Saved';
    setTimeout(() => { savedMsg.value = ''; }, 1500);
    showPanel('analyzer');
  }).catch((e) => {
    savedMsg.value = 'Save failed: ' + String(e);
  });
}
</script>

<template>
  <div class="settings-panel">
    <h2 class="panel-title">Settings</h2>

    <n-tabs v-model:value="activeTab" type="line" animated>
      <n-tab-pane
        v-for="tab in SETTINGS_TABS"
        :key="tab.id"
        :name="tab.id"
        :tab="tab.label"
      >
        <div class="settings-tab-panel">
          <SettingsGeneralTab v-if="tab.id === 'general'" />
          <SettingsAiModelsTab v-else-if="tab.id === 'ai'" />
          <SettingsFoldersTab v-else-if="tab.id === 'folders'" />
          <SettingsCanopyTab v-else-if="tab.id === 'canopy'" />
          <SettingsDataForSeoTab v-else-if="tab.id === 'dataforseo'" />
          <SettingsWinningCatTab v-else-if="tab.id === 'winningcat'" />
          <SettingsDatabaseTab
            v-else-if="tab.id === 'database'"
            :active="activeTab === 'database'"
          />
        </div>
      </n-tab-pane>
    </n-tabs>

    <n-space v-if="showSaveFooter" align="center" class="settings-footer">
      <n-button type="primary" @click="onSave">Save Settings</n-button>
      <n-text v-if="savedMsg" type="success" style="font-size: 12px;">{{ savedMsg }}</n-text>
    </n-space>
  </div>
</template>

<style scoped>
.settings-panel {
  padding: clamp(14px, 2vw, 24px);
  overflow-y: auto;
  width: 100%;
  max-width: none;
  height: 100%;
  display: flex;
  flex-direction: column;
}

.panel-title {
  font-size: 16px;
  font-weight: 700;
  margin: 0 0 16px;
}

.settings-tab-panel {
  padding-top: 16px;
  min-height: 0;
}

.settings-footer {
  margin-top: 20px;
  padding-top: 16px;
  border-top: 1px solid var(--border);
}
</style>

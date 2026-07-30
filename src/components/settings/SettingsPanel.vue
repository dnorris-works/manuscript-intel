<script setup lang="ts">
import { inject, ref, computed } from 'vue';
import {
  NScrollbar, NTabs, NTabPane, NButton, NText, NSpace, NDivider, NPageHeader,
} from 'naive-ui';
import { settingsKey, showPanelKey } from '../../injectionKeys';
import { SETTINGS_TABS, type SettingsTab } from './types';
import SettingsGeneralTab from './tabs/SettingsGeneralTab.vue';
import SettingsAiModelsTab from './tabs/SettingsAiModelsTab.vue';
import SettingsFoldersTab from './tabs/SettingsFoldersTab.vue';
import SettingsCanopyTab from './tabs/SettingsCanopyTab.vue';
import SettingsDataForSeoTab from './tabs/SettingsDataForSeoTab.vue';
import SettingsWinningCatTab from './tabs/SettingsWinningCatTab.vue';
import SettingsDatabaseTab from './tabs/SettingsDatabaseTab.vue';
import SettingsStoryDataTab from './tabs/SettingsStoryDataTab.vue';
import SettingsArchivedReportsTab from './tabs/SettingsArchivedReportsTab.vue';

const settingsCtx = inject(settingsKey)!;
const showPanel = inject(showPanelKey)!;

const activeTab = ref<SettingsTab>('general');
const savedMsg = ref('');

const showSaveFooter = computed(() =>
  activeTab.value !== 'winningcat'
  && activeTab.value !== 'database'
  && activeTab.value !== 'storydata'
  && activeTab.value !== 'archived',
);

function onSave(): void {
  settingsCtx.saveSettings().then(() => {
    savedMsg.value = 'Saved';
    setTimeout(() => { savedMsg.value = ''; }, 1500);
    showPanel('analyzer');
  }).catch((e) => {
    savedMsg.value = 'Save failed: ' + String(e);
  });
}
</script>

<template>
  <div class="settings-root">
    <header class="settings-header">
      <n-page-header title="Settings" style="padding: 20px 24px 0;" />
      <n-tabs
        v-model:value="activeTab"
        type="card"
        animated
        class="settings-tabs"
        style="padding: 0 24px;"
      >
        <n-tab-pane
          v-for="tab in SETTINGS_TABS"
          :key="tab.id"
          :name="tab.id"
          :tab="tab.label"
          :display-directive="'show'"
        />
      </n-tabs>
    </header>

    <n-scrollbar class="settings-scroll">
      <div class="settings-body">
        <SettingsGeneralTab v-if="activeTab === 'general'" />
        <SettingsAiModelsTab v-else-if="activeTab === 'ai'" />
        <SettingsFoldersTab v-else-if="activeTab === 'folders'" />
        <SettingsCanopyTab v-else-if="activeTab === 'canopy'" />
        <SettingsDataForSeoTab v-else-if="activeTab === 'dataforseo'" />
        <SettingsWinningCatTab v-else-if="activeTab === 'winningcat'" />
        <SettingsStoryDataTab
          v-else-if="activeTab === 'storydata'"
          :active="activeTab === 'storydata'"
        />
        <SettingsArchivedReportsTab
          v-else-if="activeTab === 'archived'"
          :active="activeTab === 'archived'"
        />
        <SettingsDatabaseTab
          v-else-if="activeTab === 'database'"
          :active="activeTab === 'database'"
        />
      </div>
    </n-scrollbar>

    <footer v-if="showSaveFooter" class="settings-footer">
      <n-divider style="margin: 0;" />
      <n-space align="center" style="padding: 12px 24px;">
        <n-button type="primary" @click="onSave">Save Settings</n-button>
        <n-text v-if="savedMsg" type="success">{{ savedMsg }}</n-text>
      </n-space>
    </footer>
  </div>
</template>

<style scoped>
.settings-root {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.settings-header {
  flex-shrink: 0;
  background: var(--bg);
  border-bottom: 1px solid var(--border);
  z-index: 1;
}

.settings-tabs :deep(.n-tabs-pane-wrapper) {
  display: none;
}

.settings-scroll {
  flex: 1;
  min-height: 0;
}

.settings-body {
  padding: 16px 24px 24px;
}

.settings-footer {
  flex-shrink: 0;
  background: var(--bg);
}
</style>

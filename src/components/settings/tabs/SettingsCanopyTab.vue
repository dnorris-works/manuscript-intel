<script setup lang="ts">
import { inject, ref } from 'vue';
import { NAlert, NForm, NFormItem, NInput, NButton, NSpace } from 'naive-ui';
import { settingsKey } from '../../../injectionKeys';

const settingsCtx = inject(settingsKey)!;
const canopyTestStatus = ref('');

async function onTestCanopy(): Promise<void> {
  canopyTestStatus.value = 'Testing...';
  const result = await settingsCtx.testCanopy();
  canopyTestStatus.value = result.success ? 'Connected' : result.error;
}
</script>

<template>
  <n-space vertical :size="16">
    <n-alert type="info" :bordered="false">
      Connect to the Canopy API for market intelligence reports.
    </n-alert>

    <n-form label-placement="top">
      <n-form-item label="Canopy API Key">
        <n-input
          v-model:value="settingsCtx.canopyApiKey.value"
          type="password"
          show-password-on="click"
          placeholder="Enter Canopy API key"
        />
      </n-form-item>

      <n-form-item :feedback="canopyTestStatus || undefined">
        <n-button @click="onTestCanopy">Test Connection</n-button>
      </n-form-item>
    </n-form>
  </n-space>
</template>

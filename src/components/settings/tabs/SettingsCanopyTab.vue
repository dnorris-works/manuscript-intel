<script setup lang="ts">
import { inject, ref } from 'vue';
import { NForm, NFormItem, NInput, NButton, NText, NSpace } from 'naive-ui';
import { settingsKey } from '../../../injectionKeys';

const settingsCtx = inject(settingsKey)!;
const canopyTestStatus = ref('');

async function onTestCanopy(): Promise<void> {
  canopyTestStatus.value = 'Testing...';
  const result = await settingsCtx.testCanopy();
  canopyTestStatus.value = result.success ? '✓ Connected' : '✗ ' + result.error;
}
</script>

<template>
  <n-form label-placement="top" :show-feedback="false">
    <n-text depth="3" style="display: block; margin-bottom: 12px;">
      Connect to the Canopy API for market intelligence reports.
    </n-text>

    <n-form-item label="Canopy API Key">
      <n-input
        v-model:value="settingsCtx.canopyApiKey.value"
        type="password"
        show-password-on="click"
        placeholder="Enter Canopy API key"
      />
    </n-form-item>

    <n-space align="center">
      <n-button size="small" @click="onTestCanopy">Test Connection</n-button>
      <n-text v-if="canopyTestStatus" depth="3" style="font-size: 12px;">
        {{ canopyTestStatus }}
      </n-text>
    </n-space>
  </n-form>
</template>

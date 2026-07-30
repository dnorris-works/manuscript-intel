<script setup lang="ts">
import { inject, ref } from 'vue';
import { NForm, NFormItem, NInput, NButton, NText, NSpace } from 'naive-ui';
import { settingsKey } from '../../../injectionKeys';

const settingsCtx = inject(settingsKey)!;
const dataforseoTestStatus = ref('');

async function onTestDataforseo(): Promise<void> {
  dataforseoTestStatus.value = 'Testing...';
  const result = await settingsCtx.testDataforseo();
  dataforseoTestStatus.value = result.success ? '✓ Connected' : '✗ ' + result.error;
}
</script>

<template>
  <n-form label-placement="top" :show-feedback="false">
    <n-text depth="3" style="display: block; margin-bottom: 12px;">
      Used for keyword search volume data (Amazon + Google). Get credentials at
      <strong>app.dataforseo.com</strong>.
    </n-text>

    <n-form-item label="Login (email)">
      <n-input
        v-model:value="settingsCtx.dataforseoLogin.value"
        placeholder="your@email.com"
      />
    </n-form-item>

    <n-form-item label="Password">
      <n-input
        v-model:value="settingsCtx.dataforseoPassword.value"
        type="password"
        show-password-on="click"
        placeholder="DataForSEO API password"
      />
    </n-form-item>

    <n-space align="center">
      <n-button size="small" @click="onTestDataforseo">Test Connection</n-button>
      <n-text v-if="dataforseoTestStatus" depth="3" style="font-size: 12px;">
        {{ dataforseoTestStatus }}
      </n-text>
    </n-space>
  </n-form>
</template>

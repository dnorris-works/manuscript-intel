<script setup lang="ts">
import { inject, ref } from 'vue';
import { NAlert, NForm, NFormItem, NInput, NButton, NSpace } from 'naive-ui';
import { settingsKey } from '../../../injectionKeys';

const settingsCtx = inject(settingsKey)!;
const dataforseoTestStatus = ref('');

async function onTestDataforseo(): Promise<void> {
  dataforseoTestStatus.value = 'Testing...';
  const result = await settingsCtx.testDataforseo();
  dataforseoTestStatus.value = result.success ? 'Connected' : result.error;
}
</script>

<template>
  <n-space vertical :size="16">
    <n-alert type="info" :bordered="false">
      Keyword search volume for Amazon + Google. Credentials at
      <strong>app.dataforseo.com</strong>.
    </n-alert>

    <n-form label-placement="top">
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

      <n-form-item :feedback="dataforseoTestStatus || undefined">
        <n-button @click="onTestDataforseo">Test Connection</n-button>
      </n-form-item>
    </n-form>
  </n-space>
</template>

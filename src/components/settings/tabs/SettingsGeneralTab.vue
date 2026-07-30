<script setup lang="ts">
import { inject } from 'vue';
import { NForm, NFormItem, NRadioGroup, NRadioButton } from 'naive-ui';
import { settingsKey } from '../../../injectionKeys';

const settingsCtx = inject(settingsKey)!;
</script>

<template>
  <n-form label-placement="top" :show-feedback="false">
    <n-form-item label="Theme">
      <n-radio-group
        :value="settingsCtx.theme.value"
        @update:value="settingsCtx.setTheme"
      >
        <n-radio-button value="dark">Dark</n-radio-button>
        <n-radio-button value="light">Light</n-radio-button>
      </n-radio-group>
    </n-form-item>
  </n-form>
</template>

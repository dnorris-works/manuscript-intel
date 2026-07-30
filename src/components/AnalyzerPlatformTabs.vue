<script setup lang="ts">
import { NRadioGroup, NRadioButton } from 'naive-ui';
import { usePlatform } from '../composables/usePlatform';

const platformCtx = usePlatform();
</script>

<template>
  <n-radio-group
    :value="platformCtx.platform.value"
    style="margin-bottom: 12px;"
    @update:value="platformCtx.setPlatform"
  >
    <n-radio-button value="kdp">KDP</n-radio-button>
    <n-radio-button value="wide">Wide</n-radio-button>
    <n-radio-button value="craft">Craft</n-radio-button>
    <n-radio-button value="publish">Publish</n-radio-button>
    <n-radio-button value="saved">Saved Reports</n-radio-button>
  </n-radio-group>
</template>

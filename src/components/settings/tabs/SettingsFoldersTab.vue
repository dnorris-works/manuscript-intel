<script setup lang="ts">
import { inject } from 'vue';
import {
  NAlert, NForm, NFormItem, NInput, NDynamicInput, NSpace,
} from 'naive-ui';
import { settingsKey } from '../../../injectionKeys';

const settingsCtx = inject(settingsKey)!;
const folderStructure = settingsCtx.folderStructure;
</script>

<template>
  <n-space vertical :size="16">
    <n-alert type="info" :bordered="false">
      Used when you choose <strong>Create empty story</strong>. Rename paths as needed — required folders cannot be removed.
    </n-alert>

    <n-form label-placement="top">
      <n-form-item label="Manuscript" feedback="Chapter files used for analysis.">
        <n-input v-model:value="folderStructure.manuscript" placeholder="Manuscript" />
      </n-form-item>

      <n-alert type="default" :bordered="false">
        Always created under Manuscript:
        <template v-for="(act, i) in (folderStructure.acts || [])" :key="act">
          <strong>{{ folderStructure.manuscript || 'Manuscript' }}/{{ act }}</strong><span v-if="i < (folderStructure.acts.length - 1)">, </span>
        </template>
      </n-alert>

      <n-form-item label="Bible" feedback="Story bible documents.">
        <n-input v-model:value="folderStructure.bible" placeholder="Bible" />
      </n-form-item>

      <n-form-item label="Characters" feedback="Character documents.">
        <n-input v-model:value="folderStructure.characters" placeholder="Characters" />
      </n-form-item>

      <n-form-item label="Locations" feedback="Location documents.">
        <n-input v-model:value="folderStructure.locations" placeholder="Locations" />
      </n-form-item>

      <n-form-item
        label="Additional folders"
        feedback="Created with new stories for your own use. Add or delete freely."
      >
        <n-dynamic-input
          v-model:value="folderStructure.extra"
          placeholder="Extra/Folder"
          :on-create="() => ''"
        />
      </n-form-item>
    </n-form>
  </n-space>
</template>

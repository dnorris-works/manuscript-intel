<script setup lang="ts">
import { inject } from 'vue';
import { NForm, NFormItem, NInput, NButton, NText, NSpace } from 'naive-ui';
import { settingsKey } from '../../../injectionKeys';

const settingsCtx = inject(settingsKey)!;
const folderStructure = settingsCtx.folderStructure;
</script>

<template>
  <n-form label-placement="top" :show-feedback="false">
    <n-text depth="3" style="display: block; margin-bottom: 12px;">
      Used when you choose <strong>Create empty story</strong>. The app uses these folders by purpose
      — you can rename the paths, but not remove them.
    </n-text>

    <n-form-item label="Manuscript — chapter files (analysis)">
      <n-input v-model:value="folderStructure.manuscript" placeholder="Manuscript" />
    </n-form-item>

    <n-text depth="3" style="font-size: 12px; margin: -4px 0 8px;">
      Always created under Manuscript:
      <template v-for="(act, i) in (folderStructure.acts || [])" :key="act">
        <strong>{{ folderStructure.manuscript || 'Manuscript' }}/{{ act }}</strong><span v-if="i < (folderStructure.acts.length - 1)">, </span>
      </template>
      — not optional.
    </n-text>

    <n-form-item label="Bible — story bible docs">
      <n-input v-model:value="folderStructure.bible" placeholder="Bible" />
    </n-form-item>

    <n-form-item label="Characters — character docs">
      <n-input v-model:value="folderStructure.characters" placeholder="Characters" />
    </n-form-item>

    <n-form-item label="Locations — location docs">
      <n-input v-model:value="folderStructure.locations" placeholder="Locations" />
    </n-form-item>

    <n-form-item label="Additional folders">
      <n-text depth="3" style="font-size: 12px; margin-bottom: 8px;">
        Created with new stories for your own use. The app does not read these specially — add or delete freely.
      </n-text>
      <n-space vertical :size="8" style="width: 100%;">
        <n-space
          v-for="(_path, index) in folderStructure.extra"
          :key="index"
          align="center"
          style="width: 100%;"
        >
          <n-input
            v-model:value="folderStructure.extra[index]"
            placeholder="Extra/Folder"
            style="flex: 1;"
          />
          <n-button size="small" type="error" @click="settingsCtx.removeFolderEntry(index)">
            Delete
          </n-button>
        </n-space>
        <n-button size="small" secondary @click="settingsCtx.addFolderEntry()">
          Add Folder
        </n-button>
      </n-space>
    </n-form-item>
  </n-form>
</template>

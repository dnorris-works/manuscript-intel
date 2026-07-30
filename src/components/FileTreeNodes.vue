<script setup lang="ts">
import { computed, h } from 'vue';
import { NTree, NButton, type TreeOption } from 'naive-ui';

export interface FileTreeEntry {
  name: string;
  path: string;
  is_dir: boolean;
  children: FileTreeEntry[];
}

const props = defineProps<{
  entries: FileTreeEntry[];
  expandedKeys: string[];
}>();

const emit = defineEmits<{
  (e: 'update:expandedKeys', keys: string[]): void;
  (e: 'open', entry: FileTreeEntry): void;
  (e: 'add', entry: FileTreeEntry): void;
}>();

const entryByPath = computed(() => {
  const map = new Map<string, FileTreeEntry>();
  function walk(entries: FileTreeEntry[]): void {
    for (const entry of entries) {
      map.set(entry.path, entry);
      if (entry.children.length) walk(entry.children);
    }
  }
  walk(props.entries);
  return map;
});

function toTreeOptions(entries: FileTreeEntry[]): TreeOption[] {
  return entries.map((entry) => ({
    key: entry.path,
    label: entry.is_dir ? entry.name : entry.name.replace(/\.md$/, ''),
    isLeaf: !entry.is_dir,
    children: entry.is_dir && entry.children.length
      ? toTreeOptions(entry.children)
      : undefined,
    suffix: entry.is_dir
      ? () => h(NButton, {
        size: 'tiny',
        quaternary: true,
        onClick: (ev: Event) => {
          ev.stopPropagation();
          emit('add', entry);
        },
      }, { default: () => '+' })
      : undefined,
  }));
}

const treeData = computed(() => toTreeOptions(props.entries));

function onUpdateExpandedKeys(keys: Array<string | number>): void {
  emit('update:expandedKeys', keys as string[]);
}

function onUpdateSelectedKeys(keys: Array<string | number>): void {
  const key = keys[0] as string | undefined;
  if (!key) return;
  const entry = entryByPath.value.get(key);
  if (entry && !entry.is_dir) emit('open', entry);
}
</script>

<template>
  <n-tree
    block-line
    selectable
    :data="treeData"
    :expanded-keys="expandedKeys"
    @update:expanded-keys="onUpdateExpandedKeys"
    @update:selected-keys="onUpdateSelectedKeys"
  />
</template>

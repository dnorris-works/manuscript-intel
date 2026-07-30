<script setup lang="ts">
import { NScrollbar, NPageHeader, NDivider } from 'naive-ui';

defineProps<{
  title: string;
}>();
</script>

<template>
  <div class="form-panel-root">
    <header class="form-panel-header">
      <n-page-header :title="title" style="padding: 20px 24px 12px;" />
    </header>

    <n-scrollbar class="form-panel-scroll">
      <div class="form-panel-body">
        <slot />
      </div>
    </n-scrollbar>

    <footer v-if="$slots.footer" class="form-panel-footer">
      <n-divider style="margin: 0;" />
      <div class="form-panel-footer-inner">
        <slot name="footer" />
      </div>
    </footer>
  </div>
</template>

<style scoped>
.form-panel-root {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.form-panel-header {
  flex-shrink: 0;
  background: var(--bg);
  border-bottom: 1px solid var(--border);
}

.form-panel-scroll {
  flex: 1;
  min-height: 0;
}

.form-panel-body {
  padding: 16px 24px 24px;
  max-width: 520px;
}

.form-panel-footer {
  flex-shrink: 0;
  background: var(--bg);
}

.form-panel-footer-inner {
  padding: 12px 24px;
}
</style>

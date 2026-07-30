<script setup lang="ts">
import { ref, nextTick, watch, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { NText, NSelect, NButton, NSpace, NInput, NAlert } from 'naive-ui';
import { useSettings } from '../composables/useSettings';
import { formatMarkdown } from '../formatMarkdown';

const props = defineProps<{
  chapterText: string;
  chapterTitle: string;
  storyFolder: string;
  selectedText: string;
  pinnedCount: number;
}>();

const emit = defineEmits<{
  (e: 'pin'): void;
  (e: 'clear-pins'): void;
}>();

const settings = useSettings();

const modelOptions = computed(() =>
  settings.models.value.map(m => ({ label: m.id, value: m.id })),
);

interface Message {
  role: 'user' | 'assistant';
  content: string;
}

const messages = ref<Message[]>([]);
const input = ref('');
const loading = ref(false);
const error = ref('');
const chatPane = ref<HTMLElement | null>(null);
const chatModel = ref(settings.modelFor('prose'));

async function onSend(): Promise<void> {
  const text = input.value.trim();
  if (!text || loading.value) return;

  if (!chatModel.value) {
    error.value = 'No model selected.';
    return;
  }

  error.value = '';
  messages.value.push({ role: 'user', content: text });
  input.value = '';
  loading.value = true;
  await scrollToBottom();

  // If there's selected text, include it as context in the actual message sent to AI
  const messageForAi = props.selectedText
    ? `[Selected text from manuscript: "${props.selectedText}"]\n\n${text}`
    : text;

  // Load bible from folder
  const bible = await loadBible();

  try {
    const result = await invoke<{ success: boolean; reply: string; error: string }>('chat_with_context', {
      request: {
        provider: settings.provider.value,
        api_key: settings.apiKey.value,
        model: chatModel.value,
        message: messageForAi,
        chapter_text: props.chapterText,
        chapter_title: props.chapterTitle,
        bible,
        history: messages.value.slice(0, -1), // exclude the message we just added
      }
    });

    if (result.success) {
      messages.value.push({ role: 'assistant', content: result.reply });
    } else {
      error.value = result.error;
    }
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
    await scrollToBottom();
  }
}

async function loadBible(): Promise<string> {
  // The backend will discover it from the folder, but we pass empty
  // and let the chat command handle it via the system prompt.
  // Actually, we can load it here for the frontend to pass.
  // For simplicity, pass empty — the backend system prompt includes bible.
  return '';
}

function onKeydown(e: KeyboardEvent): void {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    onSend();
  }
}

async function scrollToBottom(): Promise<void> {
  await nextTick();
  if (chatPane.value) {
    chatPane.value.scrollTop = chatPane.value.scrollHeight;
  }
}

function clearChat(): void {
  messages.value = [];
  error.value = '';
}

// Clear chat when chapter changes
watch(() => props.chapterTitle, () => {
  // Don't clear automatically — the user might want to keep context
});
</script>

<template>
  <div class="ai-chat">
    <div class="chat-header">
      <n-text depth="3" style="font-size: 12px; font-weight: 700; text-transform: uppercase;">AI Chat</n-text>
      <n-select v-model:value="chatModel" size="small" :options="modelOptions" style="flex: 1;" />
      <n-button v-if="messages.length > 0" size="tiny" quaternary @click="clearChat">Clear</n-button>
    </div>

    <div ref="chatPane" class="chat-messages">
      <n-text v-if="messages.length === 0" depth="3" style="font-size: 12px; font-style: italic; text-align: center; padding: 20px 0;">
        Ask anything about your chapter — rewrites, brainstorming, continuity, prose feedback.
      </n-text>
      <div
        v-for="(msg, i) in messages"
        :key="i"
        class="chat-msg"
        :class="msg.role"
      >
        <div v-if="msg.role === 'user'" class="chat-msg-text">{{ msg.content }}</div>
        <div v-else class="chat-msg-text" v-html="formatMarkdown(msg.content)"></div>
      </div>
      <div v-if="loading" class="chat-msg assistant">
        <div class="chat-msg-text chat-loading">Thinking...</div>
      </div>
    </div>

    <n-alert v-if="error" type="error" :show-icon="false" style="margin: 0 14px;">{{ error }}</n-alert>

    <div v-if="props.selectedText || props.pinnedCount > 0" class="chat-selection-bar">
      <div v-if="props.selectedText" class="chat-selection-indicator">
        <span class="selection-label">{{ props.pinnedCount > 0 ? `${props.pinnedCount} pinned` : 'Selected' }}:</span>
        {{ props.selectedText.length > 80 ? props.selectedText.substring(0, 80) + '...' : props.selectedText }}
      </div>
      <n-space size="small" style="padding: 2px 14px 6px;">
        <n-button size="tiny" @click="emit('pin')" title="Pin selection (⌘D)">Pin</n-button>
        <n-button v-if="props.pinnedCount > 0" size="tiny" quaternary @click="emit('clear-pins')">Clear pins</n-button>
      </n-space>
    </div>

    <div class="chat-input-row">
      <n-input
        v-model:value="input"
        type="textarea"
        :rows="2"
        placeholder="Ask about your chapter..."
        :disabled="loading"
        @keydown="onKeydown"
      />
      <n-button type="primary" :loading="loading" :disabled="!input.trim()" @click="onSend">Send</n-button>
    </div>
  </div>
</template>

<style scoped>
.ai-chat {
  display: flex;
  flex-direction: column;
  height: 100%;
  border-left: 1px solid var(--border);
}

.chat-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 14px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.chat-title {
  font-size: 12px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted);
}

.chat-model-select {
  flex: 1;
  background: var(--surface2);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text);
  font-size: 11px;
  padding: 3px 6px;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}

.chat-clear {
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: 11px;
  cursor: pointer;
  padding: 2px 6px;
  border-radius: var(--radius);
}

.chat-clear:hover {
  color: var(--text);
  background: var(--surface2);
}

.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 12px 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.chat-empty {
  color: var(--text-muted);
  font-size: 12px;
  font-style: italic;
  padding: 20px 0;
  text-align: center;
}

.chat-msg {
  max-width: 90%;
}

.chat-msg.user {
  align-self: flex-end;
}

.chat-msg.assistant {
  align-self: flex-start;
}

.chat-msg.user .chat-msg-text {
  background: var(--accent);
  color: #fff;
  border-radius: 12px 12px 2px 12px;
  padding: 8px 12px;
  font-size: 13px;
  line-height: 1.5;
  white-space: pre-wrap;
}

.chat-msg.assistant .chat-msg-text {
  background: var(--surface2);
  color: var(--text);
  border-radius: 12px 12px 12px 2px;
  padding: 8px 12px;
  font-size: 13px;
  line-height: 1.6;
}

.chat-msg.assistant .chat-msg-text :deep(p) {
  margin: 0 0 0.5em;
}

.chat-msg.assistant .chat-msg-text :deep(p:last-child) {
  margin-bottom: 0;
}

.chat-msg.assistant .chat-msg-text :deep(pre) {
  background: var(--surface);
  padding: 8px;
  border-radius: var(--radius);
  font-size: 12px;
  overflow-x: auto;
  margin: 6px 0;
}

.chat-msg.assistant .chat-msg-text :deep(code) {
  background: var(--surface);
  padding: 1px 4px;
  border-radius: 3px;
  font-size: 12px;
}

.chat-loading {
  font-style: italic;
  color: var(--text-muted);
}

.chat-error {
  color: #e74c3c;
  font-size: 11px;
  padding: 4px 14px;
  flex-shrink: 0;
}

.chat-selection-bar {
  border-top: 1px solid var(--border);
  flex-shrink: 0;
  background: rgba(232, 97, 44, 0.04);
}

.chat-selection-indicator {
  font-size: 11px;
  color: var(--accent);
  padding: 4px 14px 2px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.selection-label {
  font-weight: 600;
}

.chat-pin-actions {
  display: flex;
  gap: 6px;
  padding: 2px 14px 6px;
}

.pin-btn {
  background: none;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text-muted);
  font-size: 10px;
  padding: 2px 8px;
  cursor: pointer;
}

.pin-btn:hover {
  border-color: var(--accent);
  color: var(--accent);
}

.pin-clear {
  color: var(--text-muted);
}

.pin-clear:hover {
  border-color: #e74c3c;
  color: #e74c3c;
}

.chat-input-row {
  display: flex;
  gap: 8px;
  padding: 10px 14px;
  border-top: 1px solid var(--border);
  flex-shrink: 0;
  align-items: flex-end;
}

.chat-input {
  flex: 1;
  background: var(--surface2);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text);
  font-size: 13px;
  padding: 8px 10px;
  resize: none;
  font-family: inherit;
  line-height: 1.4;
}

.chat-input:focus {
  outline: none;
  border-color: var(--accent);
}

.chat-send {
  background: var(--accent);
  border: none;
  border-radius: var(--radius);
  color: #fff;
  font-size: 12px;
  font-weight: 600;
  padding: 8px 14px;
  cursor: pointer;
  white-space: nowrap;
}

.chat-send:hover { background: var(--accent-dim); }
.chat-send:disabled { background: var(--surface2); color: var(--text-muted); cursor: not-allowed; }
</style>

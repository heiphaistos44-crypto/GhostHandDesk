<template>
  <div class="chat-panel" :class="{ open: isOpen }">
    <div class="chat-header" @click="$emit('toggle')">
      <span class="chat-title">Chat</span>
      <span v-if="unreadCount > 0" class="unread-badge">{{ unreadCount }}</span>
      <span class="chat-toggle">{{ isOpen ? '✕' : '💬' }}</span>
    </div>

    <div v-if="isOpen" class="chat-body">
      <div class="chat-messages" ref="messagesRef">
        <div v-if="messages.length === 0" class="chat-empty">
          Aucun message pour cette session
        </div>
        <div
          v-for="(msg, i) in messages"
          :key="i"
          class="chat-message"
          :class="{ 'own': msg.from === deviceId }"
        >
          <div class="msg-meta">
            <span class="msg-from">{{ msg.from === deviceId ? 'Moi' : msg.from }}</span>
            <span class="msg-time">{{ formatTime(msg.timestamp) }}</span>
          </div>
          <div class="msg-text">{{ msg.text }}</div>
        </div>
      </div>

      <form class="chat-input-form" @submit.prevent="sendMessage">
        <input
          v-model="inputText"
          type="text"
          placeholder="Tapez un message..."
          class="chat-input"
          :disabled="!connected"
        />
        <button type="submit" class="chat-send-btn" :disabled="!inputText.trim() || !connected">
          ➤
        </button>
      </form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, nextTick, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';

interface ChatMsg {
  from: string;
  text: string;
  timestamp: number;
}

interface Props {
  isOpen: boolean;
  deviceId: string;
  connected: boolean;
}

const props = defineProps<Props>();
defineEmits<{ toggle: [] }>();

const messages = ref<ChatMsg[]>([]);
const inputText = ref('');
const unreadCount = ref(0);
const messagesRef = ref<HTMLDivElement | null>(null);

// Recevoir les messages de chat via l'API d'événements typés Tauri
function handleChatMessage(detail: ChatMsg) {
  messages.value.push({
    from: detail.from,
    text: detail.text,
    timestamp: detail.timestamp,
  });
  if (!props.isOpen) {
    unreadCount.value++;
  }
  scrollToBottom();
}

// Exposer pour que le parent puisse appeler
defineExpose({ handleChatMessage });

watch(() => props.isOpen, (open) => {
  if (open) {
    unreadCount.value = 0;
    scrollToBottom();
  }
});

async function sendMessage() {
  const text = inputText.value.trim();
  if (!text) return;

  try {
    await invoke('send_chat_message', { text });
    messages.value.push({
      from: props.deviceId,
      text,
      timestamp: Date.now(),
    });
    inputText.value = '';
    scrollToBottom();
  } catch (e) {
    console.error('Erreur envoi chat:', e);
  }
}

function scrollToBottom() {
  nextTick(() => {
    if (messagesRef.value) {
      messagesRef.value.scrollTop = messagesRef.value.scrollHeight;
    }
  });
}

function formatTime(ts: number): string {
  const d = new Date(ts);
  return d.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' });
}
</script>

<style scoped>
.chat-panel {
  position: absolute;
  right: 0;
  top: 50px;
  bottom: 0;
  width: 0;
  background: #1e1e1e;
  border-left: 1px solid #3e3e42;
  transition: width 0.3s ease;
  overflow: hidden;
  z-index: 20;
  display: flex;
  flex-direction: column;
}

.chat-panel.open {
  width: 320px;
}

.chat-header {
  display: flex;
  align-items: center;
  padding: 10px 14px;
  background: #2d2d30;
  border-bottom: 1px solid #3e3e42;
  cursor: pointer;
  min-height: 40px;
}

.chat-title {
  flex: 1;
  font-size: 13px;
  font-weight: 600;
  color: #ccc;
}

.unread-badge {
  background: #e44;
  color: #fff;
  font-size: 11px;
  padding: 2px 6px;
  border-radius: 10px;
  margin-right: 8px;
}

.chat-toggle {
  font-size: 14px;
  color: #888;
}

.chat-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.chat-empty {
  text-align: center;
  color: #666;
  font-size: 13px;
  padding: 30px 10px;
}

.chat-message {
  max-width: 85%;
  padding: 8px 12px;
  background: #2d2d30;
  border-radius: 8px;
  border: 1px solid #3e3e42;
  align-self: flex-start;
}

.chat-message.own {
  align-self: flex-end;
  background: #0e4a73;
  border-color: #0e639c;
}

.msg-meta {
  display: flex;
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 4px;
}

.msg-from {
  font-size: 11px;
  color: #4ec9b0;
  font-weight: 600;
}

.msg-time {
  font-size: 10px;
  color: #666;
}

.msg-text {
  font-size: 13px;
  color: #ddd;
  word-break: break-word;
  white-space: pre-wrap;
}

.chat-input-form {
  display: flex;
  padding: 10px;
  gap: 8px;
  border-top: 1px solid #3e3e42;
  background: #252526;
}

.chat-input {
  flex: 1;
  padding: 8px 12px;
  background: #3c3c3c;
  border: 1px solid #555;
  border-radius: 6px;
  color: #fff;
  font-size: 13px;
  outline: none;
}

.chat-input:focus {
  border-color: #0e639c;
}

.chat-send-btn {
  padding: 8px 14px;
  background: #0e639c;
  border: none;
  border-radius: 6px;
  color: #fff;
  font-size: 16px;
  cursor: pointer;
}

.chat-send-btn:hover:not(:disabled) {
  background: #1177bb;
}

.chat-send-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
</style>

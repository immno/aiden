<template>
  <div class="chat-view">
    <div class="message-list">
      <div v-for="(message, index) in messages" :key="index" :class="['message', message.role]">
        <div v-html="renderMarkdown(message.content)"></div>
      </div>
    </div>
    <div class="input-area">
      <input v-model="inputMessage" @keyup.enter="sendMessage" placeholder="Type your message..." />
      <button @click="sendMessage" :disabled="isSending || !inputMessage.trim()">Send</button>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import MarkdownIt from 'markdown-it';

export default defineComponent({
  name: 'ChatView',
  setup() {
    const messages = ref<Array<{ role: string; content: string }>>([]);
    const inputMessage = ref('');
    const isSending = ref(false);
    const md = new MarkdownIt();

    const renderMarkdown = (content: string) => {
      return md.render(content);
    };

    const sendMessage = async () => {
      if (inputMessage.value.trim() === '' || isSending.value) return;

      isSending.value = true;
      const userMessage = { role: 'user', content: inputMessage.value };
      messages.value.push(userMessage);
      inputMessage.value = ''; // 立即清空输入框

      try {
        const response = await invoke<string>('rag_query', { query: userMessage.content });
        messages.value.push({ role: 'assistant', content: response });
      } catch (error) {
        console.error('Error sending message:', error);
      } finally {
        isSending.value = false;
      }
    };

    return { messages, inputMessage, isSending, sendMessage, renderMarkdown };
  },
});
</script>

<style scoped>
/* 原有样式保持不变 */
.input-area button:disabled {
  background-color: #cccccc;
  cursor: not-allowed;
}

.chat-view {
  height: 100%; /* 占满父容器高度 */
  display: flex;
  flex-direction: column;
}

.message-list {
  flex: 1;
  overflow-y: auto; /* 允许消息列表滚动 */
  padding: 10px;
}

.message {
  margin-bottom: 10px;
  padding: 8px;
  border-radius: 5px;
}

.message.user {
  background-color: #e1f5fe;
  align-self: flex-end;
}

.message.assistant {
  background-color: #f5f5f5;
  align-self: flex-start;
}

.input-area {
  display: flex;
  padding: 10px;
  background-color: #fff;
  border-top: 1px solid #ddd;
}

.input-area input {
  flex: 1;
  padding: 8px;
  border: 1px solid #ccc;
  border-radius: 5px;
}

.input-area button {
  margin-left: 10px;
  padding: 8px 16px;
  background-color: #007bff;
  color: white;
  border: none;
  border-radius: 5px;
  cursor: pointer;
}
</style>
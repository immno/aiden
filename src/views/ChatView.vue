<template>
  <div class="chat-view">
    <h1>Chat View</h1>
    <div class="message-list">
      <div v-for="(message, index) in messages" :key="index" :class="['message', message.role]">
        {{ message.content }}
      </div>
    </div>
    <div class="input-area">
      <input v-model="inputMessage" @keyup.enter="sendMessage" placeholder="Type your message..." />
      <button @click="sendMessage">Send</button>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export default defineComponent({
  name: 'ChatView',
  setup() {
    const messages = ref<Array<{ role: string; content: string }>>([]);
    const inputMessage = ref('');

    const sendMessage = async () => {
      if (inputMessage.value.trim() === '') return;

      const userMessage = { role: 'user', content: inputMessage.value };
      messages.value.push(userMessage);

      try {
        const response = await invoke<string>('rag_query', { query: inputMessage.value });
        messages.value.push({ role: 'assistant', content: response });
      } catch (error) {
        console.error('Error sending message:', error);
      }

      inputMessage.value = '';
    };

    return { messages, inputMessage, sendMessage };
  },
});
</script>

<style scoped>
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
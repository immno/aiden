<template>
  <div class="config-view">
    <h1>Config View</h1>
    <div class="file-selector">
      <input type="file" multiple @change="handleFileSelect" />
      <button @click="scanFiles">Scan Selected Files</button>
    </div>
    <div class="group-management">
      <input v-model="newGroupName" placeholder="New group name" />
      <button @click="createGroup">Create Group</button>
      <div v-for="(group, index) in groups" :key="index" class="group">
        <h3>{{ group.name }}</h3>
        <ul>
          <li v-for="(file, fileIndex) in group.files" :key="fileIndex">{{ file }}</li>
        </ul>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export default defineComponent({
  name: 'ConfigView',
  setup() {
    const selectedFiles = ref<File[]>([]);
    const groups = ref<Array<{ name: string; files: string[] }>>([]);
    const newGroupName = ref('');

    const handleFileSelect = (event: Event) => {
      const input = event.target as HTMLInputElement;
      if (input.files) {
        selectedFiles.value = Array.from(input.files);
      }
    };

    const scanFiles = async () => {
      if (selectedFiles.value.length === 0) return;

      const filePaths = selectedFiles.value.map((file) => file.path);
      try {
        await invoke('rag_scan_files', { filePaths });
        alert('Files scanned successfully!');
      } catch (error) {
        console.error('Error scanning files:', error);
      }
    };

    const createGroup = () => {
      if (newGroupName.value.trim() === '') return;

      groups.value.push({ name: newGroupName.value, files: [] });
      newGroupName.value = '';
    };

    return { selectedFiles, groups, newGroupName, handleFileSelect, scanFiles, createGroup };
  },
});
</script>

<style scoped>
.config-view {
  height: 100%; /* 占满父容器高度 */
  padding: 20px;
  overflow-y: auto; /* 允许内容区域滚动 */
}

.file-selector {
  margin-bottom: 20px;
}

.group-management {
  margin-top: 20px;
}

.group {
  margin-bottom: 15px;
}

.group h3 {
  margin-bottom: 5px;
}

.group ul {
  list-style-type: none;
  padding-left: 0;
}

.group ul li {
  padding: 5px;
  background-color: #f9f9f9;
  border: 1px solid #ddd;
  margin-bottom: 5px;
}
</style>